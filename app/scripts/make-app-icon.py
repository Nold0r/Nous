"""Build the app icon from the logo, per platform style.

    python scripts/make-app-icon.py <style> src/assets/brand/nous-logo.svg <out.svg>

Styles:
  mac            dark rounded-square tile, mark centred with a soft shadow and
                 glossy gradients (like the Obsidian icon) -> icon.icns
  windows        Windows 11 / Fluent: no backplate, the mark itself fills the
                 canvas, light from the top-left, soft shadow below -> icon.ico 48-256
  windows-small  same without shadow and with the mark at full size, for the
                 crisp 16-32 px entries of icon.ico

Render to 1024 PNG and pack the icons with scripts/pack-icons.py
(see docs/Дизайн-система/Логотип.md).
"""
import re
import sys

style, src, out = sys.argv[1], sys.argv[2], sys.argv[3]
assert style in ('mac', 'windows', 'windows-small'), style
svg = open(src, encoding='utf8').read()
layers = dict(re.findall(r'class="nous-logo__(\w+)"[^>]*?d="([^"]+)"', svg))

SIZE = 512
BBOX = (33, 45, 991, 996)  # opaque bounds of the logo in its 1024 viewBox
# glyph width on the 512 canvas and shadow (offset, blur, opacity)
GLYPH, SHADOW = {
    'mac': (384, (4, 6.5, 0.55)),  # inside the tile, like Obsidian's mark
    'windows': (468, (8, 9, 0.4)),  # Fluent: the object is the icon, room left for the shadow
    'windows-small': (504, None),  # tiny sizes: as big and crisp as possible
}[style]

scale = GLYPH / (BBOX[2] - BBOX[0])
cx, cy = (BBOX[0] + BBOX[2]) / 2, (BBOX[1] + BBOX[3]) / 2
# Windows icons have no backplate, so a dark rim keeps the cream mark readable
# on the light Start menu / taskbar. Width is in the logo's 1024 units, half of
# it shows outside the shapes; the small art gets a thicker one to survive 16 px.
RIM = {'mac': 0, 'windows': 22, 'windows-small': 44}[style]

# mac: a touch low on the tile; windows: nudged up so mark + shadow stay centred
shift = 4 if style == 'mac' else -(SHADOW[0] / 2 if SHADOW else 0)
tx, ty = SIZE / 2 - cx * scale, SIZE / 2 - cy * scale + shift
transform = f'translate({tx:.2f} {ty:.2f}) scale({scale:.5f})'

tile = (
    f'  <rect width="{SIZE}" height="{SIZE}" rx="100" fill="url(#bg)"/>\n'
    if style == 'mac'
    else ''
)
shadow = (
    f'''  <!-- soft shadow under the mark -->
  <g filter="url(#shadow)" opacity="{SHADOW[2]}">
    <g transform="translate(0 {SHADOW[0]}) {transform}" fill="#000">
      <use href="#cream"/>
      <use href="#gold"/>
      <use href="#crimson"/>
    </g>
  </g>
'''
    if SHADOW
    else ''
)
shadow_filter = (
    f'''    <filter id="shadow" x="0" y="0" width="{SIZE}" height="{SIZE}" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
      <feGaussianBlur stdDeviation="{SHADOW[1]}"/>
    </filter>
'''
    if SHADOW
    else ''
)

rim = (
    f'''  <!-- dark rim for contrast on light backgrounds -->
  <g transform="{transform}" fill="#2b1a20" stroke="#2b1a20" stroke-width="{RIM}" stroke-linejoin="round" opacity=".85">
    <use href="#cream"/>
    <use href="#gold"/>
    <use href="#crimson"/>
  </g>
'''
    if RIM
    else ''
)

icon = f'''<svg xmlns="http://www.w3.org/2000/svg" width="{SIZE}" height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}" fill="none">
  <defs>
    <path id="cream" fill-rule="evenodd" d="{layers['cream']}"/>
    <path id="gold" fill-rule="evenodd" d="{layers['gold']}"/>
    <path id="crimson" fill-rule="evenodd" d="{layers['crimson']}"/>

    <!-- mac tile: near-black with a faint warm glow from the top -->
    <radialGradient id="bg" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="matrix(0 420 -420 0 256 40)">
      <stop stop-color="#33232b"/>
      <stop offset="1" stop-color="#1c1418"/>
    </radialGradient>

    <!-- depth, in the logo's 1024 space: light from the top-left, shade bottom-right -->
    <radialGradient id="creamLight" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="matrix(560 380 -420 620 300 230)">
      <stop stop-color="#fff" stop-opacity=".55"/>
      <stop offset=".55" stop-color="#fff" stop-opacity=".12"/>
      <stop offset="1" stop-color="#fff" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="creamShade" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="matrix(-420 -300 360 -520 760 900)">
      <stop stop-color="#3a1e28" stop-opacity=".45"/>
      <stop offset="1" stop-color="#3a1e28" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="goldLight" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="matrix(120 140 -160 130 440 300)">
      <stop stop-color="#fff" stop-opacity=".55"/>
      <stop offset="1" stop-color="#fff" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="crimsonLight" x1="512" y1="300" x2="560" y2="420" gradientUnits="userSpaceOnUse">
      <stop stop-color="#fff" stop-opacity=".22"/>
      <stop offset="1" stop-color="#fff" stop-opacity="0"/>
    </linearGradient>
{shadow_filter}  </defs>

{tile}{shadow}
{rim}  <g transform="{transform}">
    <use href="#cream" fill="#e9e4da"/>
    <use href="#cream" fill="url(#creamShade)"/>
    <use href="#cream" fill="url(#creamLight)"/>
    <use href="#gold" fill="#d8b875"/>
    <use href="#gold" fill="url(#goldLight)"/>
    <use href="#crimson" fill="#b53e58"/>
    <use href="#crimson" fill="url(#crimsonLight)"/>
  </g>
</svg>
'''
open(out, 'w', encoding='utf8').write(icon)
print(f'{style}: {out} ({len(icon)} bytes)')
