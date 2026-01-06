import pygame


class Renderer:
    def __init__(self, size):
        self.screen = pygame.display.set_mode(size)
        self.trail = pygame.Surface(size, pygame.SRCALPHA)

    def clear(self):
        self.trail.fill((0, 0, 0, 40))
        self.screen.fill((10, 10, 10))

    def draw_ball(self, ball):
        pygame.draw.circle(
            self.trail, ball.color, ball.pos.astype(int), int(ball.radius)
        )

    def draw_circle(self, center, radius):
        pygame.draw.circle(self.screen, (220, 220, 220), center, radius, 2)

    def present(self):
        self.screen.blit(self.trail, (0, 0))
        pygame.display.flip()
