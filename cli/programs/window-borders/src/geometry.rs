use clap::ValueEnum;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Layout {
    Empty,
    Full,
    Strips,
}

/// Points, with a top-left origin. The frame is in screen coordinates;
/// the ring contours and viewports are local to that frame.
pub struct BorderGeometry {
    pub screen_frame: CGRect,
    pub outer: CGRect,
    pub inner: CGRect,
    pub radius: f64,
    pub width: f64,
    pub scale: f64,
    pub surfaces: Vec<CGRect>,
}

pub fn rect(x: f64, y: f64, width: f64, height: f64) -> CGRect {
    CGRect::new(CGPoint::new(x, y), CGSize::new(width, height))
}

impl BorderGeometry {
    pub fn new(target: CGRect, layout: Layout, width: f64, radius: f64, scale: f64) -> Self {
        // One point covers outer-edge antialiasing. There is no glow or shadow
        // to reserve additional transparent space for.
        let padding = 1.0;
        let margin = width + padding;
        let x = (target.origin.x - margin).floor();
        let y = (target.origin.y - margin).floor();
        let frame = rect(
            x,
            y,
            (target.origin.x + target.size.width + margin).ceil() - x,
            (target.origin.y + target.size.height + margin).ceil() - y,
        );
        let inner = rect(
            target.origin.x - x,
            target.origin.y - y,
            target.size.width,
            target.size.height,
        );
        let outer = rect(
            inner.origin.x - width,
            inner.origin.y - width,
            target.size.width + 2.0 * width,
            target.size.height + 2.0 * width,
        );
        let radius = radius
            .min(target.size.width / 2.0)
            .min(target.size.height / 2.0);
        let w = frame.size.width;
        let h = frame.size.height;
        // SkyLight quantizes these window regions to whole points even at 2x.
        // Cut on that grid, with one pixel of antialiasing coverage inside the
        // contour. Top/bottom own the corners; no two surfaces overlap.
        let antialias = 1.0 / scale;
        let middle_y = (h / 2.0).floor();
        let top_end = (inner.origin.y + radius + antialias).ceil().min(middle_y);
        let bottom_start = (inner.origin.y + inner.size.height - radius - antialias)
            .floor()
            .max(middle_y);
        let left_end = (inner.origin.x + antialias).ceil().min((w / 2.0).floor());
        let right_start = (inner.origin.x + inner.size.width - antialias)
            .floor()
            .max((w / 2.0).floor());
        let surfaces = match layout {
            Layout::Empty => vec![],
            Layout::Full => vec![rect(0.0, 0.0, w, h)],
            Layout::Strips => vec![
                rect(0.0, 0.0, w, top_end),
                rect(0.0, bottom_start, w, h - bottom_start),
                rect(0.0, top_end, left_end, bottom_start - top_end),
                rect(
                    right_start,
                    top_end,
                    w - right_start,
                    bottom_start - top_end,
                ),
            ],
        };
        Self {
            screen_frame: frame,
            outer,
            inner,
            radius,
            width,
            scale,
            surfaces,
        }
    }

    pub fn straight_top_edge(&self) -> CGRect {
        rect(
            self.inner.origin.x + self.radius,
            self.outer.origin.y,
            self.inner.size.width - 2.0 * self.radius,
            self.width,
        )
    }

    pub fn rgba8_bytes(&self) -> u64 {
        self.surfaces
            .iter()
            .map(|r| {
                (r.size.width * self.scale).ceil() as u64
                    * (r.size.height * self.scale).ceil() as u64
                    * 4
            })
            .sum()
    }
}
