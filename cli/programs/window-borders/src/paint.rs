use crate::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderPaint {
    pub color: Color,
    pub top_segments: Option<TopSegments>,
}

/// Equal-width slots along the straight top edge; corners keep the border color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopSegments {
    pub count: usize,
    /// Zero-based index of the segment painted with the border color.
    pub active_index: usize,
    pub inactive_color: Color,
}
