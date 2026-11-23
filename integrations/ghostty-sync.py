#!/usr/bin/python
"""Local Ghostty bridge for Duvar's saved palette-colors GSettings key."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

from gi.repository import Gio, GLib


SCHEMA = "org.gnome.shell.extensions.duvar"
UNIT = "app-com.mitchellh.ghostty.service"
KEY = "palette-colors"
FIELDS = ("surface", "text", "accent")
MODES = ("light", "dark")
FALLBACK = {
    "light": {"surface": "#ffffff", "text": "#191919", "accent": "#454545"},
    "dark": {"surface": "#202126", "text": "#f5f5f5", "accent": "#bfbfbf"},
}


def themes(raw, enabled):
    pair = json.loads(raw) if enabled else FALLBACK
    for mode in MODES:
        colors = pair[mode]
        for key in FIELDS:
            if not isinstance(colors[key], str) or not re.fullmatch(r"#[0-9a-fA-F]{6}", colors[key]):
                raise ValueError(f"invalid {mode} {key}")
    return {
        mode: (
            "# Managed by Duvar Ghostty integration\n"
            f"background = {colors['surface']}\nforeground = {colors['text']}\n"
            f"cursor-color = {colors['accent']}\n"
            f"selection-background = {colors.get('accentBg', colors['accent'])}\n"
            f"selection-foreground = {colors.get('accentFg', '#f5f5f5' if mode == 'light' else '#191919')}\n"
        )
        for mode in MODES
        for colors in (pair[mode],)
    }


def write_theme(path, content):
    if path.is_symlink():
        raise ValueError(f"refusing to replace symlink: {path}")
    if path.exists() and path.read_text() == content:
        return False
    with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as tmp:
        staged = Path(tmp.name)
        try:
            tmp.write(content)
            tmp.close()
            staged.replace(path)
        finally:
            staged.unlink(missing_ok=True)
    return True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--once", action="store_true", help="write themes without reloading Ghostty")
    args = parser.parse_args()
    schema_dir = Path(GLib.get_user_data_dir()) / "gnome-shell/extensions/duvar@mo4tal.local/schemas"
    source = Gio.SettingsSchemaSource.new_from_directory(str(schema_dir), None, False)
    settings = Gio.Settings.new_full(source.lookup(SCHEMA, False), None, None)
    directory = Path(GLib.get_user_config_dir()) / "ghostty/themes"
    directory.mkdir(parents=True, exist_ok=True)

    def sync():
        wanted = themes(settings.get_string(KEY), settings.get_boolean("palette-enabled"))
        changed = False
        for mode, content in wanted.items():
            changed |= write_theme(directory / f"Duvar-{mode}", content)
        return changed

    if args.once:
        sync()
        return

    def reload_ghostty():
        base = ["/usr/bin/systemctl", "--user"]
        active = subprocess.run(base + ["is-active", "--quiet", UNIT], timeout=5).returncode == 0
        if active:
            subprocess.run(base + ["reload", UNIT], timeout=12, check=True)

    def updated(*_args):
        nonlocal pending
        try:
            # Also reload after a previous reload failure, even if the files match now.
            if sync() or pending:
                pending = True
                reload_ghostty()
                pending = False
        except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError) as exc:
            print(f"Duvar Ghostty: {exc}", flush=True)

    # At login Ghostty may already be running; if it is not, it will read the themes on launch.
    pending = True
    settings.connect(f"changed::{KEY}", updated)
    settings.connect("changed::palette-enabled", updated)
    updated()
    GLib.MainLoop().run()


if __name__ == "__main__":
    main()
