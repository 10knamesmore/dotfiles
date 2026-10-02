//! Default xterm-256color colors configured on every terminal core.
//!
//! The core answers palette and dynamic-color queries from these defaults, so the values must be
//! set before any VT data is processed.

use libghostty_vt::style::{Palette, RgbColor};

/// Default foreground reported for cells without an explicit color and for OSC 10 queries.
pub(crate) const DEFAULT_FOREGROUND: RgbColor = RgbColor {
    r: 255,
    g: 255,
    b: 255,
};

/// Default background reported for cells without an explicit color and for OSC 11 queries.
pub(crate) const DEFAULT_BACKGROUND: RgbColor = RgbColor { r: 0, g: 0, b: 0 };

/// Default cursor color reported for OSC 12 queries.
pub(crate) const DEFAULT_CURSOR: RgbColor = RgbColor {
    r: 255,
    g: 255,
    b: 255,
};

/// Build the standard xterm 256-color palette.
///
/// Indices 0-15 are the ANSI colors, 16-231 the 6x6x6 color cube, and 232-255 the grayscale ramp.
pub(crate) fn xterm_palette() -> Palette {
    let ansi = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    let mut colors = [RgbColor::default(); 256];
    for (index, (r, g, b)) in ansi.into_iter().enumerate() {
        colors[index] = RgbColor { r, g, b };
    }
    for (cube, color) in colors[16..232].iter_mut().enumerate() {
        let component = |value: usize| -> u8 {
            if value == 0 {
                0
            } else {
                (55 + value * 40) as u8
            }
        };
        *color = RgbColor {
            r: component(cube / 36),
            g: component((cube / 6) % 6),
            b: component(cube % 6),
        };
    }
    for (index, color) in colors[232..].iter_mut().enumerate() {
        let shade = (8 + index * 10) as u8;
        *color = RgbColor {
            r: shade,
            g: shade,
            b: shade,
        };
    }
    Palette(colors)
}
