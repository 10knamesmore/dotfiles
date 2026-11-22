use std::fmt::Write;

use crate::color::Rgb;
use crate::graphics::Scene;

use super::super::Outline;
use super::routing::Side;
use super::{Point, Text, Vertex, cells};

pub(super) fn size(outline: Outline, text: &Text<'_>) -> Point {
    use Outline::*;
    let (width, height) = (text.width.max(12.0), text.height);
    let (w, h) = match outline {
        Initial | Final => (20.0, 20.0),
        Synchronization { vertical: false } => ((width + 30.0).max(80.0), height + 5.0),
        Synchronization { vertical: true } => (width + 16.0, height + 60.0),
        Decision => (width * 1.95 + 30.0, height * 1.95 + 22.0),
        Circle | DoubleCircle => {
            let side = (width + 32.0).hypot(height + 26.0);
            (side, side)
        }
        Triangle { .. } => (width * 2.15 + 36.0, height * 2.15 + 26.0),
        Stadium => (width + (height + 18.0) * 0.8 + 20.0, height + 18.0),
        Flag | Hexagon | SlantedRight | SlantedLeft | Trapezoid | TrapezoidInverted => {
            (width * 1.32 + 32.0, height + 22.0)
        }
        Database | Document { .. } => (width + 32.0, height + 37.0),
        Subroutine | Brace { .. } => (width + 48.0, height + 20.0),
        _ => (width + 32.0, height + 18.0),
    };
    Point(cells(w), cells(h))
}

fn polygon(outline: Outline) -> Option<&'static [(f32, f32)]> {
    use Outline::*;
    Some(match outline {
        Decision => &[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)],
        Flag => &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.14, 0.5)],
        Hexagon => &[
            (0.14, 0.0),
            (0.86, 0.0),
            (1.0, 0.5),
            (0.86, 1.0),
            (0.14, 1.0),
            (0.0, 0.5),
        ],
        SlantedRight => &[(0.14, 0.0), (1.0, 0.0), (0.86, 1.0), (0.0, 1.0)],
        SlantedLeft => &[(0.0, 0.0), (0.86, 0.0), (1.0, 1.0), (0.14, 1.0)],
        Trapezoid => &[(0.14, 0.0), (0.86, 0.0), (1.0, 1.0), (0.0, 1.0)],
        TrapezoidInverted => &[(0.0, 0.0), (1.0, 0.0), (0.86, 1.0), (0.14, 1.0)],
        Triangle { down: false } => &[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)],
        Triangle { down: true } => &[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)],
        _ => return None,
    })
}

pub(super) fn draw(
    scene: &mut Scene,
    vertex: &Vertex<'_>,
    fill: Option<Rgb>,
    stroke: Option<Rgb>,
    corners: f32,
) {
    use Outline::*;
    let (x, y, w, h) = vertex.bounds.pixels();
    let (right, bottom) = (x + w, y + h);
    if let Some(points) = polygon(vertex.outline) {
        let mut path = String::new();
        for (i, (dx, dy)) in points.iter().enumerate() {
            let command = if i == 0 { 'M' } else { 'L' };
            write!(path, "{command}{} {} ", x + w * dx, y + h * dy).unwrap();
        }
        path.push('Z');
        scene.path(&path, fill, stroke, 1.5);
        return;
    }
    match vertex.outline {
        Initial => scene.rect(x, y, w, h, h / 2.0, stroke, None),
        Final => {
            scene.rect(x, y, w, h, h / 2.0, fill, stroke);
            scene.rect(x + 5.0, y + 5.0, w - 10.0, h - 10.0, 5.0, stroke, None);
        }
        Synchronization { vertical: false } => {
            scene.rect(x, y, w, h, 3.0, vertex.style.fill.or(stroke), stroke);
        }
        Synchronization { vertical: true } => {
            let caption = vertex.text.height;
            scene.rect(
                x + w / 2.0 - 5.0,
                y + caption,
                10.0,
                h - caption,
                3.0,
                vertex.style.fill.or(stroke),
                stroke,
            );
        }
        Note => note(scene, (x, y, w, h), fill, stroke),
        Circle | DoubleCircle => {
            scene.rect(x, y, w, h, h / 2.0, fill, stroke);
            if matches!(vertex.outline, DoubleCircle) {
                scene.rect(
                    x + 5.0,
                    y + 5.0,
                    w - 10.0,
                    h - 10.0,
                    h / 2.0 - 5.0,
                    None,
                    stroke,
                );
            }
        }
        Stadium => scene.rect(x, y, w, h, h / 2.0, fill, stroke),
        Rounded => scene.rect(x, y, w, h, 9.0, fill, stroke),
        FixedRectangle => scene.rect(x, y, w, h, 0.0, fill, stroke),
        Subroutine => {
            scene.rect(x, y, w, h, 0.0, fill, stroke);
            let (a, b) = (x + 8.0, right - 8.0);
            scene.path(
                &format!("M{a} {y}V{bottom} M{b} {y}V{bottom}"),
                None,
                stroke,
                1.5,
            );
        }
        Database => {
            let rx = w / 2.0;
            let cap = y + 9.0;
            let base = bottom - 9.0;
            scene.path(
                &format!("M{x} {cap}A{rx} 9 0 0 1 {right} {cap}V{base}A{rx} 9 0 0 1 {x} {base}Z"),
                fill,
                stroke,
                1.5,
            );
            scene.path(
                &format!("M{x} {cap}A{rx} 9 0 0 0 {right} {cap}"),
                None,
                stroke,
                1.5,
            );
        }
        Document { lined } => {
            let a = x + w / 3.0;
            let b = x + w * 2.0 / 3.0;
            let wave = bottom - 8.0;
            scene.path(
                &format!(
                    "M{x} {y}H{right}V{wave}C{b} {} {a} {} {x} {wave}Z",
                    wave - 24.0,
                    wave + 24.0
                ),
                fill,
                stroke,
                1.5,
            );
            if lined {
                let (a, b, rule) = (x + 9.0, right - 9.0, y + 10.0);
                scene.path(&format!("M{a} {rule}H{b}"), None, stroke, 1.5);
            }
        }
        Brace {
            left,
            right: show_right,
        } => {
            let mid = y + h / 2.0;
            for (show, edge, sign) in [(left, x, 1.0), (show_right, right, -1.0)] {
                if show {
                    let inner = edge + sign * 13.0;
                    let stem = edge + sign * 6.0;
                    scene.path(
                        &format!("M{inner} {y}Q{stem} {y} {stem} {}V{}Q{stem} {mid} {edge} {mid}Q{stem} {mid} {stem} {}V{}Q{stem} {bottom} {inner} {bottom}",
                            y + 7.0, mid - 7.0, mid + 7.0, bottom - 7.0),
                        None, stroke, 1.5,
                    );
                }
            }
        }
        Text => {
            if vertex.style.fill.is_some() {
                scene.rect(x, y, w, h, 0.0, fill, None);
            }
        }
        Fork => {
            scene.rect(x, y, w, h, 3.0, fill, stroke);
            let rail = y + 5.0;
            scene.path(&format!("M{x} {rail}H{right}"), None, stroke, 4.0);
        }
        _ => scene.rect(x, y, w, h, corners, fill, stroke),
    }
}

pub(super) fn note(
    scene: &mut Scene,
    (x, y, w, h): (f32, f32, f32, f32),
    fill: Option<Rgb>,
    stroke: Option<Rgb>,
) {
    let (right, bottom) = (x + w, y + h);
    let fold = right - 10.0;
    let corner = y + 10.0;
    scene.path(
        &format!("M{x} {y}H{fold}L{right} {corner}V{bottom}H{x}Z M{fold} {y}V{corner}H{right}"),
        fill,
        stroke,
        1.5,
    );
}

pub(super) fn attachment(vertex: &Vertex<'_>, side: Side, port: Point) -> (f32, f32) {
    let (x, y, w, h) = vertex.bounds.pixels();
    let (px, py) = port.pixels();
    let horizontal = side.is_horizontal();
    let high = matches!(side, Side::Right | Side::Down);
    if matches!(vertex.outline, Outline::Synchronization { vertical: true }) {
        let bar = x + w / 2.0;
        return if horizontal {
            (
                bar + if high { 5.0 } else { -5.0 },
                py.max(y + vertex.text.height),
            )
        } else {
            (
                px.clamp(bar - 5.0, bar + 5.0),
                if high { y + h } else { y + vertex.text.height },
            )
        };
    }
    let offset = if horizontal {
        (py - y) / h
    } else {
        (px - x) / w
    };
    let face: f32 = if high { 1.0 } else { 0.0 };
    let position = if vertex.group {
        face
    } else if let Some(points) = polygon(vertex.outline) {
        points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .filter_map(|(&(ax, ay), &(bx, by))| {
                let (a, b, start, stop) = if horizontal {
                    (ay, by, ax, bx)
                } else {
                    (ax, bx, ay, by)
                };
                ((a.min(b)..=a.max(b)).contains(&offset) && a != b)
                    .then(|| start + (stop - start) * (offset - a) / (b - a))
            })
            .reduce(|a, b| if high { a.max(b) } else { a.min(b) })
            .unwrap_or(face)
    } else {
        let (span, across) = if horizontal { (w, h) } else { (h, w) };
        let radius = match vertex.outline {
            Outline::Circle
            | Outline::DoubleCircle
            | Outline::Stadium
            | Outline::Initial
            | Outline::Final => h / 2.0,
            Outline::Rounded => 9.0,
            _ => 0.0,
        };
        let corner = radius - across * offset.min(1.0 - offset);
        let inset = match vertex.outline {
            Outline::Database if !horizontal => 9.0 * (1.0 - ellipse(offset)),
            Outline::Document { .. } if matches!(side, Side::Down) => {
                8.0 + 72.0 * offset * (1.0 - offset) * (2.0 * offset - 1.0)
            }
            _ if corner > 0.0 => radius - (radius * radius - corner * corner).max(0.0).sqrt(),
            _ => 0.0,
        };
        if high {
            1.0 - inset / span
        } else {
            inset / span
        }
    };
    if horizontal {
        (x + w * position, py)
    } else {
        (px, y + h * position)
    }
}

fn ellipse(offset: f32) -> f32 {
    (1.0 - (2.0 * offset - 1.0).powi(2)).max(0.0).sqrt()
}
