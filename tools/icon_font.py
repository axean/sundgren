"""The VS Code extension's icon font: one glyph, the tree from the logo, for the status bar.

Usage: uv run tools/icon_font.py OUT.woff. The extension ships it as
editors/vscode/sundgren-icons.woff (`contributes.icons`, character U+E000). The glyph is drawn
here from circles and round-capped strokes on a 24-unit grid (y down, like SVG), filling the em
box like VS Code's codicons. Fixed timestamps, so the output only changes with the drawing.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

Point = tuple[float, float]

CHARACTER = 0xE000
EM = 1000
GRID = 24
# 2026-11-02, seconds since 1904 (the font epoch).
TIMESTAMP = 3844800000
SEGMENTS = 64


def circle(cx: float, cy: float, r: float) -> list[Point]:
    return [
        (
            cx + r * math.cos(2 * math.pi * i / SEGMENTS),
            cy + r * math.sin(2 * math.pi * i / SEGMENTS),
        )
        for i in range(SEGMENTS)
    ]


def capsule(a: Point, b: Point, width: float) -> list[Point]:
    """A straight stroke from a to b with round caps."""
    r = width / 2
    angle = math.atan2(b[1] - a[1], b[0] - a[0])
    half = SEGMENTS // 2
    points: list[Point] = []
    for end, start in ((b, angle - math.pi / 2), (a, angle + math.pi / 2)):
        for i in range(half + 1):
            t = start + math.pi * i / half
            points.append((end[0] + r * math.cos(t), end[1] + r * math.sin(t)))
    return points


def cubic(p0: Point, p1: Point, p2: Point, p3: Point, steps: int = 8) -> list[Point]:
    def at(t: float) -> Point:
        u = 1 - t
        return (
            u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
            u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1],
        )

    return [at(i / steps) for i in range(steps + 1)]


def stroke(points: list[Point], width: float) -> list[list[Point]]:
    """A polyline as overlapping capsules (nonzero fill unions them)."""
    return [capsule(a, b, width) for a, b in zip(points, points[1:], strict=False)]


def area(points: list[Point]) -> float:
    return sum(
        x0 * y1 - x1 * y0
        for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1], strict=True)
    )


def oriented(points: list[Point], clockwise: bool) -> list[Point]:
    """In font coordinates (y up), clockwise has negative area."""
    return points if (area(points) < 0) == clockwise else points[::-1]


def to_font(points: list[Point]) -> list[Point]:
    scale = EM / GRID
    return [(x * scale, (GRID - y) * scale) for x, y in points]


def tree() -> list[list[Point]]:
    """Contours in font coordinates: solids clockwise, the ring's hole counter-clockwise."""
    trunk = cubic((11.4, 17.75), (11.4, 15.65), (13.1, 15.05), (12.5, 13.05))
    trunk += cubic((12.5, 13.05), (12.2, 12.05), (12.5, 11.15), (13.1, 10.25))[1:]
    solids = [
        circle(12, 12, 11),
        capsule((6.5, 17.75), (17.5, 17.75), 1.5),
        *stroke(trunk, 1.4),
        capsule((12.4, 12.8), (8, 11), 1.4),
        capsule((12.9, 11.3), (16.9, 9.9), 1.4),
        circle(7.1, 10.4, 1.9),
        circle(9.3, 7.6, 2.1),
        circle(12.6, 6.3, 2.3),
        circle(15.7, 7.8, 2.1),
        circle(17.1, 10.4, 1.7),
    ]
    hole = oriented(to_font(circle(12, 12, 9.5)), clockwise=False)
    return [oriented(to_font(c), clockwise=True) for c in solids] + [hole]


def build(out: Path) -> None:
    pen = TTGlyphPen(None)
    for contour in tree():
        pen.moveTo(contour[0])
        for point in contour[1:]:
            pen.lineTo(point)
        pen.closePath()
    glyph = pen.glyph()
    empty = TTGlyphPen(None).glyph()

    builder = FontBuilder(EM, isTTF=True)
    builder.setupGlyphOrder([".notdef", "tree"])
    builder.setupCharacterMap({CHARACTER: "tree"})
    builder.setupGlyf({".notdef": empty, "tree": glyph})
    builder.setupHorizontalMetrics({".notdef": (EM, 0), "tree": (EM, 0)})
    builder.setupHorizontalHeader(ascent=EM, descent=0)
    builder.setupNameTable({"familyName": "sundgren-icons", "styleName": "Regular"})
    builder.setupOS2(sTypoAscender=EM, sTypoDescender=0, usWinAscent=EM, usWinDescent=0)
    builder.setupPost()
    builder.updateHead(created=TIMESTAMP, modified=TIMESTAMP)
    builder.font.recalcTimestamp = False
    builder.font.flavor = "woff"
    builder.save(str(out))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: icon_font.py OUT.woff")
    build(Path(sys.argv[1]))
