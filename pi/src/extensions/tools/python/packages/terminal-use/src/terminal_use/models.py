"""Python values returned by Kitty screen and lifecycle operations."""

from dataclasses import dataclass


@dataclass(frozen=True)
class Rect:
    x: int
    y: int
    width: int
    height: int

    def __post_init__(self) -> None:
        if self.x < 0 or self.y < 0 or self.width <= 0 or self.height <= 0:
            raise ValueError(
                "Rect requires nonnegative coordinates and positive dimensions"
            )

    def as_dict(self) -> dict[str, int]:
        return {"x": self.x, "y": self.y, "width": self.width, "height": self.height}


RectInput = Rect | dict[str, int] | tuple[int, int, int, int] | list[int]


def rectangle(value: RectInput | None) -> dict[str, int] | None:
    if value is None:
        return None
    if isinstance(value, Rect):
        return value.as_dict()
    if isinstance(value, dict):
        return Rect(**value).as_dict()
    return Rect(*value).as_dict()
