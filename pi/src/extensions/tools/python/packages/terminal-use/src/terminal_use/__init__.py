"""Operate persistent, visible Kitty terminals from Python."""

from .instance import KittyInstance, connect, launch, list_instances
from .models import Rect
from .session import Session

__all__ = ["KittyInstance", "Rect", "Session", "connect", "launch", "list_instances"]
