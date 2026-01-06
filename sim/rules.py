import math
import random
import numpy as np
from sim.vector import reflect
from sim.ball import Ball


def on_collision(ball, normal):
    ball.vel = reflect(ball.vel, normal) * 1.05

    angle = random.uniform(-0.5, 0.5)
    rot = np.array(
        [[math.cos(angle), -math.sin(angle)], [math.sin(angle), math.cos(angle)]]
    )

    child = Ball(
        pos=ball.pos.copy(),
        vel=rot @ ball.vel * 0.85,
        radius=max(3, ball.radius * 0.9),
        color=ball.color,
        generation=ball.generation + 1,
    )

    return child
