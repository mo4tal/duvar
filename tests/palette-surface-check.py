"""Check real BMS actor colors and the painted popup backgrounds."""
from collections import Counter
from pathlib import Path
import re
import subprocess
import sys


def luminance(rgb):
    values = [v / 255 for v in rgb]
    linear = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4 for v in values]
    return sum(v * w for v, w in zip(linear, (.2126, .7152, .0722)))


def contrast(a, b):
    lo, hi = sorted((luminance(a), luminance(b)))
    return (hi + .05) / (lo + .05)


def rgb(value):
    return tuple(bytes.fromhex(value[1:7]))


log, cache, themes = map(Path, sys.argv[1:])
lines = log.read_text()
for mode in ('dark', 'light'):
    css = (themes / f'Duvar-Live-{mode}/gnome-shell/duvar-blur-accent.css').read_text()
    for surface in ('quick', 'date', 'weather', 'notification'):
        tag = f'surface-{mode}-{surface}'
        roles = {'quick': 'quick-settings', 'date': 'datemenu-popover',
                 'weather': 'popup-menu-content', 'notification': 'notification-banner'}
        rows = re.findall(rf'Duvar surface {tag} (#[a-f0-9]{{8}}) (#[a-f0-9]{{8}}) ([\d,-]+) (.*)', lines)
        bg, fg, bounds, lineage = next(row for row in rows if
            {roles[surface], 'bms-popup-blurred'} <= set(row[3].split(' < ')[0].split()))
        bms = re.search(r'\bbms-popup-surface-(?:dark|light|transparent)\b', lineage)[0]
        target = 'message' if surface == 'notification' else 'popup-menu-content'
        selector = rf'\.{bms} \.{target}\.bms-popup-blurred(?![:\w-])[^\n]*?\{{'
        matches_css = re.findall(selector + r' background-color: rgba\(([\d, ]+),\s*(0\.\d+)\)', css)
        assert matches_css, (tag, bms, target)
        channels, opacity = matches_css[-1]
        assert 0.50 <= float(opacity) <= 0.88, (tag, opacity)
        expected = tuple(map(int, channels.split(',')))
        assert rgb(bg) == expected and abs(int(bg[-2:], 16) - round(float(opacity) * 255)) <= 1, (tag, bg, expected)
        x, y, w, h = map(int, bounds.split(','))
        data = subprocess.check_output(['magick', str(cache / f'{tag}.png'),
            '-crop', f'{w}x{h}+{x}+{y}', '+repage', '-depth', '8', 'rgb:-'])
        counts = Counter(zip(data[::3], data[1::3], data[2::3]))
        delta = 255 - int(bg[-2:], 16) + 2
        def nearby(p, color, tolerance):
            return max(abs(a - b) for a, b in zip(p, color)) <= tolerance
        matches = [(p, count) for p, count in counts.items() if nearby(p, expected, delta)]
        assert sum(count for _, count in matches) >= min(1000, w * h // 10), tag
        painted = max(matches, key=lambda pair: pair[1])[0]
        colors = [fg]
        if surface == 'weather':
            colors.extend(row[1] for row in rows if row[3].startswith('weatherpanel-status-label '))
            assert len(colors) > 1, 'WeatherPanel status label not mapped'
        ratio = min(contrast(rgb(color), painted) for color in colors)
        assert ratio >= 4.5, (tag, painted, colors, ratio)
        print(f'PASS: {tag} {bg}, painted RGB {painted}, text contrast {ratio:.2f}:1')
        def composite(rgba_hex, under):
            a = int(rgba_hex[-2:], 16) / 255
            return tuple(round(fg_c * a + bg_c * (1 - a)) for fg_c, bg_c in zip(rgb(rgba_hex), under))
        if surface == 'date':
            d_roles = re.findall(rf'Duvar role {tag} (\S+) (#[a-f0-9]{{8}}) (#[a-f0-9]{{8}}) (\S+) (\S+)', lines)
            month = next(item for item in d_roles if item[0] == 'calendar-month-label')
            today = next(item for item in d_roles if item[0] == 'datemenu-today-button')
            placeholder = next((item for item in d_roles if item[0] == 'message-list-placeholder'), None)
            assert contrast(rgb(month[2]), painted) >= 4.5, (tag, 'month', month, painted)
            assert contrast(rgb(today[2]), painted) >= 4.5, (tag, 'today', today, painted)
            if placeholder:
                assert contrast(rgb(placeholder[2]), painted) >= 4.5, (tag, 'placeholder', placeholder, painted)
            print(f'PASS: {tag} calendar month/today/placeholder ink legibility verified')
        if surface == 'quick':
            roles = re.findall(rf'Duvar role {tag} (\S+) (#[a-f0-9]{{8}}) (#[a-f0-9]{{8}}) (\S+) (\S+)', lines)
            def first(name, predicate=lambda _: True):
                return next(item for item in roles if item[0] == name and predicate(item))
            username = first('avatar-username-label')
            realname = first('avatar-realname-label')
            assert username[2] != realname[2], (tag, username, realname)
            assert contrast(rgb(username[2]), painted) >= 4.5, (tag, username, painted)
            active = first('active', lambda item: item[1][-2:] == 'ff')
            passive = first('passive', lambda item: item[1] != active[1] and 0x60 <= int(item[1][-2:], 16) < 0xff)
            assert contrast(rgb(active[1]), rgb(active[2])) >= 4.5, (tag, active)
            assert contrast(composite(passive[1], painted), rgb(passive[2])) >= 4.5, (tag, passive, painted)
            slider = first('slider')
            assert slider[3].startswith('#') and slider[4].startswith('#') and slider[3] != slider[4], (tag, slider)
            print(f'PASS: {tag} avatar secondary text, active solid + passive glass toggles and slider colors')
    panel, bg, active = re.search(
        rf'Duvar panel surface-{mode}-quick (\S+) .* active bg=(#[a-f0-9]{{8}}) (#[a-f0-9]{{8}})', lines).groups()
    assert bg == '#00000000', (mode, bg, active, panel)
    rule = rf'(?:#{re.escape("panel." + panel)}|#panel) \.panel-button\.duvar-system-menu:active, [^\n]*?'
    r, g, b, a = re.search(rule + r'box-shadow: inset 0 0 0 100px rgba\((\d+),(\d+),(\d+),([0-9.]+)\) !important;', css).groups()
    expected_rgb = f'#{int(r):02x}{int(g):02x}{int(b):02x}'
    expected_a = round(float(a) * 255)
    actual_a = int(active[7:9], 16)
    assert active[:7] == expected_rgb and abs(actual_a - expected_a) <= 1 and 0 < actual_a < 0xff, (
        mode, active, expected_rgb, expected_a)
    print(f'PASS: {mode} active top-panel translucent inset pill {active} (bg={bg}, {panel})')
