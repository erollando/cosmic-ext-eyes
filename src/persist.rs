use cosmic::iced::Vector;
use std::path::{Path, PathBuf};

fn sanitize_key(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return "default".to_string();
    }
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn output_key() -> String {
    let raw = std::env::var("COSMIC_PANEL_OUTPUT").unwrap_or_else(|_| "default".to_string());
    sanitize_key(&raw)
}

fn instance_key() -> String {
    if let Ok(raw) = std::env::var("COSMIC_EYES_OFFSET_KEY") {
        return sanitize_key(&raw);
    }

    let mut parts = Vec::new();
    for key in [
        "COSMIC_APPLET_INSTANCE",
        "COSMIC_APPLET_INSTANCE_ID",
        "COSMIC_APPLET_UUID",
        "COSMIC_PANEL_INSTANCE",
        "COSMIC_PANEL_NAME",
        "COSMIC_PANEL_ANCHOR",
        "COSMIC_PANEL_EDGE",
        "COSMIC_DOCK_INSTANCE",
        "COSMIC_DOCK_NAME",
        "COSMIC_DOCK_ANCHOR",
        "COSMIC_DOCK_EDGE",
    ] {
        if let Ok(raw) = std::env::var(key) {
            let raw = raw.trim();
            if !raw.is_empty() {
                parts.push(sanitize_key(raw));
            }
        }
    }

    if parts.is_empty() {
        "default".to_string()
    } else {
        parts.join("-")
    }
}

fn state_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME")?;
    Some(Path::new(&home).join(".local/state"))
}

fn offset_path() -> Option<PathBuf> {
    Some(
        state_dir()?
            .join("cosmic-ext-eyes")
            .join(format!("offset-{}-{}.txt", output_key(), instance_key())),
    )
}

fn legacy_offset_path() -> Option<PathBuf> {
    Some(
        state_dir()?
            .join("cosmic-ext-eyes")
            .join(format!("offset-{}.txt", output_key())),
    )
}

pub fn load_offset(current_scale: f32) -> Option<Vector> {
    let bytes = match std::fs::read_to_string(offset_path()?) {
        Ok(bytes) => bytes,
        Err(_) => std::fs::read_to_string(legacy_offset_path()?).ok()?,
    };

    let mut parts = bytes.split_whitespace();
    let saved_scale = parts.next()?.parse::<f32>().ok()?;
    let x = parts.next()?.parse::<f32>().ok()?;
    let y = parts.next()?.parse::<f32>().ok()?;

    if (saved_scale - current_scale).abs() > 0.01 {
        return None;
    }

    Some(Vector::new(x, y))
}

pub fn save_offset(current_scale: f32, offset: Vector) -> std::io::Result<()> {
    let Some(path) = offset_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, format!("{} {} {}\n", current_scale, offset.x, offset.y))
}
