import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {ExtensionState} from 'resource:///org/gnome/shell/misc/extensionUtils.js';
import {
    blurAccentCss, colors, gtkCss, nativeAccent, normalizePanelOptions, paletteRevision,
    panelSliceMode, profiles, shellCss, validAnalysis, validColors, validHues, wallpaperPair,
} from './palette.js';
import {requestUserTheme} from './user-theme-policy.js';

const USER_THEME = 'user-theme@gnome-shell-extensions.gcampax.github.com';
const SCHEMA = 'org.gnome.shell.extensions.user-theme';
const CSS_IMPORT = '/* Duvar palette — managed import */\n@import url("duvar-palette.css");\n';
const REPAIR_DEBOUNCE_MS = 250;
const REPAIR_WINDOW_US = 10_000_000;
const REPAIR_MAX_ATTEMPTS = 5;
const PANEL_INK_CLASSES = ['duvar-ink-light', 'duvar-ink-dark', 'duvar-ink-pill-light', 'duvar-ink-pill-dark'];
const PANEL_HOST_CLASSES = ['duvar-panel-capsule', 'duvar-panel-scrim'];
const PANEL_INK_DEBOUNCE_MS = 80;
const SHELL_MAJOR = parseInt(String(Config.PACKAGE_VERSION ?? '').split('.')[0], 10) || 49;
// GTK 4.20 (shipped with GNOME 49+) supports @media (prefers-color-scheme: dark) in user CSS.
const SUPPORTS_GTK4_MEDIA_SCHEME = SHELL_MAJOR >= 49;

export default class Duvar extends Extension {
    enable() {
        const settings = this._settings = this.getSettings();
        const iface = this._iface = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
        const bg = this._bg = new Gio.Settings({schema_id: 'org.gnome.desktop.background'});
        this._a11y = new Gio.Settings({schema_id: 'org.gnome.desktop.a11y.interface'});
        this._hasAccent = Boolean(iface.settings_schema?.has_key('accent-color'));
        this._hasPrimaryColor = Boolean(bg.settings_schema?.has_key('primary-color'));
        this._systemMenu = Main.panel.statusArea.quickSettings;
        this._systemMenu?.add_style_class_name('duvar-system-menu');
        const context = St.ThemeContext.get_for_stage(global.stage);
        const st = St.Settings.get();
        this._selfWrite = 0;
        this._writingFiles = 0;
        this._themeMonitors = [];
        this._repairTimer = 0;
        this._repairAttempts = 0;
        this._repairWindowStart = 0;
        this._repairExhausted = false;
        this._directSlot = 'a';
        this._panelInkTimer = 0;
        this._panelButtons = new Set();
        this._activeOptical = null;
        this._activeDark = false;
        const resetRepair = () => {
            this._repairAttempts = 0;
            this._repairWindowStart = 0;
            this._repairExhausted = false;
        };
        const schedule = () => {
            resetRepair();
            this._schedule();
        };
        const externalSchedule = () => {
            if (!this._selfWrite) this._schedule();
        };
        this._signals = [
            [settings, settings.connect('changed::palette-enabled', schedule)],
            [settings, settings.connect('changed::palette-follow', schedule)],
            ...['picture-uri', 'picture-uri-dark', ...(this._hasPrimaryColor ? ['primary-color'] : [])].map(key =>
                [bg, bg.connect(`changed::${key}`, () => this._scheduleWallpaper())]),
            [settings, settings.connect('changed::palette-gtk', schedule)],
            [settings, settings.connect('changed::palette-adwaita', schedule)],
            [settings, settings.connect('changed::palette-panel-style', schedule)],
            [settings, settings.connect('changed::palette-panel-capsule-strength', schedule)],
            [settings, settings.connect('changed::palette-colors', schedule)],
            ...(this._hasAccent ? [[iface, iface.connect('changed::accent-color', externalSchedule)]] : []),
            [iface, iface.connect('changed::gtk-theme', externalSchedule)],
            [iface, iface.connect('changed::color-scheme', schedule)],
            [st, st.connect('notify::color-scheme', schedule)],
            [st, st.connect('notify::high-contrast', schedule)],
            [context, context.connect('changed', () => {
                if ((this._blurAccent && !this._hasBlurAccent()) ||
                    (this._directShell && !this._hasDirectShell()))
                    schedule();
            })],
            [Main.extensionManager, Main.extensionManager.connect('extension-state-changed', (_manager, ext) => {
                if (ext.uuid === USER_THEME) schedule();
            })],
            ...(Main.panel ? [
                [Main.panel, Main.panel.connect('notify::allocation', () => this._schedulePanelInks())],
                ...[Main.panel._leftBox, Main.panel._centerBox, Main.panel._rightBox].filter(Boolean).flatMap(box => [
                    [box, box.connect('notify::allocation', () => this._schedulePanelInks())],
                    [box, box.connect('child-added', () => this._schedulePanelInks())],
                    [box, box.connect('child-removed', () => this._schedulePanelInks())],
                ]),
            ] : []),
        ];
        iface.get_string('color-scheme');
        this._update();
    }

    _panelOptions() {
        const s = this._settings;
        const schema = s?.settings_schema;
        return normalizePanelOptions({
            panelStyle: schema?.has_key('palette-panel-style') ? s.get_string('palette-panel-style') : 'capsule',
            capsuleStrength: schema?.has_key('palette-panel-capsule-strength') ? s.get_string('palette-panel-capsule-strength') : 'balanced',
        });
    }

    _wallpapers() {
        const fallback = this._hasPrimaryColor ? this._bg.get_string('primary-color') : '';
        return wallpaperPair(this._bg.get_string('picture-uri'), this._bg.get_string('picture-uri-dark'), fallback);
    }

    _scheduleWallpaper() {
        if (this._wallpaperTimer) GLib.Source.remove(this._wallpaperTimer);
        this._wallpaperTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
            this._wallpaperTimer = 0;
            this._update();
            return GLib.SOURCE_REMOVE;
        });
    }

    _followWallpaper() {
        const s = this._settings;
        if (!s.get_boolean('palette-enabled') || !s.get_boolean('palette-follow')) {
            this._stopWorker();
            this._wallpaperJob = null;
            return;
        }
        if (this._wallpaperTimer) return;
        let saved;
        try { saved = JSON.parse(s.get_string('palette-colors')); } catch {}
        const uris = this._wallpapers();
        const job = JSON.stringify(uris);
        if (job === this._wallpaperJob) return;
        this._stopWorker();
        this._wallpaperJob = job;
        if (saved?.revision === paletteRevision && uris.every((uri, i) => uri === saved.wallpapers?.[i])) return;
        if (uris.some(uri => !uri || (!uri.startsWith('file:') && !uri.startsWith('color:')))) {
            s.set_string('palette-status', 'Yerel görsel seçilmedi; önceki palet korundu.');
            return;
        }
        const worker = this._worker = Gio.Subprocess.new(
            ['gjs', '-m', `${this.path}/palette-worker.js`, ...uris],
            Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_SILENCE);
        this._workerTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 5000, () => {
            this._workerTimer = 0;
            worker.force_exit();
            return GLib.SOURCE_REMOVE;
        });
        worker.communicate_utf8_async(null, null, (_worker, result) => {
            if (worker !== this._worker) return;
            this._clearWorkerTimer();
            this._worker = null;
            if (!s.get_boolean('palette-enabled') || !s.get_boolean('palette-follow') ||
                job !== JSON.stringify(this._wallpapers())) {
                this._wallpaperJob = null;
                return;
            }
            try {
                const [, output] = worker.communicate_utf8_finish(result);
                if (!worker.get_successful()) throw new Error('Worker failed');
                const parsed = JSON.parse(output);
                if (!validAnalysis(parsed) && !validHues(parsed))
                    throw new Error('Invalid palette worker result');
                const hues = parsed.map(item => (item && typeof item === 'object' ? item.hue : item));
                let latest;
                try { latest = JSON.parse(s.get_string('palette-colors')); } catch {}
                const profile = Number.isInteger(latest?.profile) && profiles[latest.profile] ? latest.profile : 0;
                const payload = {
                    light: colors(parsed[0], profile, false),
                    dark: colors(parsed[1], profile, true),
                    profile,
                    hues,
                    ...(validAnalysis(parsed) ? {analysis: parsed} : {}),
                    wallpapers: uris,
                    revision: paletteRevision,
                };
                if (!s.set_string('palette-colors', JSON.stringify(payload)))
                    throw new Error('Palette settings are locked');
            } catch (error) {
                logError(error, 'Duvar wallpaper palette');
                s.set_string('palette-status', 'Görsel analiz edilemedi; önceki palet korundu.');
            }
        });
    }

    _clearWorkerTimer() {
        if (this._workerTimer) GLib.Source.remove(this._workerTimer);
        this._workerTimer = 0;
    }

    _stopWorker() {
        this._clearWorkerTimer();
        const worker = this._worker;
        this._worker = null;
        worker?.force_exit();
    }

    _themeSettings() {
        const ext = Main.extensionManager.lookup(USER_THEME);
        const parent = Gio.SettingsSchemaSource.get_default();
        const dir = ext?.dir?.get_child('schemas');
        const source = dir?.get_child('gschemas.compiled').query_exists(null)
            ? Gio.SettingsSchemaSource.new_from_directory(dir.get_path(), parent, false) : parent;
        const schema = source.lookup(SCHEMA, true);
        return schema ? new Gio.Settings({settings_schema: schema}) : null;
    }

    _ownership() {
        try {
            const record = JSON.parse(this._settings.get_string('palette-ownership'));
            return ['theme', 'accent', 'appliedTheme', 'appliedAccent'].every(k =>
                typeof record?.[k] === 'string') ? record : null;
        }
        catch { return null; }
    }

    _themesDir() {
        return GLib.build_filenamev([GLib.get_user_data_dir(), 'themes']);
    }

    _hasLiveThemes(themesDir) {
        for (const mode of ['light', 'dark']) {
            const dir = `${themesDir}/Duvar-Live-${mode}/gnome-shell`;
            if (!GLib.file_test(`${dir}/gnome-shell.css`, GLib.FileTest.IS_REGULAR) ||
                !GLib.file_test(`${dir}/duvar-blur-accent.css`, GLib.FileTest.IS_REGULAR))
                return false;
        }
        return true;
    }

    _hasGtkSlot(themesDir, slot) {
        if (!slot) return false;
        for (const mode of ['light', 'dark']) {
            for (const gtkDir of ['gtk-3.0', 'gtk-4.0']) {
                const dir = `${themesDir}/Duvar-GTK-${slot}-${mode}/${gtkDir}`;
                if (!GLib.file_test(`${dir}/gtk.css`, GLib.FileTest.IS_REGULAR) ||
                    !GLib.file_test(`${dir}/gtk-dark.css`, GLib.FileTest.IS_REGULAR))
                    return false;
            }
        }
        return true;
    }

    _clearRepairTimer() {
        if (this._repairTimer) GLib.Source.remove(this._repairTimer);
        this._repairTimer = 0;
    }

    _clearThemeMonitors() {
        this._clearRepairTimer();
        for (const [monitor, id] of this._themeMonitors ?? []) {
            monitor.disconnect(id);
            monitor.cancel();
        }
        this._themeMonitors = [];
        this._monitoredKey = null;
    }

    _scheduleRepair() {
        if (this._writingFiles || !this._settings?.get_boolean('palette-enabled') || this._repairExhausted)
            return;
        if (this._repairTimer) GLib.Source.remove(this._repairTimer);
        this._repairTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, REPAIR_DEBOUNCE_MS, () => {
            this._repairTimer = 0;
            if (!this._settings?.get_boolean('palette-enabled')) return GLib.SOURCE_REMOVE;
            const now = GLib.get_monotonic_time();
            if (!this._repairWindowStart || now - this._repairWindowStart > REPAIR_WINDOW_US) {
                this._repairWindowStart = now;
                this._repairAttempts = 0;
            }
            if (this._repairAttempts >= REPAIR_MAX_ATTEMPTS) {
                this._repairExhausted = true;
                this._setString(this._settings, 'palette-status',
                    'Tema dosyaları art arda silindi; otomatik onarım durduruldu.');
                return GLib.SOURCE_REMOVE;
            }
            this._repairAttempts++;
            this._update();
            return GLib.SOURCE_REMOVE;
        });
    }

    _syncThemeMonitors(force = false) {
        if (!this._settings?.get_boolean('palette-enabled')) {
            this._clearThemeMonitors();
            return;
        }
        const themesDir = this._themesDir();
        const gtk4Dir = GLib.build_filenamev([GLib.get_user_config_dir(), 'gtk-4.0']);
        const watchAdwaita = this._settings.get_boolean('palette-adwaita');
        const specs = [
            {
                path: themesDir,
                match: (file, name) => file.get_path() === themesDir ||
                    name === 'duvar-blur-accent.css' ||
                    /^duvar-shell-direct-[ab]\.css$/.test(name) ||
                    /^Duvar-Live-(light|dark)$/.test(name) ||
                    (this._settings?.get_boolean('palette-gtk') &&
                        (!this._gtkSlot
                            ? /^Duvar-GTK-[ab]-(light|dark)$/.test(name)
                            : name.startsWith(`Duvar-GTK-${this._gtkSlot}-`))),
            },
            ...['light', 'dark'].map(mode => {
                const dir = `${themesDir}/Duvar-Live-${mode}/gnome-shell`;
                return {
                    path: dir,
                    match: (file, name) => file.get_path() === dir ||
                        name === 'gnome-shell.css' || name === 'duvar-blur-accent.css',
                };
            }),
            ...(watchAdwaita ? [{
                path: gtk4Dir,
                match: (file, name) => file.get_path() === gtk4Dir ||
                    name === 'duvar-palette.css' || name === 'gtk.css',
            }] : []),
        ];
        const key = specs.map(s => s.path).join('|');
        if (!force && this._monitoredKey === key && this._themeMonitors.length === specs.length)
            return;
        this._clearThemeMonitors();
        for (const spec of specs) {
            if (!GLib.file_test(spec.path, GLib.FileTest.IS_DIR)) continue;
            try {
                const monitor = Gio.File.new_for_path(spec.path)
                    .monitor_directory(Gio.FileMonitorFlags.NONE, null);
                const id = monitor.connect('changed', (_m, file, _other, eventType) => {
                    if (eventType !== Gio.FileMonitorEvent.DELETED &&
                        eventType !== Gio.FileMonitorEvent.MOVED_OUT)
                        return;
                    const name = file?.get_basename();
                    if (file && name && spec.match(file, name))
                        this._scheduleRepair();
                });
                this._themeMonitors.push([monitor, id]);
            } catch (error) {
                logError(error, `Duvar theme monitor (${spec.path})`);
            }
        }
        this._monitoredKey = this._themeMonitors.length === specs.length ? key : null;
    }

    _writeCss(path, content) {
        // Runtime-generated theme sheets are rewritten on mode switches; skip
        // synchronous fsync on the compositor main thread.
        this._writingFiles++;
        try {
            if (typeof GLib.file_set_contents_full === 'function')
                return GLib.file_set_contents_full(path, content, GLib.FileSetContentsFlags.NONE, 0o644);
            return GLib.file_set_contents(path, content);
        } finally {
            this._writingFiles--;
        }
    }

    _setString(settings, key, value) {
        if (settings.get_string(key) === value) return;
        this._selfWrite++;
        try { settings.set_string(key, value); }
        finally { this._selfWrite--; }
    }

    _clearAccentTimer() {
        if (this._accentTimer) GLib.Source.remove(this._accentTimer);
        this._accentTimer = 0;
        this._pendingAccent = null;
    }

    _clearGtkTimer() {
        if (this._gtkTimer) GLib.Source.remove(this._gtkTimer);
        this._gtkTimer = 0;
        this._pendingGtk = null;
    }

    _clearRefreshStart() {
        if (this._refreshStart) GLib.Source.remove(this._refreshStart);
        this._refreshStart = 0;
    }

    _restore(refresh = true) {
        this._clearThemeMonitors();
        this._clearAccentTimer();
        this._clearGtkTimer();
        this._clearRefreshStart();
        this._clearPanelInks();
        this._clearBlurAccent();
        this._clearDirectShell();
        this._syncAdwaita(null, 'light', refresh);
        const record = this._ownership();
        if (!record) return;
        const theme = this._themeSettings();
        if (theme && record.appliedTheme && theme.get_string('name') === record.appliedTheme)
            this._setString(theme, 'name', record.theme);
        if (this._hasAccent && record.appliedAccent && this._iface.get_string('accent-color') === record.appliedAccent)
            this._setString(this._iface, 'accent-color', record.accent);
        this._restoreGtk(record);
        this._setString(this._settings, 'palette-ownership', '');
    }

    _syncAdwaita(pair, mode = 'light', refresh = true, refreshDelay = 0) {
        const dir = GLib.build_filenamev([GLib.get_user_config_dir(), 'gtk-4.0']);
        const path = `${dir}/gtk.css`;
        const companion = `${dir}/duvar-palette.css`;
        const file = Gio.File.new_for_path(path);
        let previous = null;
        try {
            const info = file.query_info('standard::type', Gio.FileQueryInfoFlags.NOFOLLOW_SYMLINKS, null);
            if (info.get_file_type() !== Gio.FileType.REGULAR) return false;
            const [, bytes] = file.load_contents(null);
            previous = new TextDecoder('utf-8', {fatal: true, ignoreBOM: true}).decode(bytes);
        } catch (error) {
            if (!error.matches?.(Gio.IOErrorEnum, Gio.IOErrorEnum.NOT_FOUND)) throw error;
        }
        const original = previous?.replace(CSS_IMPORT, '') ?? '';
        if (!pair) {
            if (previous !== null && original !== previous) {
                // GTK may cache the parent stylesheet bytes in open processes.
                // Neutralize our import for those providers before removing it.
                if (!GLib.file_test(companion, GLib.FileTest.IS_SYMLINK))
                    GLib.file_set_contents(companion, '/* Duvar palette disabled. */\n');
                GLib.file_set_contents(path, original);
                if (refresh && this._settings.get_boolean('palette-adwaita-refresh'))
                    this._refreshAdwaita();
            }
            this._adwaitaPair = null;
            return true;
        }
        if (/rewaita/i.test(original) || original.startsWith('\uFEFF') || /^\s*@charset/i.test(original))
            return false;
        if (GLib.mkdir_with_parents(dir, 0o700) !== 0) throw new Error(`Cannot create ${dir}`);
        const raw = SUPPORTS_GTK4_MEDIA_SCHEME
            ? JSON.stringify([pair.light, pair.dark])
            : JSON.stringify([mode, pair[mode]]);
        const missingCompanion = !GLib.file_test(companion, GLib.FileTest.IS_REGULAR);
        const changed = this._adwaitaPair !== raw || missingCompanion;
        if (changed) {
            const rules = SUPPORTS_GTK4_MEDIA_SCHEME
                ? gtkCss(pair.light, false, true, false, false) +
                    `@media (prefers-color-scheme: dark) {\n${gtkCss(pair.dark, true, true, false, false)}}\n`
                : gtkCss(pair[mode], mode === 'dark', true, false, true);
            if (GLib.file_test(companion, GLib.FileTest.IS_SYMLINK)) return false;
            this._writingFiles++;
            try { GLib.file_set_contents(companion, rules); }
            finally { this._writingFiles--; }
            this._adwaitaPair = raw;
        }
        const next = CSS_IMPORT + original;
        if (previous !== next) {
            this._writingFiles++;
            try { GLib.file_set_contents(path, next); }
            finally { this._writingFiles--; }
        }
        if ((changed || previous !== next) && this._settings.get_boolean('palette-adwaita-refresh'))
            this._refreshAdwaita(refreshDelay);
        return true;
    }

    _refreshAdwaita(delay = 0) {
        const key = 'high-contrast';
        if (this._contrastPulse || this._refreshStart || this._a11y.get_boolean(key) || !this._a11y.is_writable(key)) return;
        const start = () => {
            if (this._contrastPulse || !this._a11y || this._a11y.get_boolean(key) || !this._a11y.is_writable(key)) return;
            this._contrastWasSet = this._a11y.get_user_value(key) !== null;
            if (!this._a11y.set_boolean(key, true)) return;
            this._contrastPulse = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 250, () => {
                this._contrastPulse = 0;
                this._endContrastPulse();
                this._schedule();
                return GLib.SOURCE_REMOVE;
            });
        };
        if (delay > 0) {
            this._refreshStart = GLib.timeout_add(GLib.PRIORITY_DEFAULT, delay, () => {
                this._refreshStart = 0;
                start();
                return GLib.SOURCE_REMOVE;
            });
        } else {
            start();
        }
    }

    _endContrastPulse() {
        const key = 'high-contrast';
        if (!this._a11y.get_boolean(key)) return;
        if (this._contrastWasSet) this._a11y.set_boolean(key, false);
        else this._a11y.reset(key);
    }

    _restoreGtk(record) {
        if (typeof record.gtk === 'string' && record.appliedGtk &&
            this._iface.get_string('gtk-theme') === record.appliedGtk)
            this._setString(this._iface, 'gtk-theme', record.gtk);
    }

    _prepareGtk(pair, currentGtk) {
        const raw = JSON.stringify([pair.light, pair.dark]);
        const themesDir = this._themesDir();
        if (this._gtkPair === raw && this._hasGtkSlot(themesDir, this._gtkSlot)) return false;
        // Write the inactive slot first so an app reopening mid-update sees a complete theme.
        const slot = /^Duvar-GTK-[ab]-(light|dark)$/.test(currentGtk) && currentGtk.startsWith('Duvar-GTK-a-') ? 'b' : 'a';
        const darkCss3 = gtkCss(pair.dark, true, false);
        const darkCss4 = gtkCss(pair.dark, true, true);
        for (const mode of ['light', 'dark']) {
            const themeName = `Duvar-GTK-${slot}-${mode}`;
            const rootDir = GLib.build_filenamev([themesDir, themeName]);
            if (GLib.mkdir_with_parents(rootDir, 0o755) !== 0) throw new Error(`Cannot create ${rootDir}`);
            this._writeCss(`${rootDir}/index.theme`,
                `[Desktop Entry]\nType=X-GNOME-Metatheme\nName=${themeName}\nComment=Generated by Duvar\nEncoding=UTF-8\n\n[X-GNOME-Metatheme]\nGtkTheme=${themeName}\n`);
            for (const gtk4 of [false, true]) {
                const dir = GLib.build_filenamev([rootDir, gtk4 ? 'gtk-4.0' : 'gtk-3.0']);
                if (GLib.mkdir_with_parents(dir, 0o755) !== 0) throw new Error(`Cannot create ${dir}`);
                const mainCss = mode === 'dark' ? (gtk4 ? darkCss4 : darkCss3) : gtkCss(pair.light, false, gtk4);
                this._writeCss(`${dir}/gtk.css`, mainCss);
                // Keep gtk-dark.css dark so apps explicitly requesting dark mode in a
                // light session (e.g. image/video viewers) stay dark and harmonized.
                this._writeCss(`${dir}/gtk-dark.css`, gtk4 ? darkCss4 : darkCss3);
            }
        }
        this._gtkSlot = slot;
        this._gtkPair = raw;
        return true;
    }

    _update() {
        if (this._contrastPulse) return;
        try {
            const s = this._settings;
            const theme = this._themeSettings();
            const themePath = Main.extensionManager.lookup(USER_THEME)?.path;
            if (themePath !== this._themePath || (theme && !this._themeWatch)) {
                if (this._themeWatch) this._themeWatch[0].disconnect(this._themeWatch[1]);
                this._themeWatch = theme ? [theme, theme.connect('changed::name', () => {
                    if (!this._selfWrite) this._schedule();
                })] : null;
                theme?.get_string('name'); // Prime GSettings change notifications.
                this._themePath = themePath;
            }
            let status;
            if (!s.get_boolean('palette-enabled')) {
                this._requestedUserTheme = false;
                this._restore();
                status = 'Kapalı';
            } else {
                status = this._apply(theme);
            }
            this._setString(s, 'palette-status', status);
            this._followWallpaper();
        } catch (e) {
            logError(e, 'Duvar palette');
            this._setString(this._settings, 'palette-status', 'Tema uygulanamadı; GNOME günlüğünü kontrol et.');
        }
    }

    _schedule() {
        if (this._idle) return;
        this._idle = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._idle = 0;
            this._update();
            return GLib.SOURCE_REMOVE;
        });
    }

    _apply(theme) {
        const s = this._settings;
        const userThemeActive = Boolean(theme && Main.extensionManager.lookup(USER_THEME)?.state === ExtensionState.ACTIVE);
        if (!userThemeActive && theme && !this._requestedUserTheme) {
            this._requestedUserTheme = true;
            if (requestUserTheme(Main.extensionManager, USER_THEME))
                return 'User Themes etkinleştiriliyor…';
        }
        if (userThemeActive) this._requestedUserTheme = true;
        if (St.Settings.get().high_contrast) {
            this._restore();
            return 'Yüksek kontrast açık; Duvar renkleri uygulanmadı.';
        }
        let pair;
        try { pair = JSON.parse(s.get_string('palette-colors')); } catch {}
        if (!validColors(pair?.light) || !validColors(pair?.dark))
            return 'Önce bir palet hazırla ve kaydet.';
        const iface = this._iface;
        const currentTheme = userThemeActive ? theme.get_string('name') : '';
        const currentAccent = this._hasAccent ? iface.get_string('accent-color') : '';
        const currentGtk = iface.get_string('gtk-theme');
        const useGtk = s.get_boolean('palette-gtk') && iface.is_writable('gtk-theme');
        const canWriteAccent = this._hasAccent && iface.is_writable('accent-color');
        let record = this._ownership();
        const rewaita = /rewaita/i.test(`${currentTheme} ${currentGtk}`);
        if (record && (
            (userThemeActive && record.appliedTheme && record.appliedTheme !== currentTheme) ||
            (this._hasAccent && record.appliedAccent && record.appliedAccent !== currentAccent) ||
            (record.appliedGtk && record.appliedGtk !== currentGtk)
        )) {
            this._restore();
            s.set_boolean('palette-enabled', false);
            return 'Tema dışarıdan değiştirildi; Duvar kontrolü bıraktı.';
        }
        if (userThemeActive && !theme.is_writable('name'))
            return 'Tema ayarları yazmaya kapalı.';
        record ??= {theme: currentTheme, accent: currentAccent};
        const panelOpts = this._panelOptions();
        const rendered = JSON.stringify([pair.light, pair.dark, panelOpts]);
        const themesDir = this._themesDir();
        const themesMissing = !this._hasLiveThemes(themesDir);
        if (this._pair !== rendered || themesMissing) {
            // Prepare both sheets before selecting one. No decoding or external process in Shell.
            const blurCache = {};
            const shellCache = {};
            for (const [mode, tokens] of Object.entries({light: pair.light, dark: pair.dark})) {
                // Keep the legacy Duvar-light/dark files intact: a saved ownership
                // record may still use one of them as its original theme.
                const dir = `${themesDir}/Duvar-Live-${mode}/gnome-shell`;
                if (GLib.mkdir_with_parents(dir, 0o755) !== 0) throw new Error(`Cannot create ${dir}`);
                const dark = mode === 'dark';
                const shell = shellCss(tokens, dark, panelOpts);
                const blur = blurAccentCss(tokens, dark, panelOpts);
                shellCache[mode] = shell;
                blurCache[mode] = blur;
                this._writeCss(`${dir}/gnome-shell.css`, shell);
                this._writeCss(`${dir}/duvar-blur-accent.css`, blur);
            }
            this._shellCssCache = shellCache;
            this._blurCssCache = blurCache;
            this._pair = rendered;
        }
        if (rewaita) {
            this._restore();
            return 'Palet kaydedildi; Rewaita teması seçili olduğu için uygulanmadı.';
        }
        // GNOME's Shell may prefer dark chrome even when the desktop scheme is
        // “Default”. Follow the desktop wallpaper and application mode instead.
        const mode = iface.get_string('color-scheme') === 'prefer-dark' ? 'dark' : 'light';
        const name = userThemeActive ? `Duvar-Live-${mode}` : '';
        const accent = canWriteAccent ? nativeAccent(pair[mode].accent) : '';
        const activeCss = JSON.stringify([mode, userThemeActive, pair[mode], panelOpts]);
        const path = `${themesDir}/duvar-blur-accent.css`;
        const blurMissing = !GLib.file_test(path, GLib.FileTest.IS_REGULAR);
        const reload = (userThemeActive ? currentTheme !== name : !this._hasDirectShell()) ||
            this._appliedPair !== activeCss || themesMissing || blurMissing;

        let targetGtk = null;
        if (useGtk) {
            this._prepareGtk(pair, currentGtk);
            record.gtk ??= currentGtk;
            targetGtk = `Duvar-GTK-${this._gtkSlot}-${mode}`;
        } else {
            this._clearGtkTimer();
            if (record.appliedGtk && iface.is_writable('gtk-theme')) {
                this._restoreGtk(record);
                delete record.gtk;
                delete record.appliedGtk;
            }
        }
        if (this._gtkTimer && this._pendingGtk !== targetGtk)
            this._clearGtkTimer();
        if (this._accentTimer && this._pendingAccent !== accent)
            this._clearAccentTimer();
        // Avoid stacking gtk-theme, accent-color restyle, and Main.loadTheme() in the same frame.
        const deferGtk = Boolean(targetGtk) && (reload || Boolean(this._gtkTimer)) && currentGtk !== targetGtk;
        if (targetGtk) record.appliedGtk = deferGtk ? currentGtk : targetGtk;
        const deferAccent = canWriteAccent && (reload || Boolean(this._accentTimer)) && currentAccent !== accent;
        const appliedAccent = canWriteAccent ? (deferAccent ? currentAccent : accent) : '';
        this._setString(s, 'palette-ownership', JSON.stringify({...record, appliedTheme: name, appliedAccent}));

        if (canWriteAccent && currentAccent !== accent && !deferAccent)
            this._setString(iface, 'accent-color', accent);

        if (userThemeActive) {
            this._clearDirectShell();
            // Main.loadTheme reparses the existing custom stylesheets before
            // installing the new theme. Keep the same path and extension origin;
            // separate unload/load calls on the running theme restyle it twice.
            if (reload || !this._blurAccent)
                this._writeCss(path, this._blurCssCache?.[mode] ?? blurAccentCss(pair[mode], mode === 'dark', panelOpts));
            if (currentTheme !== name) this._setString(theme, 'name', name);
            else if (reload) Main.loadTheme();
            if (!this._hasBlurAccent()) {
                this._clearBlurAccent();
                const file = Gio.File.new_for_path(path);
                St.ThemeContext.get_for_stage(global.stage).get_theme().load_stylesheet(file);
                this._blurAccent = file;
            }
        } else {
            // Works out of the box on any GNOME desktop even when User Themes is not installed.
            this._clearBlurAccent();
            if (reload || !this._hasDirectShell()) {
                const nextSlot = this._directSlot === 'a' ? 'b' : 'a';
                const directPath = `${themesDir}/duvar-shell-direct-${nextSlot}.css`;
                const shell = this._shellCssCache?.[mode] ?? shellCss(pair[mode], mode === 'dark', panelOpts);
                const blur = this._blurCssCache?.[mode] ?? blurAccentCss(pair[mode], mode === 'dark', panelOpts);
                this._writeCss(directPath, `${shell}\n${blur}`);
                const stTheme = St.ThemeContext.get_for_stage(global.stage).get_theme();
                const nextFile = Gio.File.new_for_path(directPath);
                const prevFile = this._directShell;
                stTheme.load_stylesheet(nextFile);
                if (prevFile && !prevFile.equal(nextFile))
                    stTheme.unload_stylesheet(prevFile);
                this._directShell = nextFile;
                this._directSlot = nextSlot;
            }
        }

        this._appliedPair = activeCss;
        this._activeOptical = pair[mode].optical ?? null;
        this._activeDark = mode === 'dark';
        this._syncPanelInks();
        this._schedulePanelInks();
        if (targetGtk && currentGtk !== targetGtk && !deferGtk)
            this._setString(iface, 'gtk-theme', targetGtk);
        if (deferGtk && !this._gtkTimer) {
            this._pendingGtk = targetGtk;
            this._gtkTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 60, () => {
                this._gtkTimer = 0;
                const target = this._pendingGtk;
                this._pendingGtk = null;
                if (!target || !this._settings?.get_boolean('palette-enabled') ||
                    !this._settings?.get_boolean('palette-gtk') || !this._iface?.is_writable('gtk-theme'))
                    return GLib.SOURCE_REMOVE;
                const latest = this._ownership();
                if (!latest || this._iface.get_string('gtk-theme') !== latest.appliedGtk)
                    return GLib.SOURCE_REMOVE;
                this._setString(this._settings, 'palette-ownership', JSON.stringify({...latest, appliedGtk: target}));
                this._setString(this._iface, 'gtk-theme', target);
                return GLib.SOURCE_REMOVE;
            });
        }
        if (deferAccent && !this._accentTimer) {
            this._pendingAccent = accent;
            this._accentTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 160, () => {
                this._accentTimer = 0;
                const target = this._pendingAccent;
                this._pendingAccent = null;
                if (!target || !this._settings?.get_boolean('palette-enabled') || !this._hasAccent || !this._iface?.is_writable('accent-color'))
                    return GLib.SOURCE_REMOVE;
                const latest = this._ownership();
                if (!latest || this._iface.get_string('accent-color') !== latest.appliedAccent)
                    return GLib.SOURCE_REMOVE;
                this._setString(this._settings, 'palette-ownership', JSON.stringify({...latest, appliedAccent: target}));
                this._setString(this._iface, 'accent-color', target);
                return GLib.SOURCE_REMOVE;
            });
        }
        if (s.get_boolean('palette-adwaita')) {
            const gtk4Missing = !GLib.file_test(
                GLib.build_filenamev([GLib.get_user_config_dir(), 'gtk-4.0', 'duvar-palette.css']),
                GLib.FileTest.IS_REGULAR);
            const refreshDelay = reload ? (this._accentTimer ? 280 : 140) : 0;
            const synced = this._syncAdwaita(pair, mode, true, refreshDelay);
            this._syncThemeMonitors(themesMissing || gtk4Missing);
            if (!synced) return 'Shell etkin · Mevcut GTK CSS’si korunuyor; libadwaita uygulanmadı.';
            return 'Etkin · Shell ve libadwaita' + (useGtk ? ' ve GTK' : '');
        }
        this._syncAdwaita(null, mode);
        this._syncThemeMonitors(themesMissing);
        return useGtk ? 'Etkin · Shell ve deneysel GTK teması' :
            'Etkin · GNOME vurgusu ve Duvar Shell teması';
    }

    _hasBlurAccent() {
        return this._blurAccent && St.ThemeContext.get_for_stage(global.stage).get_theme()
            .get_custom_stylesheets().some(file => file.equal(this._blurAccent));
    }

    _clearBlurAccent() {
        if (!this._blurAccent) return;
        const file = this._blurAccent;
        this._blurAccent = null;
        St.ThemeContext.get_for_stage(global.stage).get_theme().unload_stylesheet(file);
    }

    _hasDirectShell() {
        return this._directShell && St.ThemeContext.get_for_stage(global.stage).get_theme()
            .get_custom_stylesheets().some(file => file.equal(this._directShell));
    }

    _clearDirectShell() {
        if (!this._directShell) return;
        const file = this._directShell;
        this._directShell = null;
        St.ThemeContext.get_for_stage(global.stage).get_theme().unload_stylesheet(file);
    }

    _collectPanelButtons() {
        const buttons = [];
        const visit = (actor, depth) => {
            if (!actor || depth > 2) return;
            if (typeof actor.has_style_class_name === 'function' && actor.has_style_class_name('panel-button')) {
                if (!actor.has_style_class_name('screen-recording-indicator') &&
                    !actor.has_style_class_name('screen-sharing-indicator'))
                    buttons.push(actor);
                return;
            }
            if (typeof actor.get_children === 'function') {
                for (const child of actor.get_children())
                    visit(child, depth + 1);
            }
        };
        for (const box of [Main.panel?._leftBox, Main.panel?._centerBox, Main.panel?._rightBox]) {
            if (!box || typeof box.get_children !== 'function') continue;
            for (const child of box.get_children())
                visit(child, 0);
        }
        return buttons;
    }

    _schedulePanelInks() {
        if (!this._settings?.get_boolean('palette-enabled')) return;
        const {panelStyle} = this._panelOptions();
        if (!this._activeOptical && panelStyle !== 'capsule') return;
        if (this._panelInkTimer) GLib.Source.remove(this._panelInkTimer);
        this._panelInkTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, PANEL_INK_DEBOUNCE_MS, () => {
            this._panelInkTimer = 0;
            this._syncPanelInks();
            return GLib.SOURCE_REMOVE;
        });
    }

    _syncPanelHostClasses(panelStyle) {
        if (!Main.panel) return;
        const wantCapsule = panelStyle === 'capsule';
        const wantScrim = panelStyle === 'scrim';
        const toggle = (cls, on) => {
            const has = Main.panel.has_style_class_name(cls);
            if (on && !has) Main.panel.add_style_class_name(cls);
            else if (!on && has) Main.panel.remove_style_class_name(cls);
        };
        toggle('duvar-panel-capsule', wantCapsule);
        toggle('duvar-panel-scrim', wantScrim);
    }

    _syncPanelInks() {
        if (!Main.panel || !this._settings?.get_boolean('palette-enabled')) {
            this._clearPanelInks();
            return;
        }
        const {panelStyle} = this._panelOptions();
        const optical = this._activeOptical;
        if (!optical && panelStyle !== 'capsule') {
            this._clearPanelInks();
            return;
        }
        this._syncPanelHostClasses(panelStyle);
        const currentButtons = new Set(this._collectPanelButtons());
        for (const prev of this._panelButtons ?? []) {
            if (!currentButtons.has(prev)) {
                try {
                    for (const cls of PANEL_INK_CLASSES)
                        prev.remove_style_class_name(cls);
                } catch {}
            }
        }
        this._panelButtons = currentButtons;
        // Fast O(N) zero-geometry path when uniform floating glass capsules are active.
        if (panelStyle === 'capsule') {
            const targetClass = this._activeDark ? 'duvar-ink-pill-dark' : 'duvar-ink-pill-light';
            for (const button of currentButtons) {
                if (!button.visible) continue;
                for (const cls of PANEL_INK_CLASSES) {
                    if (cls !== targetClass && button.has_style_class_name(cls))
                        button.remove_style_class_name(cls);
                }
                if (!button.has_style_class_name(targetClass))
                    button.add_style_class_name(targetClass);
            }
            return;
        }
        const [panelX] = Main.panel.get_transformed_position();
        const panelW = Main.panel.width;
        if (!Number.isFinite(panelX) || !Number.isFinite(panelW) || panelW <= 0)
            return;
        for (const button of currentButtons) {
            if (!button.visible) continue;
            const [bx] = button.get_transformed_position();
            const bw = button.width;
            if (!Number.isFinite(bx) || !Number.isFinite(bw) || bw <= 0) continue;
            const x0 = Math.max(0, Math.min(1, (bx - panelX) / panelW));
            const x1 = Math.max(x0, Math.min(1, (bx + bw - panelX) / panelW));
            const mode = panelSliceMode(optical, x0, x1, this._activeDark, panelStyle);
            const targetClass = `duvar-ink-${mode}`;
            for (const cls of PANEL_INK_CLASSES) {
                if (cls !== targetClass && button.has_style_class_name(cls))
                    button.remove_style_class_name(cls);
            }
            if (!button.has_style_class_name(targetClass))
                button.add_style_class_name(targetClass);
        }
    }

    _clearPanelInks() {
        if (this._panelInkTimer) {
            GLib.Source.remove(this._panelInkTimer);
            this._panelInkTimer = 0;
        }
        if (Main.panel) {
            for (const cls of PANEL_HOST_CLASSES) {
                try {
                    if (Main.panel.has_style_class_name(cls))
                        Main.panel.remove_style_class_name(cls);
                } catch {}
            }
        }
        const targets = new Set([...(this._panelButtons ?? []), ...this._collectPanelButtons()]);
        for (const button of targets) {
            try {
                for (const cls of PANEL_INK_CLASSES)
                    button.remove_style_class_name(cls);
            } catch {}
        }
        this._panelButtons = new Set();
        this._activeOptical = null;
    }

    disable() {
        this._clearAccentTimer();
        this._clearGtkTimer();
        this._clearRefreshStart();
        if (this._contrastPulse) {
            GLib.Source.remove(this._contrastPulse);
            this._contrastPulse = 0;
            this._endContrastPulse();
        }
        this._stopWorker();
        this._wallpaperJob = null;
        this._requestedUserTheme = false;
        for (const [object, id] of this._signals) object.disconnect(id);
        this._signals = [];
        this._systemMenu?.remove_style_class_name('duvar-system-menu');
        this._systemMenu = null;
        if (this._themeWatch) this._themeWatch[0].disconnect(this._themeWatch[1]);
        this._themeWatch = this._themePath = null;
        if (this._idle) GLib.Source.remove(this._idle);
        this._idle = 0;
        if (this._wallpaperTimer) GLib.Source.remove(this._wallpaperTimer);
        this._wallpaperTimer = 0;
        this._restore(false);
        this._gtkPair = this._gtkSlot = this._blurCssCache = this._shellCssCache = null;
        this._selfWrite = 0;
        this._writingFiles = 0;
        this._repairAttempts = 0;
        this._repairWindowStart = 0;
        this._repairExhausted = false;
        this._settings = this._iface = this._bg = this._a11y = this._pair = this._appliedPair = null;
    }
}
