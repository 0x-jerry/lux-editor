//! Runtime-generated tray icon: no bundled assets, works on every platform.

const SIZE: u32 = 32;
const RADIUS: f32 = 6.0;

/// RGBA tray icon: blue rounded square with a white "L". Used as a template
/// image on macOS so the system renders it monochrome in either menu bar theme.
pub(crate) fn tray_icon() -> Option<tray_icon::Icon> {
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            if !in_rounded_rect(px, py) {
                continue;
            }
            let white = in_l(px, py);
            let offset = ((y * SIZE + x) * 4) as usize;
            rgba[offset] = if white { 255 } else { 59 };
            rgba[offset + 1] = if white { 255 } else { 130 };
            rgba[offset + 2] = if white { 255 } else { 246 };
            rgba[offset + 3] = 255;
        }
    }
    tray_icon::Icon::from_rgba(rgba, SIZE, SIZE).ok()
}

fn in_rounded_rect(x: f32, y: f32) -> bool {
    let edge = SIZE as f32 - 1.0;
    let (cx, cy) = (x.clamp(RADIUS, edge - RADIUS), y.clamp(RADIUS, edge - RADIUS));
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= RADIUS * RADIUS
}

/// Bold "L": vertical stroke plus a foot.
fn in_l(x: f32, y: f32) -> bool {
    (9.5..=13.5).contains(&x) && (8.5..=23.5).contains(&y)
        || (9.5..=25.5).contains(&x) && (19.5..=23.5).contains(&y)
}