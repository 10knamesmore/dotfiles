use std::fmt::Write;

use crate::{color::Style, graphics::Scene};

use super::Shape;

pub(super) fn draw(
    scene: &mut Scene,
    shape: Shape,
    (cx, cy): (f32, f32),
    (width, height): (f32, f32),
    style: Style,
) {
    let (x, y) = (cx - width / 2.0, cy - height / 2.0);
    let d = match shape {
        Shape::Plain | Shape::Rectangle | Shape::Rounded => {
            let radius = if matches!(shape, Shape::Rectangle) {
                0.0
            } else {
                8.0
            };
            scene.rect(x, y, width, height, radius, style.fill, style.stroke);
            return;
        }
        Shape::Circle => {
            let r = width / 2.0;
            let right = x + width;
            format!("M {x} {cy} A {r} {r} 0 0 1 {right} {cy} A {r} {r} 0 0 1 {x} {cy} Z")
        }
        Shape::Hexagon => {
            let right = x + width;
            let bottom = y + height;
            let left_corner = x + 15.0;
            let right_corner = right - 15.0;
            format!(
                "M {x} {cy} L {left_corner} {y} H {right_corner} L {right} {cy} L {right_corner} {bottom} H {left_corner} Z"
            )
        }
        Shape::Bang => {
            let shoulders = [
                (0.0, 0.5),
                (0.1, 0.34),
                (0.04, 0.10),
                (0.23, 0.21),
                (0.34, 0.0),
                (0.45, 0.17),
                (0.65, 0.05),
                (0.72, 0.20),
                (0.94, 0.06),
                (0.90, 0.32),
                (1.0, 0.5),
            ];
            let mut d = format!("M {x} {cy}");
            for (u, v) in shoulders.into_iter().skip(1).chain(
                shoulders
                    .into_iter()
                    .rev()
                    .skip(1)
                    .map(|(u, v)| (u, 1.0 - v)),
            ) {
                write!(d, " L {} {}", x + u * width, y + v * height).unwrap();
            }
            d.push_str(" Z");
            d
        }
        Shape::Cloud => {
            let mut d = format!("M {x} {cy}");
            for curve in [
                [(0.0, 0.28), (0.06, 0.18), (0.15, 0.23)],
                [(0.21, 0.00), (0.41, 0.00), (0.50, 0.17)],
                [(0.59, 0.00), (0.79, 0.00), (0.85, 0.23)],
                [(0.94, 0.18), (1.00, 0.28), (1.00, 0.50)],
                [(1.00, 0.72), (0.94, 0.82), (0.85, 0.77)],
                [(0.79, 1.00), (0.59, 1.00), (0.50, 0.83)],
                [(0.41, 1.00), (0.21, 1.00), (0.15, 0.77)],
                [(0.06, 0.82), (0.00, 0.72), (0.00, 0.50)],
            ] {
                d.push_str(" C");
                for (u, v) in curve {
                    write!(d, " {} {}", x + u * width, y + v * height).unwrap();
                }
            }
            d.push_str(" Z");
            d
        }
    };
    scene.path(&d, style.fill, style.stroke, 1.5);
}
