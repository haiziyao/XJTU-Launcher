//! Per-plugin settings.
//!
//! Settings live with the rest of the app's data, under
//! `~/.xjtu-launcher/plugins/settings/<plugin-id>.json`. They are deliberately
//! separate from `config.json`: a font or pet preference should not be part of
//! the config snapshot the main page saves on every toggle, and a damaged
//! plugin file must not stop the app from loading.
//!
//! Only built-in plugins can own settings. Values are normalised on both read
//! and write, so a hand-edited file cannot put arbitrary CSS or an unknown pet
//! id into the interface.
use serde_json::{json, Map, Value};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::AppHandle;

use crate::plugins;

const MAX_BYTES: usize = 64 * 1024;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub const PET_ID: &str = plugins::DESKTOP_PET_ID;

pub const PET_CHARACTERS: [&str; 4] = [
    "deepseek-girl",
    "deepseek-whalechan",
    "gpt-dragon-girl",
    "claude",
];
pub const PET_BAR_STYLES: [&str; 3] = ["glass-light", "glass-dark", "solid"];
pub const PET_SOURCES: [&str; 4] = ["campus", "agent", "class", "update"];
pub const FONT_REGIONS: [&str; 5] = ["ui", "title", "body", "timetable", "code"];

fn default_pet() -> Value {
    json!({
        "character": "deepseek-girl",
        "barStyle": "glass-light",
        "barHidden": false,
        "bubble": true,
        "sources": ["campus", "agent", "update"],
        "position": Value::Null,
    })
}

fn default_font_region(family: &str, size: f64, weight: u32, line_height: f64) -> Value {
    json!({
        "family": family,
        "size": size,
        "weight": weight,
        "lineHeight": line_height,
        "letterSpacing": 0.0,
        "color": "",
    })
}

/// `#rrggbb`, or empty for "follow the app theme". Anything else falls back.
fn font_color(value: Option<&Value>, fallback: &Value) -> String {
    let raw = value
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if raw.is_empty() {
        return String::new();
    }
    let hex = raw.strip_prefix('#').unwrap_or("");
    if hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        raw
    } else {
        fallback.as_str().unwrap_or("").to_string()
    }
}

pub fn default_font() -> Value {
    json!({
        "ui": default_font_region("system", 13.0, 400, 1.5),
        "title": default_font_region("system", 20.0, 600, 1.35),
        "body": default_font_region("system", 12.5, 400, 1.6),
        "timetable": default_font_region("system", 13.0, 600, 1.35),
        "code": default_font_region("mono", 12.0, 400, 1.5),
    })
}

fn defaults(id: &str) -> Result<Value, String> {
    match id {
        plugins::DESKTOP_PET_ID => Ok(default_pet()),
        plugins::FONT_BEAUTIFY_ID => Ok(default_font()),
        _ if plugins::is_builtin(id) => Ok(json!({})),
        _ => Err("未知插件".into()),
    }
}

fn settings_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    if !plugins::is_builtin(id) {
        return Err("未知插件".into());
    }
    Ok(crate::storage::path(app, "plugins/settings")?.join(format!("{id}.json")))
}

fn valid_family(value: &str) -> bool {
    let len = value.chars().count();
    // Downloaded and local fonts may use any Unicode family name; keep the
    // same character set as `fonts::sanitize_family` so a name accepted when
    // adding a file is not silently reset when the page saves settings.
    (1..=64).contains(&len)
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
}

fn text(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .filter(|value| valid_family(value))
        .unwrap_or(fallback)
        .to_string()
}

fn number(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    value
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(min, max))
        .unwrap_or(fallback)
}

fn bool_value(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn string_list(value: Option<&Value>, allowed: &[&str], fallback: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    if let Some(list) = value.and_then(Value::as_array) {
        for entry in list {
            let Some(entry) = entry.as_str() else {
                continue;
            };
            if allowed.contains(&entry) && !result.iter().any(|existing| existing == entry) {
                result.push(entry.to_string());
            }
        }
    }
    if result.is_empty() && !fallback.is_empty() {
        return fallback.iter().map(|entry| (*entry).to_string()).collect();
    }
    result
}

fn position(value: Option<&Value>) -> Value {
    let Some(pair) = value.and_then(Value::as_array) else {
        return Value::Null;
    };
    if pair.len() != 2 {
        return Value::Null;
    }
    let Some(x) = pair[0].as_i64().and_then(|value| i32::try_from(value).ok()) else {
        return Value::Null;
    };
    let Some(y) = pair[1].as_i64().and_then(|value| i32::try_from(value).ok()) else {
        return Value::Null;
    };
    json!([x, y])
}

fn font_region(value: Option<&Value>, fallback: &Value) -> Value {
    let fields = value.and_then(Value::as_object);
    let reads = |key: &str| fields.and_then(|map| map.get(key));
    let default_family = fallback["family"].as_str().unwrap_or("system");
    let family = text(reads("family"), default_family);
    let size = number(
        reads("size"),
        fallback["size"].as_f64().unwrap_or(13.0),
        8.0,
        48.0,
    );
    let weight = number(
        reads("weight"),
        fallback["weight"].as_f64().unwrap_or(400.0),
        100.0,
        900.0,
    );
    // Snap to the nearest hundred so CSS gets a value a font can honour.
    let weight = ((weight / 100.0).round() * 100.0).clamp(100.0, 900.0) as u32;
    let line_height = number(
        reads("lineHeight"),
        fallback["lineHeight"].as_f64().unwrap_or(1.5),
        1.0,
        2.5,
    );
    let letter_spacing = number(reads("letterSpacing"), 0.0, -1.0, 5.0);
    let letter_spacing = (letter_spacing * 10.0).round() / 10.0;
    let color = font_color(reads("color"), &fallback["color"]);
    let round = |value: f64| (value * 10.0).round() / 10.0;
    json!({
        "family": family,
        "size": round(size),
        "weight": weight,
        "lineHeight": (line_height * 100.0).round() / 100.0,
        "letterSpacing": letter_spacing,
        "color": color,
    })
}

fn sanitize_font(value: Option<&Value>) -> Value {
    let defaults = default_font();
    let fields = value.and_then(Value::as_object);
    let mut result = Map::new();
    for region in FONT_REGIONS {
        let fallback = &defaults[region];
        let value = fields
            .and_then(|map| map.get(region))
            .map(|entry| font_region(Some(entry), fallback))
            .unwrap_or_else(|| fallback.clone());
        result.insert(region.to_string(), value);
    }
    Value::Object(result)
}

fn sanitize_pet(value: Option<&Value>) -> Value {
    let fields = value.and_then(Value::as_object);
    let reads = |key: &str| fields.and_then(|map| map.get(key));
    let character = text(reads("character"), "deepseek-girl");
    let character = if PET_CHARACTERS.contains(&character.as_str()) {
        character
    } else {
        "deepseek-girl".into()
    };
    let bar_style = text(reads("barStyle"), "glass-light");
    let bar_style = if PET_BAR_STYLES.contains(&bar_style.as_str()) {
        bar_style
    } else {
        "glass-light".into()
    };
    json!({
        "character": character,
        "barStyle": bar_style,
        "barHidden": bool_value(reads("barHidden"), false),
        "bubble": bool_value(reads("bubble"), true),
        "sources": string_list(reads("sources"), &PET_SOURCES, &["campus", "agent", "update"]),
        "position": position(reads("position")),
    })
}

/// Coerces a stored or submitted object into the shape the plugin expects.
/// Invalid values fall back one field at a time rather than failing the whole
/// page.
pub fn sanitize(id: &str, value: &Value) -> Result<Value, String> {
    match id {
        plugins::DESKTOP_PET_ID => Ok(sanitize_pet(Some(value))),
        plugins::FONT_BEAUTIFY_ID => Ok(sanitize_font(Some(value))),
        _ if plugins::is_builtin(id) => Ok(json!({})),
        _ => Err("未知插件".into()),
    }
}

pub fn load(app: &AppHandle, id: &str) -> Result<Value, String> {
    let path = settings_path(app, id)?;
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return defaults(id);
        }
        Err(_) => return Err("无法读取插件设置".into()),
    };
    if metadata.len() > MAX_BYTES as u64 {
        return Err("插件设置文件过大".into());
    }
    let bytes = fs::read(&path).map_err(|_| "无法读取插件设置")?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "插件设置文件格式错误".to_string())?;
    if !value.is_object() {
        return Err("插件设置格式错误".into());
    }
    sanitize(id, &value)
}

/// Internal callers (the pet window) should keep working with defaults even if
/// the settings file was damaged by hand.
pub fn load_or_default(app: &AppHandle, id: &str) -> Value {
    load(app, id).unwrap_or_else(|_| defaults(id).unwrap_or_else(|_| json!({})))
}

pub fn save(app: &AppHandle, id: &str, value: &Value) -> Result<Value, String> {
    let sanitized = sanitize(id, value)?;
    let bytes = serde_json::to_vec_pretty(&sanitized).map_err(|_| "无法序列化插件设置")?;
    if bytes.len() > MAX_BYTES {
        return Err("插件设置文件过大".into());
    }
    let _guard = WRITE_LOCK.lock().map_err(|_| "插件设置服务繁忙")?;
    crate::config::write_atomic(&settings_path(app, id)?, &bytes)?;
    Ok(sanitized)
}

/// Read-modify-write with the same lock as `save`, so the pet's drag position
/// and the settings page cannot overwrite each other.
pub fn update(app: &AppHandle, id: &str, edit: impl FnOnce(&mut Value)) -> Result<Value, String> {
    let _guard = WRITE_LOCK.lock().map_err(|_| "插件设置服务繁忙")?;
    let path = settings_path(app, id)?;
    let mut value = match fs::read(&path) {
        Ok(bytes) if bytes.len() <= MAX_BYTES => serde_json::from_slice::<Value>(&bytes)
            .ok()
            .filter(Value::is_object)
            .unwrap_or_else(|| defaults(id).unwrap_or_else(|_| json!({}))),
        _ => defaults(id).unwrap_or_else(|_| json!({})),
    };
    edit(&mut value);
    let sanitized = sanitize(id, &value)?;
    let bytes = serde_json::to_vec_pretty(&sanitized).map_err(|_| "无法序列化插件设置")?;
    crate::config::write_atomic(&path, &bytes)?;
    Ok(sanitized)
}

/// Moves the pet settings once, from the retired `config.json` fields into the
/// plugin store. An existing plugin file always wins.
pub fn migrate_pet(app: &AppHandle, config: &crate::config::AppConfig) -> Result<(), String> {
    let path = settings_path(app, plugins::DESKTOP_PET_ID)?;
    if path.exists() {
        return Ok(());
    }
    let mut value = default_pet();
    if let Some(fields) = value.as_object_mut() {
        fields.insert(
            "character".into(),
            Value::String(config.pet_character.clone()),
        );
        fields.insert(
            "barStyle".into(),
            Value::String(config.pet_bar_style.clone()),
        );
        fields.insert("barHidden".into(), Value::Bool(config.pet_bar_hidden));
        fields.insert("bubble".into(), Value::Bool(config.pet_bubble_enabled));
        if let Ok(list) = serde_json::to_value(&config.pet_sources) {
            fields.insert("sources".into(), list);
        }
        if let Some(position) = config.pet_position {
            fields.insert("position".into(), json!(position));
        }
    }
    let sanitized = sanitize_pet(Some(&value));
    let bytes = serde_json::to_vec_pretty(&sanitized).map_err(|_| "无法序列化插件设置")?;
    crate::config::write_atomic(&path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pet_defaults_are_valid_and_legacy_values_are_kept() {
        let value = sanitize_pet(Some(&json!({
            "character": "claude",
            "barStyle": "solid",
            "sources": ["campus", "campus", "bogus", "agent"],
            "position": [12, 34]
        })));
        assert_eq!(value["character"], "claude");
        assert_eq!(value["barStyle"], "solid");
        assert_eq!(value["sources"], json!(["campus", "agent"]));
        assert_eq!(value["position"], json!([12, 34]));
        assert_eq!(value["bubble"], true);

        let value = sanitize_pet(Some(&json!({
            "character": "not-a-pet",
            "sources": [],
            "position": [1]
        })));
        assert_eq!(value["character"], "deepseek-girl");
        assert_eq!(value["sources"], json!(["campus", "agent", "update"]));
        assert_eq!(value["position"], Value::Null);
    }

    #[test]
    fn font_values_are_clamped_and_snapped() {
        let value = sanitize_font(Some(&json!({
            "ui": {
                "family": "PingFang SC",
                "size": 200,
                "weight": 437,
                "lineHeight": 0.2,
                "letterSpacing": 99,
                "color": "#FF8800"
            },
            "code": { "family": "Comic Sans; background: red", "color": "red" }
        })));
        assert_eq!(value["ui"]["size"], 48.0);
        assert_eq!(value["ui"]["weight"], 400);
        assert_eq!(value["ui"]["lineHeight"], 1.0);
        assert_eq!(value["ui"]["letterSpacing"], 5.0);
        assert_eq!(value["ui"]["color"], "#ff8800");
        // An invalid family falls back instead of becoming CSS.
        assert_eq!(value["code"]["family"], "mono");
        // Colours are limited to hex literals; anything else means "default".
        assert_eq!(value["code"]["color"], "");
        // Untouched regions keep their defaults.
        assert_eq!(value["title"]["size"], 20.0);
    }

    #[test]
    fn unknown_plugins_have_no_settings() {
        assert!(sanitize("local.nope", &json!({})).is_err());
        assert!(sanitize(plugins::CAMPUS_ID, &json!({"x": 1})).is_ok());
    }
}
