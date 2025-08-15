"""generate_gosper.py — generate a Gosper (flowsnake) curve as an SVG.

Usage:
  python generate_gosper.py [-n ORDER] [-o OUT.svg] [--size 1024] [--stroke 1.6]
Examples:
  python generate_gosper.py
  python generate_gosper.py -n 4 -o gosper.svg --size 1600 --stroke 2.0
"""

import argparse
import numpy as np

RULE_A = "A-B--B+A++AA+B-"
RULE_B = "+A-BB--B-A++A+B"
ANGLE_DEG = 60.0


def lsystem_instructions(order: int) -> str:
    """Generate L-system instructions for the Gosper curve.

    Parameters:
        order (int): The recursion order

    Returns:
        str: The L-system instructions
    """
    s = "A"
    for _ in range(order):
        out = []
        for ch in s:
            if ch == "A":
                out.append(RULE_A)
            elif ch == "B":
                out.append(RULE_B)
            else:
                out.append(ch)
        s = "".join(out)
    return s


def turtle_path(instr: str, step: float = 10.0) -> list[tuple[float, float]]:
    """Generate the turtle path from L-system instructions.

    Parameters:
        instr (str): The L-system instructions
        step (float): The step size for the turtle

    Returns:
        list[tuple[float, float]]: The list of points visited by the turtle
    """
    x, y = 0.0, 0.0
    angle = 0.0
    pts = [(x, y)]
    turn = np.radians(ANGLE_DEG)
    for ch in instr:
        if ch in ("A", "B"):
            x += step * np.cos(angle)
            y += step * np.sin(angle)
            pts.append((x, y))
        elif ch == "+":
            angle += turn
        elif ch == "-":
            angle -= turn
    return pts


def fit_to_viewbox(points: list[tuple[float, float]], size: int, margin: float = 32.0):
    """Fit the points to the SVG viewbox.

    Parameters:
        points (list[tuple[float, float]]): The list of points to fit
        size (int): The size of the SVG canvas
        margin (float): The margin to apply (default 32.0)

    Returns:
        list[tuple[float, float]]: The transformed list of points
    """
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    xmin, xmax = min(xs), max(xs)
    ymin, ymax = min(ys), max(ys)
    w = max(1e-9, xmax - xmin)
    h = max(1e-9, ymax - ymin)
    scale = (size - 2 * margin) / max(w, h)
    cx = (xmin + xmax) / 2.0
    cy = (ymin + ymax) / 2.0
    out = []
    for x, y in points:
        sx = (x - cx) * scale + size / 2.0
        sy = (cy - y) * scale + size / 2.0  # flip Y for SVG
        out.append((sx, sy))
    return out


def svg_path_from_points(points: list[tuple[float, float]]) -> str:
    """Generate an SVG path string from a list of points.

    Parameters:
        points (list[tuple[float, float]]): The list of points to convert

    Returns:
        str: The SVG path string
    """

    def fmt(v: float) -> str:
        return f"{v:.3f}".rstrip("0").rstrip(".")

    if not points:
        return ""
    cmds = [f"M {fmt(points[0][0])} {fmt(points[0][1])}"]
    for x, y in points[1:]:
        cmds.append(f"L {fmt(x)} {fmt(y)}")
    return " ".join(cmds)


def make_svg(path_d: str, size: int, stroke: float) -> str:
    """Generate an SVG string from the path data.

    Parameters:
        path_d (str): The SVG path data
        size (int): The size of the SVG canvas
        stroke (float): The stroke width for the SVG path

    Returns:
        str: The SVG string
    """
    bg = "#101215"
    fg = "#FFC857"
    text_color = "#FFFFFF"
    text_size = size * 0.05  # 5% of canvas size

    return f"""<?xml version="1.0" encoding="UTF-8"?>
<svg width="{size}" height="{size}" viewBox="0 0 {size} {size}"
     xmlns="http://www.w3.org/2000/svg" version="1.1">
  <defs>
    <style>
      .bg {{ fill: {bg}; }}
      .curve {{ fill: none; stroke: {fg}; stroke-width: {stroke};
                stroke-linecap: round; stroke-linejoin: round; }}
      .label {{ fill: {text_color}; font-family: sans-serif; font-weight: bold;
                font-size: {text_size}px; }}
    </style>
  </defs>
  <rect class="bg" x="0" y="0" width="{size}" height="{size}" />
  <path class="curve" d="{path_d}" />
  <text class="label" x="{size * 0.02}" y="{size * 0.96}">gOASper</text>
</svg>"""


def main():
    """Generate a Gosper curve SVG with 'gOASper' label."""
    ap = argparse.ArgumentParser(
        description="Generate a Gosper curve SVG with 'gOASper' label."
    )
    ap.add_argument(
        "-n",
        "--order",
        type=int,
        default=4,
        help="recursion order (2-5 sensible; default 4)",
    )
    ap.add_argument("-o", "--output", default="gosper.svg", help="output SVG path")
    ap.add_argument(
        "--size", type=int, default=1024, help="canvas size in px (default 1024)"
    )
    ap.add_argument(
        "--stroke", type=float, default=1.6, help="stroke width in px (default 1.6)"
    )
    args = ap.parse_args()

    if args.order < 0 or args.order > 6:
        raise SystemExit("Choose an order between 0 and 6 (try 3-5).")

    instr = lsystem_instructions(args.order)
    pts = turtle_path(instr, step=10.0)
    pts = fit_to_viewbox(pts, size=args.size, margin=32.0)
    path_d = svg_path_from_points(pts)

    svg = make_svg(path_d, size=args.size, stroke=args.stroke)
    with open(args.output, "w", encoding="utf-8") as f:
        f.write(svg)
    print(f"Wrote {args.output} (order={args.order}, points={len(pts)})")


if __name__ == "__main__":
    main()
