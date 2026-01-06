import numpy as np
from sim.vector import norm


class World:
    def __init__(self, center, radius):
        self.center = center
        self.radius = radius

    def check_collision(self, ball):
        d = ball.pos - self.center
        dist = np.linalg.norm(d)
        hit = dist >= self.radius - ball.radius
        normal = norm(d)
        return hit, normal
