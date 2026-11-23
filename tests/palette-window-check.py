"""Verify focused and backdrop CSD outlines in a controlled Wayland overlap."""
import json
from pathlib import Path
import re
import subprocess
import sys

cache, css_path = map(Path, sys.argv[1:])
light, dark = css_path.read_text().split('@media (prefers-color-scheme: dark)')
ids = None
for mode, css in [('light', light), ('dark', dark)]:
    active, inactive = [bytes.fromhex(c) for c in re.findall(
        r'window\.csd(?::backdrop)? \{ outline-color: #([0-9a-f]{6}); \}', css)]
    assert active != inactive
    for index, kind in enumerate(('4', 'adw')):
        tag = f'overlap-{mode}-{kind}'
        gtk, adw = windows = json.loads((cache / f'{tag}.geometry').read_text())
        current_ids = [w['id'] for w in windows]
        ids = ids or current_ids
        assert ids == current_ids and len(set(ids)) == 2, (tag, windows)
        assert [w['focused'] for w in windows] == [index == 0, index == 1], (tag, windows)
        assert gtk['x'] < adw['x'] < gtk['x'] + gtk['width'] < adw['x'] + adw['width'], (tag, windows)
        y = max(w['y'] for w in windows) + 55
        assert y + 60 < min(w['y'] + w['height'] for w in windows), (tag, windows)
        for j, window in enumerate(windows):
            focused = j == index
            right = (j == 0) if focused else (j == 1)
            # The 1 px outline is inside the frame. Sampling just that
            # column excludes wallpaper and the other window's surface.
            x = window['x'] + (window['width'] - 1 if right else 0)
            data = subprocess.check_output(['magick', str(cache / f'{tag}.png'),
                '-crop', f'1x60+{x}+{y}', '+repage', '-depth', '8', 'rgb:-'])
            pixels = list(zip(data[::3], data[1::3], data[2::3]))
            color = active if focused else inactive
            other = inactive if focused else active
            def distance(pixel, target):
                return max(abs(a - b) for a, b in zip(pixel, target))
            # The compositor blends the 1 px CSD outline with backdrop shadow and
            # wallpaper, shifting it by up to ~14 levels over dark vivid wallpapers.
            # Each pixel must still strictly favor its own focused/backdrop role.
            matches = sum(distance(pixel, color) <= 16 and distance(pixel, color) < distance(pixel, other)
                          for pixel in pixels)
            assert matches >= 50, (tag, j, x, y, color.hex(), matches,
                [bytes(pixel).hex() for pixel in pixels[::10]])
        print(f'PASS: {tag}: active #{active.hex()}, backdrop #{inactive.hex()}, same windows')
