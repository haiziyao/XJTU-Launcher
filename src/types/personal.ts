//! personal domain types.

export type PersonalImageKind = 'avatar' | 'wallpaper';
/** Crop rectangle as fractions (0–1) of the source image. */
export interface ImageCrop {
  x: number;
  y: number;
  width: number;
  height: number;
}
