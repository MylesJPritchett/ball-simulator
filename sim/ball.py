from dataclasses import dataclass
import numpy as np


@dataclass
class Ball:
    pos: np.ndarray
    vel: np.ndarray
    radius: float
    color: tuple
    generation: int = 0

    def update(self, dt: float):
        self.pos += self.vel * dt
