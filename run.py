import pygame
import numpy as np

from sim.ball import Ball
from sim.world import World
from sim.rules import on_collision
from sim.rng import seed_all
from sim.vector import vec
from render.renderer import Renderer
from render.palette import random_color
import config


def main(seed=0):
    seed_all(seed)
    pygame.init()

    renderer = Renderer((config.WIDTH, config.HEIGHT))
    clock = pygame.time.Clock()

    world = World(np.array(config.CENTER), config.WORLD_RADIUS)

    balls = [
        Ball(
            pos=vec(config.CENTER[0] + 40, config.CENTER[1]),
            vel=vec(180, 220),
            radius=6,
            color=random_color(),
        )
    ]

    frames = config.FPS * config.DURATION

    for _ in range(frames):
        renderer.clear()
        new_balls = []

        for b in balls:
            b.update(1 / config.FPS)
            hit, normal = world.check_collision(b)

            if hit and len(balls) < config.MAX_BALLS:
                new_balls.append(on_collision(b, normal))

            renderer.draw_ball(b)

        balls.extend(new_balls)
        renderer.draw_circle(config.CENTER, config.WORLD_RADIUS)
        renderer.present()
        clock.tick(config.FPS)

    pygame.quit()


if __name__ == "__main__":
    main(seed=42)
