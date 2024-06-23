from typing import List
import turbo_python


def example():
    return turbo_python.rusty(5)

def update(colors: List[turbo_python.Color]):
    color = turbo_python.Color()
    colors.append(turbo_python.Color())
    for color in colors:
        color.red += 1
        color.blue += 1
        color.green += 1
    print(f"Fftresults max amp: ", turbo_python.fft_result.get_max_amplitude())

    return colors
