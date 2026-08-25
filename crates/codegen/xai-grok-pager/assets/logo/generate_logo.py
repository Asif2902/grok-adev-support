#!/usr/bin/env python3
"""Generates the ADEVGrok braille-art logo (logo07.txt / logo05.txt).

Renders a bold "AG" monogram as a boolean pixel grid, then packs it into
Unicode Braille Pattern characters (U+2800-U+28FF), where each character
encodes a 2-wide x 4-tall block of dots:

    1 4
    2 5
    3 6
    7 8

Run `python3 generate_logo.py` from this directory (or anywhere — paths are
relative to the script) to regenerate both files after adjusting the shape
functions below. logo07.txt is the full (7-row) logo used in the hero box
and tall terminals; logo05.txt is the small (5-row) variant used at medium
terminal heights (see crates/codegen/xai-grok-pager/src/views/welcome/logo.rs).

Add `--preview` to print the pixel grids as ASCII before writing the files.
"""

import sys
from pathlib import Path

# Bit offset for each (dx, dy) position within a braille cell's 2x4 dot grid.
DOT_BITS = {
    (0, 0): 0x01, (0, 1): 0x02, (0, 2): 0x04, (0, 3): 0x40,
    (1, 0): 0x08, (1, 1): 0x10, (1, 2): 0x20, (1, 3): 0x80,
}


def new_grid(width: int, height: int) -> list[list[bool]]:
    return [[False] * width for _ in range(height)]


def fill_rect(grid, x0: int, y0: int, x1: int, y1: int) -> None:
    """Fills the inclusive rectangle [x0, x1] x [y0, y1], clipped to the grid."""
    height, width = len(grid), len(grid[0])
    for y in range(max(0, y0), min(height, y1 + 1)):
        for x in range(max(0, x0), min(width, x1 + 1)):
            grid[y][x] = True


def _stamp(grid, x: int, y: int, r: int) -> None:
    """Square brush centered on (x, y) — gives diagonal strokes even weight."""
    height, width = len(grid), len(grid[0])
    for yy in range(y - r, y + r + 1):
        for xx in range(x - r, x + r + 1):
            if 0 <= yy < height and 0 <= xx < width:
                grid[yy][xx] = True


def _thick_line(grid, xa, ya, xb, yb, r) -> None:
    steps = 2 * max(abs(xb - xa), abs(yb - ya)) + 1
    for i in range(steps + 1):
        t = i / steps
        _stamp(grid, round(xa + (xb - xa) * t), round(ya + (yb - ya) * t), r)


def make_ag_monogram(width: int, height: int) -> list[list[bool]]:
    """A bold geometric "AG" pair, proportioned for a roughly square dot canvas."""
    grid = new_grid(width, height)
    margin_x = max(1, round(width * 0.04))
    margin_y = max(1, round(height * 0.07))
    y0, y1 = margin_y, height - 1 - margin_y
    inner_w = width - 2 * margin_x
    letter_w = max(6, round(inner_w * 0.42))

    ax0 = margin_x
    ax1 = ax0 + letter_w - 1
    gx1 = width - 1 - margin_x
    gx0 = gx1 - letter_w + 1

    s = max(2, round(height * 0.13))  # stroke/stem thickness

    # --- A: peaked cap, two stems, crossbar; hollow counters ---
    apex_x = (ax0 + ax1) // 2
    y_cap = y0 + max(3, round((y1 - y0) * 0.22))
    for yy in range(y0, y_cap + 1):
        frac = (yy - y0) / max(1, y_cap - y0)
        half = round((ax1 - ax0) / 2 * frac)
        fill_rect(grid, apex_x - half, yy, apex_x + half, yy)
    fill_rect(grid, ax0, y_cap, ax0 + s - 1, y1)           # left stem
    fill_rect(grid, ax1 - s + 1, y_cap, ax1, y1)           # right stem
    bar_y = y0 + round((y1 - y0) * 0.62)
    fill_rect(grid, ax0, bar_y, ax1, bar_y + s - 1)        # crossbar

    # --- G: open-top-right box with an inward spur ---
    g_s = max(2, round(height * 0.17))
    fill_rect(grid, gx0, y0, gx1, y0 + g_s - 1)            # top bar
    fill_rect(grid, gx0, y0, gx0 + g_s - 1, y1)            # left stem
    fill_rect(grid, gx0, y1 - g_s + 1, gx1, y1)            # bottom bar
    mid_y = y0 + round((y1 - y0) * 0.46)
    fill_rect(grid, gx1 - g_s + 1, mid_y, gx1, y1)         # right lower stem
    spur_x = gx0 + round((gx1 - gx0) * 0.55)
    fill_rect(grid, spur_x, mid_y, gx1, mid_y + g_s - 1)   # spur into the bowl
    return grid


def to_braille(grid) -> str:
    """Packs a boolean pixel grid into rows of Braille characters."""
    height, width = len(grid), len(grid[0])
    lines = []
    for cell_y in range(0, height, 4):
        row_chars = []
        for cell_x in range(0, width, 2):
            code = 0
            for (dx, dy), bit in DOT_BITS.items():
                x, y = cell_x + dx, cell_y + dy
                if y < height and x < width and grid[y][x]:
                    code |= bit
            row_chars.append(chr(0x2800 + code))
        lines.append("".join(row_chars))
    return "\n".join(lines) + "\n"


def preview(grid) -> str:
    return "\n".join("".join("#" if px else "." for px in row) for row in grid)


def main() -> None:
    show_preview = "--preview" in sys.argv[1:]
    here = Path(__file__).parent
    full = make_ag_monogram(28, 28)  # 14 cols x 7 rows of braille cells
    small = make_ag_monogram(20, 20)  # 10 cols x 5 rows of braille cells
    if show_preview:
        print("full:\n" + preview(full))
        print("small:\n" + preview(small))
    (here / "logo07.txt").write_text(to_braille(full), encoding="utf-8")
    (here / "logo05.txt").write_text(to_braille(small), encoding="utf-8")
    print("wrote logo07.txt and logo05.txt")


if __name__ == "__main__":
    main()
