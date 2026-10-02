//! Default xterm-256color values used to answer host color queries.

use alacritty_terminal::vte::ansi::Rgb;

const COLOR_COUNT: usize = 269;

/// Return the default RGB value for an xterm color slot.
pub(crate) fn default_color(index: usize) -> Rgb {
    let ansi = [
        Rgb { r: 0, g: 0, b: 0 },
        Rgb { r: 205, g: 0, b: 0 },
        Rgb { r: 0, g: 205, b: 0 },
        Rgb {
            r: 205,
            g: 205,
            b: 0,
        },
        Rgb { r: 0, g: 0, b: 238 },
        Rgb {
            r: 205,
            g: 0,
            b: 205,
        },
        Rgb {
            r: 0,
            g: 205,
            b: 205,
        },
        Rgb {
            r: 229,
            g: 229,
            b: 229,
        },
        Rgb {
            r: 127,
            g: 127,
            b: 127,
        },
        Rgb { r: 255, g: 0, b: 0 },
        Rgb { r: 0, g: 255, b: 0 },
        Rgb {
            r: 255,
            g: 255,
            b: 0,
        },
        Rgb {
            r: 92,
            g: 92,
            b: 255,
        },
        Rgb {
            r: 255,
            g: 0,
            b: 255,
        },
        Rgb {
            r: 0,
            g: 255,
            b: 255,
        },
        Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
    ];
    match index {
        0..=15 => ansi[index],
        16..=231 => {
            let cube = index - 16;
            let component = |value: usize| {
                if value == 0 {
                    0
                } else {
                    (55 + value * 40) as u8
                }
            };
            Rgb {
                r: component(cube / 36),
                g: component((cube / 6) % 6),
                b: component(cube % 6),
            }
        }
        233..=255 => {
            let shade = (8 + (index - 233) * 10) as u8;
            Rgb {
                r: shade,
                g: shade,
                b: shade,
            }
        }
        256 | 267 => Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
        257 | 268 => Rgb { r: 0, g: 0, b: 0 },
        258 => Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
        259..=266 => ansi[index - 259],
        _ if index < COLOR_COUNT => Rgb { r: 0, g: 0, b: 0 },
        _ => Rgb { r: 0, g: 0, b: 0 },
    }
}
