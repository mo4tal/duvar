// Private compositor only. Force continuous paints so idle time is not counted as a missed frame.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

export function samplePerformance(base, name) {
    if (!/^[a-z-]+$/.test(name)) throw new Error('Invalid sample name');
    const now = GLib.get_monotonic_time;
    const start = now();
    const ms = t => (t - start) / 1000;
    const frames = [], writes = [], themes = [], contrast = [], workers = [];
    const themeStart = new WeakMap();
    const a11y = new Gio.Settings({schema_id: 'org.gnome.desktop.a11y.interface'});
    a11y.get_boolean('high-contrast');
    const contrastId = a11y.connect('changed::high-contrast', () =>
        contrast.push([ms(now()), a11y.get_boolean('high-contrast')]));
    const originalWrite = GLib.file_set_contents;
    const originalWriteFull = GLib.file_set_contents_full;
    const originalTheme = St.Theme;
    const originalSetTheme = St.ThemeContext.prototype.set_theme;
    const originalSpawn = Gio.Subprocess.new;
    const operations = [];
    const hooks = [
        [St.Theme.prototype, 'load_stylesheet'],
        [St.Theme.prototype, 'unload_stylesheet'],
        [Gio.Settings.prototype, 'set_string'],
    ].map(([prototype, method]) => {
        const original = prototype[method];
        prototype[method] = function (...args) {
            const label = method === 'set_string' ? args[0] : args[0].get_basename();
            const t = now();
            try { return original.apply(this, args); }
            finally { operations.push([method, label, ms(t), ms(now()) - ms(t)]); }
        };
        return [prototype, method, original];
    });
    GLib.file_set_contents = function (path, bytes) {
        if (!path.endsWith('.css')) return originalWrite(path, bytes);
        const t = now();
        try { return originalWrite(path, bytes); }
        finally { writes.push([ms(t), (now() - t) / 1000, path.split('/').at(-1)]); }
    };
    if (typeof originalWriteFull === 'function') {
        GLib.file_set_contents_full = function (path, bytes, flags, mode) {
            if (!path.endsWith('.css')) return originalWriteFull(path, bytes, flags, mode);
            const t = now();
            try { return originalWriteFull(path, bytes, flags, mode); }
            finally { writes.push([ms(t), (now() - t) / 1000, path.split('/').at(-1)]); }
        };
    }
    St.Theme = new Proxy(originalTheme, {
        construct(target, args) {
            const t = now();
            const theme = Reflect.construct(target, args);
            themeStart.set(theme, t);
            return theme;
        },
    });
    St.ThemeContext.prototype.set_theme = function (theme) {
        const t = themeStart.get(theme) ?? now();
        try { return originalSetTheme.call(this, theme); }
        finally { themes.push([ms(t), (now() - t) / 1000]); }
    };
    Gio.Subprocess.new = function (argv, flags) {
        if (argv.some(arg => arg.endsWith('/palette-worker.js'))) workers.push(ms(now()));
        return originalSpawn.call(this, argv, flags);
    };
    const stage = global.stage;
    const paintId = stage.connect('after-paint', () => {
        frames.push(ms(now()));
        stage.queue_redraw();
    });
    stage.queue_redraw();
    GLib.timeout_add(GLib.PRIORITY_DEFAULT, 2200, () => {
        stage.disconnect(paintId);
        a11y.disconnect(contrastId);
        GLib.file_set_contents = originalWrite;
        if (typeof originalWriteFull === 'function') GLib.file_set_contents_full = originalWriteFull;
        St.Theme = originalTheme;
        St.ThemeContext.prototype.set_theme = originalSetTheme;
        Gio.Subprocess.new = originalSpawn;
        for (const [prototype, method, original] of hooks) prototype[method] = original;
        originalWrite(`${base}/perf-${name}.json`, JSON.stringify({frames, writes, themes, contrast, workers, operations}));
        return GLib.SOURCE_REMOVE;
    });
    originalWrite(`${base}/perf-armed`, name);
}
