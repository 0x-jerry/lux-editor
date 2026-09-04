use eframe::egui::Color32;

/// `#rgb`, `#rrggbb` or `#rrggbbaa`.
pub(super) fn parse_color(value: &str) -> Result<Color32, String> {
    let [r, g, b, a] = parse_rgba(value)?;
    Ok(Color32::from_rgba_unmultiplied(r, g, b, a))
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa`; raw (non-premultiplied) RGBA bytes.
pub(super) fn parse_rgba(value: &str) -> Result<[u8; 4], String> {
    let hex = value
        .strip_prefix('#')
        .ok_or_else(|| format!("'{value}' is missing a leading '#'"))?;
    let digit = |index: usize| -> Result<u8, String> {
        u8::from_str_radix(&hex[index..index + 1], 16)
            .map(|nibble| nibble * 0x11)
            .map_err(|e| e.to_string())
    };
    let byte = |index: usize| -> Result<u8, String> {
        u8::from_str_radix(&hex[index..index + 2], 16).map_err(|e| e.to_string())
    };
    let channel = |index: usize| -> Result<u8, String> {
        match hex.len() {
            3 => digit(index / 2),
            6 | 8 => byte(index),
            len => Err(format!("'{value}' has {len} hex digits, expected 3, 6 or 8")),
        }
    };
    Ok([
        channel(0)?,
        channel(2)?,
        channel(4)?,
        if hex.len() == 8 { channel(6)? } else { 255 },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_accepts_documented_shapes() {
        // Color32 stores premultiplied gamma-space values; parse_rgba keeps raw bytes.
        assert_eq!(parse_rgba("#fff").unwrap(), [255, 255, 255, 255]);
        assert_eq!(parse_rgba("#6aa1ff").unwrap(), [0x6a, 0xa1, 0xff, 255]);
        assert_eq!(parse_rgba("#6aa1ff40").unwrap(), [0x6a, 0xa1, 0xff, 0x40]);
        assert_eq!(
            parse_color("#6aa1ff40").unwrap(),
            Color32::from_rgba_unmultiplied(0x6a, 0xa1, 0xff, 0x40)
        );
    }

    #[test]
    fn parse_color_rejects_malformed_values() {
        for bad in ["white", "6aa1ff", "#ff", "#ffff", "#gg0000", "#ff0000ff00"] {
            assert!(parse_rgba(bad).is_err(), "{bad} must not parse");
        }
    }
}
