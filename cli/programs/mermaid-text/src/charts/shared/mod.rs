pub(super) mod architecture_layout;
pub(super) mod area;
pub(super) mod block_layout;
pub(super) mod board;
pub(super) mod cards;
pub(super) mod history;
pub(super) mod lane;
pub(super) mod model_header;
pub(super) mod plot;
pub(super) mod sequence_diagram;
pub(super) mod sets;

#[derive(Clone, Copy)]
pub(super) enum Direction {
    Right,
    Left,
    Down,
    Up,
}
