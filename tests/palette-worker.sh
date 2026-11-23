#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf -- "$t"' EXIT
# A large warm-neutral field, a coherent green/turquoise region and a
# few saturated red pixels. The first region must not turn the palette beige.
{
    printf 'P3\n96 1\n255\n'
    printf '173 151 129\n%.0s' {1..65}
    printf '40 143 91\n%.0s' {1..15}
    printf '40 143 126\n%.0s' {1..14}
    printf '255 0 0\n%.0s' {1..2}
} > "$t/mixed.ppm"
printf 'P3\n1 1\n255\n86 66 155\n' > "$t/dark.ppm"
# Dark forest-like low-chroma image: must preserve its hue rather than collapsing to neutral.
{
    printf 'P3\n20 1\n255\n'
    printf '9 17 19\n%.0s' {1..14}
    printf '15 26 28\n%.0s' {1..6}
} > "$t/dark-forest.ppm"
printf 'P3\n3 1\n255\n0 0 0 100 101 100 255 255 255\n' > "$t/neutral.ppm"
printf 'P3\n4 4\n255\n' > "$t/black.ppm"
printf '5 5 6\n%.0s' {1..16} >> "$t/black.ppm"
printf 'P3\n4 4\n255\n' > "$t/white.ppm"
printf '250 250 250\n%.0s' {1..16} >> "$t/white.ppm"
# Split top bar: dark left, mid center, bright right (high variance / mixed top strip).
{
    printf 'P3\n12 4\n255\n'
    for _ in 1 2 3 4; do
        printf '10 12 18\n%.0s' {1..4}
        printf '95 105 120\n%.0s' {1..4}
        printf '242 244 248\n%.0s' {1..4}
    done
} > "$t/split.ppm"
cat > "$t/timed.xml" <<EOF
<background>
  <static>
    <duration>1795.0</duration>
    <file>$t/dark.ppm</file>
  </static>
</background>
EOF
out=$(gjs -m "$root/extension/palette-worker.js" \
    "file://$t/mixed.ppm" "file://$t/dark.ppm" "file://$t/mixed.ppm" \
    "file://$t/dark-forest.ppm" "file://$t/neutral.ppm" \
    "file://$t/black.ppm" "file://$t/white.ppm" "file://$t/split.ppm" \
    "file://$t/timed.xml" "color:#208060")
python - "$out" <<'PY'
import json, sys
light, dark, again, forest, neutral, black, white, split, timed, solid = json.loads(sys.argv[1])
assert 145 <= light["hue"] <= 175, light
assert 245 <= dark["hue"] <= 275, dark
assert light == again, (light, again)
assert 175 <= forest["hue"] <= 210 and forest["top"]["luma"] < 0.05, forest
assert neutral["hue"] is None, neutral
assert black["hue"] is None and black["top"]["luma"] < 0.01 and black["scene"]["luma"] < 0.01, black
assert white["hue"] is None and white["top"]["luma"] > 0.94 and white["scene"]["luma"] > 0.94, white
assert split["top"]["zones"][0] < 0.03 and split["top"]["zones"][2] > 0.85 and split["top"]["spread"] >= 0.80, split
assert len(split["top"]["bins"]) == 16 and split["top"]["bins"][0][0] < 0.03 and split["top"]["bins"][15][0] > 0.85, split
assert 245 <= timed["hue"] <= 275, timed
assert 150 <= solid["hue"] <= 170 and solid["top"]["spread"] == 0 and len(solid["top"]["bins"]) == 16, solid
print(f"PASS: OKLab dominant hues ({light['hue']:.1f}°, {dark['hue']:.1f}°, forest {forest['hue']:.1f}°), spatial luminance, XML & solid color")
PY
