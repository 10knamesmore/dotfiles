use kurbo::{Arc, PathEl, Point, SvgArc, Vec2};
use svgtypes::{PathParser, PathSegment};
use tiny_skia::{Path, PathBuilder, Rect};

const MAX_COORDINATE: f32 = 32768.0;
const PATH_BUDGET: &str = "PNG scene exceeds its drawing budget";
const INVALID_COORDINATES: &str = "invalid PNG path coordinates";

pub(super) fn valid_coordinates(values: &[f32]) -> bool {
    values.iter().all(|value| value.abs() <= MAX_COORDINATE)
}

pub(super) fn rectangle(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let rect = Rect::from_xywh(x, y, w, h)?;
    let r = r.clamp(0.0, rect.width().min(rect.height()) / 2.0);
    if r == 0.0 {
        return Some(PathBuilder::from_rect(rect));
    }
    let (right, bottom) = (rect.right(), rect.bottom());
    let k = r * 0.552_284_8;
    let mut path = PathBuilder::new();
    path.move_to(x + r, y);
    path.line_to(right - r, y);
    path.cubic_to(right - r + k, y, right, y + r - k, right, y + r);
    path.line_to(right, bottom - r);
    path.cubic_to(
        right,
        bottom - r + k,
        right - r + k,
        bottom,
        right - r,
        bottom,
    );
    path.line_to(x + r, bottom);
    path.cubic_to(x + r - k, bottom, x, bottom - r + k, x, bottom - r);
    path.line_to(x, y + r);
    path.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    path.close();
    path.finish()
}

enum Control {
    Cubic(Point),
    Quadratic(Point),
}

struct Builder {
    path: PathBuilder,
    remaining: usize,
}

impl Builder {
    fn push(&mut self, element: PathEl) -> Result<(), &'static str> {
        self.remaining = self.remaining.checked_sub(1).ok_or(PATH_BUDGET)?;
        let point = |p: Point| {
            let xy = (p.x as f32, p.y as f32);
            valid_coordinates(&[xy.0, xy.1])
                .then_some(xy)
                .ok_or(INVALID_COORDINATES)
        };
        match element {
            PathEl::MoveTo(to) => {
                let (x, y) = point(to)?;
                self.path.move_to(x, y);
            }
            PathEl::LineTo(to) => {
                let (x, y) = point(to)?;
                self.path.line_to(x, y);
            }
            PathEl::QuadTo(first, to) => {
                let (x1, y1) = point(first)?;
                let (x, y) = point(to)?;
                self.path.quad_to(x1, y1, x, y);
            }
            PathEl::CurveTo(first, second, to) => {
                let (x1, y1) = point(first)?;
                let (x2, y2) = point(second)?;
                let (x, y) = point(to)?;
                self.path.cubic_to(x1, y1, x2, y2, x, y);
            }
            PathEl::ClosePath => self.path.close(),
        }
        Ok(())
    }

    fn arc(&mut self, arc: SvgArc) -> Result<(), &'static str> {
        if !valid_coordinates(&[
            arc.to.x as f32,
            arc.to.y as f32,
            arc.radii.x as f32,
            arc.radii.y as f32,
            arc.x_rotation as f32,
        ]) {
            return Err(INVALID_COORDINATES);
        }
        if (arc.from - arc.to).hypot2() <= f64::MIN_POSITIVE {
            return Ok(());
        }
        let Some(arc) = Arc::from_svg_arc(&arc) else {
            return self.push(PathEl::LineTo(arc.to));
        };
        if !valid_coordinates(&[arc.radii.x as f32, arc.radii.y as f32]) {
            return Err(INVALID_COORDINATES);
        }
        for element in arc.append_iter(0.1) {
            self.push(element)?;
        }
        Ok(())
    }
}

pub(super) fn svg(data: &str, segment_limit: usize) -> Result<Option<Path>, &'static str> {
    let mut builder = Builder {
        path: PathBuilder::new(),
        remaining: segment_limit,
    };
    let (mut current, mut start) = (Point::ORIGIN, Point::ORIGIN);
    let mut previous = None;
    for segment in PathParser::from(data) {
        let segment = segment.map_err(|_| "invalid PNG SVG path")?;
        let point = |x, y| {
            if segment.is_abs() {
                Point::new(x, y)
            } else {
                current + Vec2::new(x, y)
            }
        };
        let mut control = None;
        let (element, end) = match segment {
            PathSegment::MoveTo { x, y, .. } => {
                start = point(x, y);
                (PathEl::MoveTo(start), start)
            }
            PathSegment::LineTo { x, y, .. } => {
                let to = point(x, y);
                (PathEl::LineTo(to), to)
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                let to = Point::new(if abs { x } else { current.x + x }, current.y);
                (PathEl::LineTo(to), to)
            }
            PathSegment::VerticalLineTo { abs, y } => {
                let to = Point::new(current.x, if abs { y } else { current.y + y });
                (PathEl::LineTo(to), to)
            }
            PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
                ..
            } => {
                let (second, to) = (point(x2, y2), point(x, y));
                control = Some(Control::Cubic(second));
                (PathEl::CurveTo(point(x1, y1), second, to), to)
            }
            PathSegment::SmoothCurveTo { x2, y2, x, y, .. } => {
                let first = match previous {
                    Some(Control::Cubic(p)) => current + (current - p),
                    _ => current,
                };
                let (second, to) = (point(x2, y2), point(x, y));
                control = Some(Control::Cubic(second));
                (PathEl::CurveTo(first, second, to), to)
            }
            PathSegment::Quadratic { x1, y1, x, y, .. } => {
                let (first, to) = (point(x1, y1), point(x, y));
                control = Some(Control::Quadratic(first));
                (PathEl::QuadTo(first, to), to)
            }
            PathSegment::SmoothQuadratic { x, y, .. } => {
                let first = match previous {
                    Some(Control::Quadratic(p)) => current + (current - p),
                    _ => current,
                };
                let to = point(x, y);
                control = Some(Control::Quadratic(first));
                (PathEl::QuadTo(first, to), to)
            }
            PathSegment::EllipticalArc {
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
                ..
            } => {
                let to = point(x, y);
                builder.arc(SvgArc {
                    from: current,
                    to,
                    radii: Vec2::new(rx, ry),
                    x_rotation: x_axis_rotation.to_radians(),
                    large_arc,
                    sweep,
                })?;
                current = to;
                previous = None;
                continue;
            }
            PathSegment::ClosePath { .. } => (PathEl::ClosePath, start),
        };
        builder.push(element)?;
        current = end;
        previous = control;
    }
    Ok(builder.path.finish())
}
