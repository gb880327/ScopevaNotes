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

/// Turn a remembered window size into one the current display can show.
///
/// `config.json` is a file the user can edit, so a stored value is untrusted
/// input: smaller than the window's minimum gets lifted to the minimum, larger
/// than the display gets pulled back onto it, and a missing value falls back to
/// `default` through the same rules (design D4).
pub fn clamp_window_size(
    saved: Option<(f64, f64)>,
    default: (f64, f64),
    min: (f64, f64),
    area: (f64, f64, f64, f64),
) -> (f64, f64) {
    let (aw, ah) = (area.2.max(1.0), area.3.max(1.0));
    let (sw, sh) = saved.unwrap_or(default);
    // `min` first so a stored 1x1 becomes usable, then the display so it never
    // spills off-screen; the outer `min` wins when the display itself is
    // smaller than the minimum (better a cramped window than an invisible one).
    let width = sw.max(min.0.min(aw)).min(aw);
    let height = sh.max(min.1.min(ah)).min(ah);
    (width, height)
}

/// Anchor a menu-bar panel under its icon. `area` is the monitor, `icon` the
/// tray button and `panel` the window size — all in the same unit (logical
/// pixels, converted by the caller). Horizontally centred on the icon, then
/// clamped into the monitor; when there is no room below the menu bar the
/// panel flips above the icon rather than hanging off-screen (design D6).
pub fn tray_anchor_origin(
    area: (f64, f64, f64, f64),
    icon: (f64, f64, f64, f64),
    panel: (f64, f64),
    gap: f64,
) -> (f64, f64) {
    let (ax, ay, aw, ah) = area;
    let (ix, iy, iw, ih) = icon;
    let (pw, ph) = (panel.0.min(aw), panel.1.min(ah));

    let mut x = ix + (iw - pw) / 2.0;
    let max_x = ax + aw - pw;
    if x > max_x {
        x = max_x;
    }
    if x < ax {
        x = ax;
    }

    let mut y = iy + ih + gap;
    let max_y = ay + ah - ph;
    if y > max_y {
        let above = iy - gap - ph;
        y = if above >= ay { above } else { max_y.max(ay) };
    }

    (x.round(), y.round())
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
    fn anchors_under_a_centred_icon() {
        // 24px menu bar, icon 22px wide at x=700, panel 480x320.
        let (x, y) = tray_anchor_origin(
            (0.0, 0.0, 1440.0, 900.0),
            (700.0, 0.0, 22.0, 24.0),
            (480.0, 320.0),
            6.0,
        );
        assert_eq!(y, 30.0);
        assert_eq!(x, 471.0, "icon centre 711 minus half the panel");
    }

    #[test]
    fn anchor_is_clamped_to_the_right_edge() {
        let (x, _) = tray_anchor_origin(
            (0.0, 0.0, 1440.0, 900.0),
            (1410.0, 0.0, 22.0, 24.0),
            (480.0, 320.0),
            6.0,
        );
        assert_eq!(x, 960.0, "1440 - 480");
    }

    #[test]
    fn anchor_respects_negative_origin_displays() {
        let (x, y) = tray_anchor_origin(
            (-1920.0, 0.0, 1920.0, 1080.0),
            (-1000.0, 0.0, 22.0, 24.0),
            (480.0, 320.0),
            6.0,
        );
        assert_eq!(x, -1229.0);
        assert_eq!(y, 30.0);
    }

    #[test]
    fn anchor_flips_above_the_icon_when_there_is_no_room_below() {
        let (_, y) = tray_anchor_origin(
            (0.0, 0.0, 1440.0, 300.0),
            (700.0, 0.0, 22.0, 24.0),
            (480.0, 320.0),
            6.0,
        );
        assert_eq!(y, 0.0, "clamped to the monitor top, never off-screen");
    }

    #[test]
    fn remembered_size_survives_a_normal_display() {
        assert_eq!(
            clamp_window_size(
                Some((1320.0, 860.0)),
                (1100.0, 720.0),
                (760.0, 480.0),
                (0.0, 0.0, 2560.0, 1440.0)
            ),
            (1320.0, 860.0)
        );
    }

    #[test]
    fn remembered_size_is_lifted_to_the_minimum() {
        assert_eq!(
            clamp_window_size(
                Some((200.0, 120.0)),
                (480.0, 320.0),
                (360.0, 220.0),
                (0.0, 0.0, 1680.0, 1050.0)
            ),
            (360.0, 220.0)
        );
    }

    #[test]
    fn remembered_size_is_pulled_onto_a_smaller_display() {
        assert_eq!(
            clamp_window_size(
                Some((2000.0, 1400.0)),
                (1100.0, 720.0),
                (760.0, 480.0),
                (0.0, 0.0, 1440.0, 900.0)
            ),
            (1440.0, 900.0)
        );
    }

    #[test]
    fn missing_size_falls_back_to_the_default_unless_the_display_is_smaller() {
        assert_eq!(
            clamp_window_size(
                None,
                (480.0, 320.0),
                (360.0, 220.0),
                (0.0, 0.0, 1680.0, 1050.0)
            ),
            (480.0, 320.0)
        );
        assert_eq!(
            clamp_window_size(
                None,
                (480.0, 320.0),
                (360.0, 220.0),
                (0.0, 0.0, 400.0, 300.0)
            ),
            (400.0, 300.0),
            "a display smaller than the default wins over the default"
        );
    }
}
