"""Trace the flat 3-colour Nous logo PNG into an SVG (one potrace pass per colour).

    pip install potracer numpy pillow
    python scripts/trace-logo.py src/assets/brand/nous-logo.png src/assets/brand/nous-logo.svg

The app icon is generated from the PNG instead: npx tauri icon src/assets/brand/nous-logo.png
"""
import sys

import numpy as np
import potrace
from PIL import Image

src, out = sys.argv[1], sys.argv[2]
im = np.asarray(Image.open(src).convert('RGBA')).astype(int)
rgb, alpha = im[..., :3], im[..., 3]

# bottom to top: the crimson needle overlaps the rest
colors = [('cream', (233, 228, 218)), ('gold', (216, 184, 117)), ('crimson', (181, 62, 88))]
palette = np.array([c for _, c in colors])
dist = ((rgb[:, :, None, :] - palette[None, None]) ** 2).sum(-1)
nearest = dist.argmin(-1)
opaque = alpha > 110

paths = []
for i, (name, (r, g, b)) in enumerate(colors):
    mask = opaque & (nearest == i)
    if name != 'crimson':
        # grow lower layers 2px under the needle so no hairline seam shows
        under = opaque & (nearest == 2)
        grown = mask.copy()
        for dy in (-2, -1, 0, 1, 2):
            for dx in (-2, -1, 0, 1, 2):
                grown |= np.roll(np.roll(mask, dy, 0), dx, 1)
        mask = mask | (grown & under)
    # potracer treats truthy pixels as white (background), so pass the inverse
    bmp = potrace.Bitmap(~mask)
    plist = bmp.trace(turdsize=6, turnpolicy=potrace.POTRACE_TURNPOLICY_MINORITY, alphamax=1.0, opticurve=True, opttolerance=0.2)
    d = []
    for curve in plist:
        sp = curve.start_point
        d.append(f'M{sp.x:.1f} {sp.y:.1f}')
        for seg in curve.segments:
            if seg.is_corner:
                d.append(f'L{seg.c.x:.1f} {seg.c.y:.1f}L{seg.end_point.x:.1f} {seg.end_point.y:.1f}')
            else:
                d.append(
                    f'C{seg.c1.x:.1f} {seg.c1.y:.1f} {seg.c2.x:.1f} {seg.c2.y:.1f} {seg.end_point.x:.1f} {seg.end_point.y:.1f}'
                )
        d.append('Z')
    paths.append(f'  <path class="nous-logo__{name}" fill="#{r:02x}{g:02x}{b:02x}" fill-rule="evenodd" d="{"".join(d)}"/>')
    print(name, len(plist), 'curves', file=sys.stderr)

svg = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">\n' + '\n'.join(paths) + '\n</svg>\n'
)
open(out, 'w', encoding='utf8').write(svg)
print(len(svg), 'bytes', file=sys.stderr)
