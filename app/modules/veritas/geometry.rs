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
/// A widget: `position` is its top-left as a fraction of the client area,
/// `size` its content in CSS pixels. Kept fully inside the client area.
pub fn widget(target: Rect, dpi: u32, scale: f64, position: [f64; 2], size: (f64, f64)) -> Rect {
    let ratio = f64::from(dpi.max(96)) / 96.0 * scale;
    let width = ((size.0 * ratio).ceil() as u32).clamp(1, target.width.max(1));
    let height = ((size.1 * ratio).ceil() as u32).clamp(1, target.height.max(1));
    let max_x = target.width.saturating_sub(width) as i32;
    let max_y = target.height.saturating_sub(height) as i32;
    Rect {
        x: target.x + ((position[0] * f64::from(target.width)).round() as i32).clamp(0, max_x),
        y: target.y + ((position[1] * f64::from(target.height)).round() as i32).clamp(0, max_y),
        width,
        height,
    }
}
/// Inverse of `widget`: where a dragged widget's corner is, as fractions.
pub fn fraction(target: Rect, x: i32, y: i32) -> [f64; 2] {
    let fx = f64::from(x - target.x) / f64::from(target.width.max(1));
    let fy = f64::from(y - target.y) / f64::from(target.height.max(1));
    [fx.clamp(0.0, 1.0), fy.clamp(0.0, 1.0)]
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
    #[test]
    fn widget_scales_with_dpi_and_stays_inside() {
        let target = Rect { x: -1920, y: 0, width: 1920, height: 1080 };
        let w = widget(target, 144, 1.0, [0.5, 0.5], (200.0, 100.0));
        assert_eq!(w, Rect { x: -960, y: 540, width: 300, height: 150 });
        let edge = widget(target, 96, 2.0, [1.0, 1.0], (200.0, 100.0));
        assert_eq!((edge.x + edge.width as i32, edge.y + edge.height as i32), (0, 1080));
    }
    #[test]
    fn dragged_position_round_trips() {
        let target = Rect { x: 100, y: 50, width: 1600, height: 900 };
        let f = fraction(target, 500, 275);
        assert_eq!(f, [0.25, 0.25]);
        assert_eq!(widget(target, 96, 1.0, f, (10.0, 10.0)).x, 500);
    }
}
