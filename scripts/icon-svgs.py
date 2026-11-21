"""Writes the Retone icon and logo SVGs in assets/icon. Run it only to change the shapes or
colours; scripts/make-icons.sh turns the SVGs into AppIcon.icns and the menu-bar images."""
import math, sys

out = sys.argv[1]
C, R, W = 512, 268, 74          # loop centre, radius, stroke width
BG, BG_EDGE = "#f2ecbc", "#e2d9a6"   # lotusWhite3, a darker edge
BLUE, RED = "#4d699b", "#c84053"     # lotusBlue4, lotusRed


def pt(deg, r=R):
    a = math.radians(deg)
    return C + r * math.cos(a), C + r * math.sin(a)


def arc(a0, a1):
    (x0, y0), (x1, y1) = pt(a0), pt(a1)
    large = 1 if (a1 - a0) % 360 > 180 else 0
    return f"M{x0:.1f} {y0:.1f} A{R} {R} 0 {large} 1 {x1:.1f} {y1:.1f}"


def head(deg):
    """Arrowhead at angle deg, pointing clockwise along the circle."""
    a = math.radians(deg)
    tx, ty = -math.sin(a), math.cos(a)        # clockwise tangent (y down)
    nx, ny = math.cos(a), math.sin(a)         # outward normal
    bx, by = pt(deg)
    tip = (bx + tx * 92, by + ty * 92)
    s = 82
    p1 = (bx + nx * s, by + ny * s)
    p2 = (bx - nx * s, by - ny * s)
    return f"M{tip[0]:.1f} {tip[1]:.1f} L{p1[0]:.1f} {p1[1]:.1f} L{p2[0]:.1f} {p2[1]:.1f} Z"


def loop(blue, red, line_alpha, lines=True, w=W):
    # Blue: from the lower left over the top to the upper right. Red: from the lower right under
    # the bottom to the left. The gaps hold the arrowheads.
    return f"""
  <path d="{arc(168, 318)}" fill="none" stroke="{blue}" stroke-width="{w}" stroke-linecap="round"/>
  <path d="{head(318)}" fill="{blue}" stroke="{blue}" stroke-width="18" stroke-linejoin="round"/>
  <path d="{arc(348, 138)}" fill="none" stroke="{red}" stroke-width="{w}" stroke-linecap="round"/>
  <path d="{head(138)}" fill="{red}" stroke="{red}" stroke-width="18" stroke-linejoin="round"/>
{'' if not lines else f'''  <g fill="{blue}" opacity="{line_alpha}">
    <rect x="352" y="452" width="118" height="40" rx="20"/>
    <rect x="352" y="532" width="96" height="40" rx="20"/>
    <rect x="554" y="452" width="118" height="40" rx="20"/>
    <rect x="576" y="532" width="96" height="40" rx="20"/>
  </g>'''}
  <g fill="{red}">
    <rect x="488" y="372" width="48" height="280" rx="24"/>
    <rect x="436" y="360" width="152" height="44" rx="22"/>
    <rect x="436" y="620" width="152" height="44" rx="22"/>
  </g>"""


icon = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#f7f2cc"/><stop offset="1" stop-color="{BG}"/>
    </linearGradient>
  </defs>
  <rect x="100" y="100" width="824" height="824" rx="186" fill="url(#bg)" stroke="{BG_EDGE}" stroke-width="6"/>{loop(BLUE, RED, 0.7)}
</svg>
"""
# 16 px: no text lines, thicker strokes, so the loop and cursor survive the downscale.
small = icon.replace(loop(BLUE, RED, 0.7), loop(BLUE, RED, 0.7, lines=False, w=104).replace('width="48"', 'width="72"').replace('x="488"', 'x="476"'))
# Template image: black with alpha only; macOS tints it for light and dark menu bars.
template = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="150 150 724 724">{loop("#000", "#000", 0.55, lines=False, w=96).replace('width="48"', 'width="72"').replace('x="488"', 'x="476"')}
</svg>
"""


def logo(ink):
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 830 300">
  <g transform="translate(-20 -20) scale(0.332)">{icon.split('>', 1)[1].rsplit('</svg>', 1)[0]}</g>
  <text x="330" y="196" font-family="-apple-system, 'SF Pro Display', 'Helvetica Neue', Arial, sans-serif" font-size="150" font-weight="700" letter-spacing="-3" fill="{ink}">Retone</text>
</svg>
"""


for name, body in [("icon.svg", icon), ("icon-16.svg", small), ("menubar-template.svg", template),
                   ("logo-light.svg", logo("#1f1f28")), ("logo-dark.svg", logo("#f2ecbc"))]:
    open(f"{out}/{name}", "w").write(body)
