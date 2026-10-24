/// Pure geometry helper so window placement can be unit tested without a Mac.
/// `area` is (x, y, width, height) in physical pixels.
pub fn centered_origin(area: (f64, f64, f64, f64), window: (f64, f64)) -> (f64, f64) {
    let (ax, ay, aw, ah) = area;
    let (ww, wh) = window;
    let ww = ww.min(aw);
    let wh = wh.min(ah);
    (
        ax + ((aw - ww) / 2.0).round(),
        ay + ((ah - wh) / 2.0).round(),
    )
}

/// Keep a window fully inside the monitor it is being placed on.
pub fn clamp_to_area(area: (f64, f64, f64, f64), window: (f64, f64)) -> (f64, f64) {
    let (_, _, aw, ah) = area;
    (window.0.min(aw).max(1.0), window.1.min(ah).max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_inside_primary_display() {
        let origin = centered_origin((0.0, 0.0, 1440.0, 900.0), (1100.0, 720.0));
        assert_eq!(origin, (170.0, 90.0));
    }

    #[test]
    fn centers_inside_negative_origin_display() {
        let origin = centered_origin((-1920.0, 0.0, 1920.0, 1080.0), (1100.0, 720.0));
        assert_eq!(origin, (-1510.0, 180.0));
    }

    #[test]
    fn oversized_window_is_clamped_to_display() {
        assert_eq!(
            clamp_to_area((0.0, 0.0, 900.0, 600.0), (1200.0, 900.0)),
            (900.0, 600.0)
        );
    }
}
