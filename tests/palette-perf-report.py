"""Summarize isolated compositor paint gaps; these are not presentation timestamps."""
import json
import statistics
import sys
from pathlib import Path

for phase in ('idle', 'mode-dark', 'mode-default', 'wallpaper-no-pulse', 'wallpaper-pulse'):
    data = json.loads((Path(sys.argv[1]) / f'perf-{phase}.json').read_text())
    frames, writes, themes, contrast, workers = (
        data[k] for k in ('frames', 'writes', 'themes', 'contrast', 'workers'))
    assert len(frames) >= 30, (phase, len(frames))
    if phase.startswith('mode-'):
        assert themes and not contrast and not workers, (phase, data)
        assert [w[2] for w in writes] in ([], ['duvar-blur-accent.css']), (phase, writes)
        if writes:  # The stable custom path is reparsed on the replacement theme.
            operations = [(method, label) for method, label, *_ in data['operations']]
            assert operations.count(('load_stylesheet', 'duvar-blur-accent.css')) == len(themes), (phase, operations)
            assert ('unload_stylesheet', 'duvar-blur-accent.css') not in operations, (phase, operations)
    if phase.startswith('wallpaper-'):
        assert writes and themes and len(workers) == 1, (phase, data)
    assert [item[1] for item in contrast] == ([True, False] if phase == 'wallpaper-pulse' else [])
    gaps = sorted(((end - start, start, end) for start, end in zip(frames, frames[1:])), reverse=True)
    max_gap, start, end = gaps[0]
    p95 = gaps[len(gaps) // 20][0]
    print(f'{phase}: {len(frames)} paints; gap median/p95/max '
          f'{statistics.median(g[0] for g in gaps):.1f}/{p95:.1f}/{max_gap:.1f} ms '
          f'(max at {start:.0f}–{end:.0f} ms)')
    print(f'  CSS writes: {len(writes)}, total {sum(w[1] for w in writes):.2f} ms; '
          f'St.Theme construct→install: {[(round(t, 1), round(d, 2)) for t, d in themes]}; '
          f'worker start: {[round(t, 1) for t in workers]}; '
          f'high contrast: {[(round(t, 1), value) for t, value in contrast]}')
    for method, label, t, duration in data.get('operations', []):
        if duration >= 1 or label == 'duvar-blur-accent.css':
            print(f'  {method}({label}): {duration:.2f} ms at {t:.1f} ms')
