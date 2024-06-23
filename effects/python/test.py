from typing import List
import turbo_python


def example():
    return turbo_python.rusty(5)


def update(colors: List[turbo_python.Color]):
    for color in colors:
        color.red = min(255, color.red + 1)
        color.blue = min(255, color.blue + 1)
        color.green = min(255, color.green + 1)

    return colors
