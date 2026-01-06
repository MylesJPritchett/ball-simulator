import pygame
import numpy as np
import random
import math

# ---------------- CONFIG ----------------
WIDTH, HEIGHT = 800, 800
FPS = 60
DURATION = 20  # seconds
MAX_BALLS = 200

CIRCLE_RADIUS = 360
CENTER = np.array([WIDTH // 2, HEIGHT // 2])

SPAWN_DECAY = 0.85
SPEED_BOOST = 1.03

# ----------------------------------------


def reflect(v, n):
    return v - 2 * np.dot(v, n) * n


class Ball:
    def __init__(self, pos, vel, radius, color, generation=0):
        self.pos = pos
        self.vel = vel
        self.radius = radius
        self.color = color
        self.generation = generation

    def update(self, dt):
        self.pos += self.vel * dt

    def check_collision(self):
        d = self.pos - CENTER
        dist = np.linalg.norm(d)
        return dist >= CIRCLE_RADIUS - self.radius, d / dist if dist != 0 else d


def random_color():
    return pygame.Color(
        random.randint(80, 255), random.randint(80, 255), random.randint(80, 255)
    )


def main(seed=0):
    random.seed(seed)
    np.random.seed(seed)

    pygame.init()
    screen = pygame.display.set_mode((WIDTH, HEIGHT))
    clock = pygame.time.Clock()

    balls = [
        Ball(
            CENTER + np.array([50, 0], dtype=float),
            np.array([180, 220], dtype=float),
            30,
            random_color(),
        )
    ]

    trail = pygame.Surface((WIDTH, HEIGHT), pygame.SRCALPHA)

    running = True
    frames = FPS * DURATION
    for _ in range(frames):
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                running = False
        if not running:
            break

        screen.fill((10, 10, 10))
        trail.fill((0, 0, 0, 40))

        new_balls = []

        for b in balls:
            b.update(1 / FPS)
            hit, normal = b.check_collision()

            if hit:
                b.pos = CENTER + normal * (CIRCLE_RADIUS - b.radius)
                b.vel = reflect(b.vel, normal) * SPEED_BOOST
                b.color = random_color()

                if len(balls) + len(new_balls) < MAX_BALLS:
                    angle = random.uniform(-0.5, 0.5)
                    rot = np.array(
                        [
                            [math.cos(angle), -math.sin(angle)],
                            [math.sin(angle), math.cos(angle)],
                        ]
                    )

                    new_vel = rot @ b.vel * SPAWN_DECAY
                    new_balls.append(
                        Ball(
                            b.pos.copy(),
                            new_vel,
                            max(20, b.radius * 0.9),
                            random_color(),
                            b.generation + 1,
                        )
                    )

            pygame.draw.circle(trail, b.color, b.pos.astype(int), b.radius)

        balls.extend(new_balls)

        # draw boundary
        pygame.draw.circle(screen, (220, 220, 220), CENTER, CIRCLE_RADIUS, 2)

        screen.blit(trail, (0, 0))
        pygame.display.flip()
        clock.tick(FPS)

    pygame.quit()


if __name__ == "__main__":
    main(seed=42)
