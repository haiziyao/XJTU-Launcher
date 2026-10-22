//! Personal avatar and wallpaper import: hand the source to the crop dialog, then crop the frame the user chose.
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, metadata::Orientation, DynamicImage,
    ImageDecoder, ImageFormat, ImageReader,
};
use serde::Deserialize;
use std::{io::Cursor, path::Path};

const MAX_VALUE: usize = 12 * 1024 * 1024;

/// Crop rectangle as fractions of the source image, so the preview scale never matters.
#[derive(Deserialize)]
pub struct Crop {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy)]
pub enum Kind {
    Avatar,
    Wallpaper,
}

impl Kind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "avatar" => Ok(Self::Avatar),
            "wallpaper" => Ok(Self::Wallpaper),
            _ => Err("图片类型无效".into()),
        }
    }
    fn max_megabytes(self) -> u64 {
        match self {
            Self::Avatar => 5,
            Self::Wallpaper => 10,
        }
    }
    fn max_side(self) -> u32 {
        match self {
            Self::Avatar => 4096,
            Self::Wallpaper => 8192,
        }
    }
}

fn read(path: &Path, kind: Kind) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|_| "无法读取图片")?;
    if !meta.is_file() || meta.len() > kind.max_megabytes() * 1024 * 1024 {
        return Err(format!("请选择不超过 {} MB 的图片", kind.max_megabytes()));
    }
    std::fs::read(path).map_err(|_| "无法读取图片".to_string())
}

fn reader(bytes: &[u8], kind: Kind) -> Result<ImageReader<Cursor<&[u8]>>, String> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "图片格式无效")?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
    ) {
        return Err("仅支持 PNG、JPEG 和 WebP".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(kind.max_side());
    limits.max_image_height = Some(kind.max_side());
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader)
}

/// Checks format and dimensions from the header only; the webview decodes the preview itself.
pub fn source(path: &Path, kind: Kind) -> Result<Vec<u8>, String> {
    let bytes = read(path, kind)?;
    let (width, height) = reader(&bytes, kind)?
        .into_dimensions()
        .map_err(|_| "图片损坏或尺寸过大")?;
    if width.max(height) > kind.max_side() {
        return Err("图片尺寸过大".into());
    }
    Ok(bytes)
}

fn decode(path: &Path, kind: Kind) -> Result<DynamicImage, String> {
    let bytes = read(path, kind)?;
    let mut decoder = reader(&bytes, kind)?
        .into_decoder()
        .map_err(|_| "图片损坏或尺寸过大")?;
    // The webview honours EXIF rotation in the crop dialog, so the crop must too.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| "图片损坏或尺寸过大")?;
    img.apply_orientation(orientation);
    Ok(img)
}

fn encode(img: &DynamicImage, kind: Kind) -> Result<String, String> {
    let mut output = Cursor::new(Vec::new());
    let mime = match kind {
        Kind::Avatar => {
            img.write_to(&mut output, ImageFormat::Png)
                .map_err(|_| "无法处理图片")?;
            "png"
        }
        // The whole config is rewritten on every settings change, so keep photos compact.
        Kind::Wallpaper => {
            img.to_rgb8()
                .write_with_encoder(JpegEncoder::new_with_quality(&mut output, 88))
                .map_err(|_| "无法处理图片")?;
            "jpeg"
        }
    };
    let value = format!(
        "data:image/{mime};base64,{}",
        STANDARD.encode(output.into_inner())
    );
    if value.len() > MAX_VALUE {
        return Err("图片处理后仍过大，请选择较小图片".into());
    }
    Ok(value)
}

/// Crops the full-resolution source, then sizes the result for storage.
pub fn crop(path: &Path, kind: Kind, crop: &Crop) -> Result<String, String> {
    let img = decode(path, kind)?;
    let (x, y, width, height) = pixel_rect(crop, img.width(), img.height())?;
    let cropped = img.crop_imm(x, y, width, height);
    let output = match kind {
        Kind::Avatar => cropped.resize_to_fill(256, 256, FilterType::Lanczos3),
        Kind::Wallpaper if width > 1920 || height > 1200 => cropped.thumbnail(1920, 1200),
        Kind::Wallpaper => cropped,
    };
    encode(&output, kind)
}

fn pixel_rect(crop: &Crop, width: u32, height: u32) -> Result<(u32, u32, u32, u32), String> {
    // Floating point from the dialog may overshoot the edge by a hair.
    const SLACK: f64 = 1e-3;
    if [crop.x, crop.y, crop.width, crop.height]
        .iter()
        .any(|v| !v.is_finite())
        || crop.x < -SLACK
        || crop.y < -SLACK
        || crop.width <= 0.0
        || crop.height <= 0.0
        || crop.x + crop.width > 1.0 + SLACK
        || crop.y + crop.height > 1.0 + SLACK
    {
        return Err("裁剪区域无效".into());
    }
    let axis = |start: f64, length: f64, size: u32| {
        let total = f64::from(size);
        let from = (start.max(0.0) * total).round().min(total - 1.0) as u32;
        let span = ((length * total).round() as u32).clamp(1, size - from);
        (from, span)
    };
    let (x, w) = axis(crop.x, crop.width, width);
    let (y, h) = axis(crop.y, crop.height, height);
    Ok((x, y, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded(value: &str, mime: &str) -> DynamicImage {
        let (head, body) = value.split_once(',').unwrap();
        assert_eq!(head, format!("data:image/{mime};base64"));
        image::load_from_memory(&STANDARD.decode(body).unwrap()).unwrap()
    }
    fn rect(x: f64, y: f64, width: f64, height: f64) -> Crop {
        Crop {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn crop_uses_source_pixels_and_sizes_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source");
        DynamicImage::new_rgb8(4000, 2000)
            .save_with_format(&path, ImageFormat::Png)
            .unwrap();
        assert_eq!(
            source(&path, Kind::Wallpaper).unwrap(),
            std::fs::read(&path).unwrap()
        );
        let wallpaper = decoded(
            &crop(&path, Kind::Wallpaper, &rect(0.25, 0.0, 0.5, 1.0)).unwrap(),
            "jpeg",
        );
        assert_eq!((wallpaper.width(), wallpaper.height()), (1200, 1200));
        let avatar = decoded(
            &crop(&path, Kind::Avatar, &rect(0.5, 0.0, 0.5, 1.0)).unwrap(),
            "png",
        );
        assert_eq!((avatar.width(), avatar.height()), (256, 256));
    }

    #[test]
    fn small_wallpaper_crops_are_not_upscaled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source");
        DynamicImage::new_rgb8(800, 600)
            .save_with_format(&path, ImageFormat::Png)
            .unwrap();
        let wallpaper = decoded(
            &crop(&path, Kind::Wallpaper, &rect(0.0, 0.0, 0.5, 0.5)).unwrap(),
            "jpeg",
        );
        assert_eq!((wallpaper.width(), wallpaper.height()), (400, 300));
    }

    #[test]
    fn rejects_invalid_crops_kinds_and_formats() {
        for bad in [
            rect(0.8, 0.0, 0.5, 1.0),
            rect(0.0, 0.0, 0.0, 1.0),
            rect(f64::NAN, 0.0, 0.5, 0.5),
            rect(-0.2, 0.0, 0.5, 0.5),
        ] {
            assert!(pixel_rect(&bad, 100, 100).is_err());
        }
        assert_eq!(
            pixel_rect(&rect(0.0, 0.0, 1.0004, 1.0), 100, 100).unwrap(),
            (0, 0, 100, 100)
        );
        assert!(Kind::parse("banner").is_err());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image");
        std::fs::write(&path, b"<svg onload='alert(1)'/>").unwrap();
        assert!(source(&path, Kind::Avatar).is_err());
        DynamicImage::new_rgb8(4100, 10)
            .save_with_format(&path, ImageFormat::Png)
            .unwrap();
        assert!(source(&path, Kind::Avatar).is_err());
    }
}
