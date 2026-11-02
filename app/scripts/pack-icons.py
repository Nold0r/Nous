"""Pack the platform app icons into src-tauri/icons.

    python scripts/pack-icons.py

Expects 1024x1024 transparent PNGs rendered from scripts/make-app-icon.py:
  src/assets/brand/nous-app-icon-windows.png        Windows, 40-256 px
  src/assets/brand/nous-app-icon-windows-small.png  Windows, 16-32 px (no shadow)
  src/assets/brand/nous-app-icon-mac.png            macOS tile

- `tauri icon` on the Windows art -> every PNG / Square*Logo / ico
- `tauri icon` on the mac art into a temp dir -> only icon.icns is kept
- icon.ico is then rebuilt with hand-picked art per size
"""
import shutil
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

root = Path(__file__).resolve().parent.parent
brand = root / 'src' / 'assets' / 'brand'
icons = root / 'src-tauri' / 'icons'
win, win_small, mac = (brand / f'nous-app-icon-{s}.png' for s in ('windows', 'windows-small', 'mac'))


def tauri_icon(src: Path, out: Path) -> None:
    subprocess.run(f'npx tauri icon "{src}" -o "{out}"', cwd=root, shell=True, check=True, capture_output=True)


tauri_icon(win, icons)
with tempfile.TemporaryDirectory() as tmp:
    tauri_icon(mac, Path(tmp))
    shutil.copy(Path(tmp) / 'icon.icns', icons / 'icon.icns')
for mobile in ('android', 'ios'):
    shutil.rmtree(icons / mobile, ignore_errors=True)

# Windows .ico: small sizes from the shadowless art so they stay crisp
SMALL, LARGE = (16, 20, 24, 32), (40, 48, 64, 96, 128, 256)
big, small = Image.open(win).convert('RGBA'), Image.open(win_small).convert('RGBA')
frames = [small.resize((s, s), Image.LANCZOS) for s in SMALL] + [big.resize((s, s), Image.LANCZOS) for s in LARGE]
frames[-1].save(icons / 'icon.ico', format='ICO', sizes=[f.size for f in frames], append_images=frames[:-1])

print('icons written to', icons)
print('ico sizes:', sorted(Image.open(icons / 'icon.ico').ico.sizes()))
