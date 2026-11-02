//! Physical-pixel placement independent of Win32 and Tauri.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
pub fn place(
    target: Rect,
    dpi: u32,
    width: f64,
    height: f64,
    anchor: &str,
    offset_x: f64,
    offset_y: f64,
) -> Rect {
    let scale = f64::from(dpi.max(96)) / 96.0;
    let width = ((width * scale).round() as u32).clamp(1, target.width.max(1));
    let height = ((height * scale).round() as u32).clamp(1, target.height.max(1));
    let max_x = target.width.saturating_sub(width) as i32;
    let max_y = target.height.saturating_sub(height) as i32;
    let dx = ((offset_x * scale).round() as i32).clamp(0, max_x);
    let dy = ((offset_y * scale).round() as i32).clamp(0, max_y);
    Rect {
        x: target.x
            + if anchor.ends_with("right") {
                max_x - dx
            } else {
                dx
            },
        y: target.y
            + if anchor.starts_with("bottom") {
                max_y - dy
            } else {
                dy
            },
        width,
        height,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_monitor_coordinates_and_dpi_are_physical_not_logical() {
        let target = Rect {
            x: -2560,
            y: 120,
            width: 2560,
            height: 1440,
        };
        assert_eq!(
            place(target, 144, 520.0, 620.0, "top-right", 24.0, 24.0),
            Rect {
                x: -816,
                y: 156,
                width: 780,
                height: 930
            }
        );
    }
    #[test]
    fn small_client_area_clamps_window_and_offsets() {
        let target = Rect {
            x: 100,
            y: -50,
            width: 300,
            height: 200,
        };
        let overlay = place(target, 192, 520.0, 620.0, "bottom-left", 1000.0, 1000.0);
        assert_eq!(overlay, target);
    }
}
