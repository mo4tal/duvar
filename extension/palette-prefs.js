import Adw from 'gi://Adw';
import Gdk from 'gi://Gdk';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk';
import {
    CAPSULE_STRENGTHS, PANEL_STYLES, capsuleAlphas, colors, layers,
    normalizePanelOptions, paletteRevision, profiles, validAnalysis,
    validColors, validHues, wallpaperPair,
} from './palette.js';

function previewCard(modeClass, heading) {
    const card = new Gtk.Box({
        orientation: Gtk.Orientation.VERTICAL,
        spacing: 10,
        hexpand: true,
        css_classes: ['duvar-preview-card', modeClass],
    });
    // Live top-bar strip preview showing floating glass capsules / scrim / plain ink.
    const topStrip = new Gtk.Box({
        orientation: Gtk.Orientation.HORIZONTAL,
        spacing: 6,
        css_classes: ['duvar-preview-topbar'],
    });
    const dotPill = new Gtk.Box({
        orientation: Gtk.Orientation.HORIZONTAL,
        spacing: 4,
        valign: Gtk.Align.CENTER,
        css_classes: ['duvar-preview-bar-item'],
    });
    dotPill.append(new Gtk.Box({css_classes: ['duvar-preview-ws-pill']}));
    dotPill.append(new Gtk.Box({css_classes: ['duvar-preview-ws-dot']}));
    const clockPill = new Gtk.Label({
        label: '16:35',
        hexpand: true,
        halign: Gtk.Align.CENTER,
        valign: Gtk.Align.CENTER,
        css_classes: ['duvar-preview-bar-item', 'duvar-preview-bar-clock'],
    });
    const trayPill = new Gtk.Label({
        label: '● ●',
        valign: Gtk.Align.CENTER,
        css_classes: ['duvar-preview-bar-item', 'duvar-preview-bar-tray'],
    });
    topStrip.append(dotPill);
    topStrip.append(clockPill);
    topStrip.append(trayPill);

    const head = new Gtk.Box({orientation: Gtk.Orientation.HORIZONTAL, spacing: 8});
    head.append(new Gtk.Label({
        label: heading,
        xalign: 0,
        hexpand: true,
        css_classes: ['duvar-preview-title'],
    }));
    head.append(new Gtk.Label({
        label: 'Etkin',
        valign: Gtk.Align.CENTER,
        css_classes: ['duvar-preview-pill', 'duvar-preview-accent'],
    }));
    const inner = new Gtk.Box({
        orientation: Gtk.Orientation.VERTICAL,
        spacing: 2,
        css_classes: ['duvar-preview-inner'],
    });
    inner.append(new Gtk.Label({label: 'Kart', xalign: 0, css_classes: ['duvar-preview-text']}));
    inner.append(new Gtk.Label({label: 'İkincil', xalign: 0, css_classes: ['duvar-preview-secondary']}));
    const chips = new Gtk.Box({orientation: Gtk.Orientation.HORIZONTAL, spacing: 6, homogeneous: true});
    for (const [label, role] of [['Pasif', 'passive'], ['Seçim', 'selected'], ['Odak', 'focus']]) {
        chips.append(new Gtk.Label({
            label,
            xalign: 0.5,
            hexpand: true,
            css_classes: ['duvar-preview-chip', `duvar-preview-${role}`],
        }));
    }
    card.append(topStrip);
    card.append(head);
    card.append(inner);
    card.append(chips);
    return card;
}

function rgbaHex(hex, alpha) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r},${g},${b},${Number(alpha.toFixed(2))})`;
}

function previewCss(pair, panelOpts) {
    const {panelStyle, capsuleStrength} = normalizePanelOptions(panelOpts);
    const base = `
.duvar-preview-card { border-radius: 14px; padding: 12px; border: 1px solid transparent; }
.duvar-preview-topbar { border-radius: 9px; padding: 4px 6px; min-height: 24px; }
.duvar-preview-bar-item { border-radius: 999px; padding: 2px 8px; font-size: 0.78em; font-weight: 700; }
.duvar-preview-ws-pill { min-width: 14px; min-height: 5px; border-radius: 999px; }
.duvar-preview-ws-dot { min-width: 5px; min-height: 5px; border-radius: 999px; opacity: 0.6; }
.duvar-preview-title { font-weight: 700; }
.duvar-preview-pill { border-radius: 999px; padding: 2px 10px; font-weight: 600; font-size: 0.85em; }
.duvar-preview-inner { border-radius: 9px; padding: 8px 10px; border: 1px solid transparent; }
.duvar-preview-text { font-weight: 600; }
.duvar-preview-secondary { font-size: 0.88em; }
.duvar-preview-chip { border-radius: 7px; padding: 4px 6px; font-size: 0.84em; border: 1px solid transparent; }
`;
    return base + pair.map((tokens, j) => {
        const dark = j === 1;
        const cls = dark ? '.duvar-preview-dark' : '.duvar-preview-light';
        const {surface, text} = tokens;
        const {card, secondary, control, controlEdge, border, frame, selected, focus, accentBg, accentFg} = layers(tokens, dark);
        const {lightGlass, darkGlass, lightAlpha, darkAlpha} = capsuleAlphas(tokens, dark, capsuleStrength);
        const barBg = dark
            ? 'linear-gradient(90deg, #1b2230 0%, #5b697c 50%, #e4e9f2 100%)'
            : 'linear-gradient(90deg, #16181f 0%, #647182 50%, #eef2f7 100%)';
        const scrimOverlay = dark
            ? 'linear-gradient(180deg, rgba(16,18,24,0.62) 0%, rgba(16,18,24,0.12) 100%), ' + barBg
            : 'linear-gradient(180deg, rgba(242,244,248,0.68) 0%, rgba(242,244,248,0.14) 100%), ' + barBg;
        const usePill = panelStyle === 'capsule';
        const useSmart = panelStyle === 'smart';
        const pillBg = dark ? rgbaHex(darkGlass, darkAlpha) : rgbaHex(lightGlass, lightAlpha);
        const pillInk = dark ? '#f5f5f5' : '#191919';
        const barCss = panelStyle === 'scrim' ? scrimOverlay : barBg;
        const itemBg = usePill ? pillBg : 'transparent';
        const itemInk = usePill ? pillInk : (panelStyle === 'scrim' ? (dark ? '#f5f5f5' : '#191919') : '#f5f5f5');
        const smartClockRule = useSmart
            ? `${cls} .duvar-preview-bar-clock { background-color: ${pillBg}; color: ${pillInk}; }\n` +
              `${cls} .duvar-preview-bar-tray { color: #191919; }\n`
            : '';
        return `${cls} { background-color: ${surface}; color: ${text}; border-color: ${frame}; }\n` +
            `${cls} .duvar-preview-topbar { background-image: ${barCss}; }\n` +
            `${cls} .duvar-preview-bar-item { background-color: ${itemBg}; color: ${itemInk}; }\n` +
            `${cls} .duvar-preview-ws-pill, ${cls} .duvar-preview-ws-dot { background-color: ${itemInk}; }\n` +
            smartClockRule +
            `${cls} .duvar-preview-title, ${cls} .duvar-preview-text { color: ${text}; }\n` +
            `${cls} .duvar-preview-accent { background-color: ${accentBg}; color: ${accentFg}; }\n` +
            `${cls} .duvar-preview-inner { background-color: ${card}; border-color: ${border}; }\n` +
            `${cls} .duvar-preview-secondary { color: ${secondary}; }\n` +
            `${cls} .duvar-preview-passive { background-color: ${control}; color: ${text}; border-color: ${controlEdge}; }\n` +
            `${cls} .duvar-preview-selected { background-color: ${selected}; color: ${text}; border-color: ${border}; }\n` +
            `${cls} .duvar-preview-focus { background-color: ${control}; color: ${text}; box-shadow: inset 0 0 0 2px ${focus}; }\n`;
    }).join('');
}

function localFileExists(uri) {
    if (!uri) return false;
    if (uri.startsWith('color:')) return true;
    if (!uri.startsWith('file://')) return false;
    try {
        return Gio.File.new_for_uri(uri).query_exists(null);
    } catch {
        return false;
    }
}

export function addPalettePage(window, settings, path, addWallpaperAction = null) {
    const bg = new Gio.Settings({schema_id: 'org.gnome.desktop.background'});
    const desktop = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
    const schema = settings.settings_schema;
    const hasPanelStyle = Boolean(schema?.has_key('palette-panel-style'));
    const hasCapsuleStrength = Boolean(schema?.has_key('palette-panel-capsule-strength'));

    const page = new Adw.PreferencesPage({title: 'Renkler', icon_name: 'applications-graphics-symbolic'});
    const subscriptions = [];
    const watch = (source, key, callback) =>
        subscriptions.push([source, source.connect(`changed::${key}`, callback)]);
    let closed = false;

    const currentPanelOptions = () => normalizePanelOptions({
        panelStyle: hasPanelStyle ? settings.get_string('palette-panel-style') : 'capsule',
        capsuleStrength: hasCapsuleStrength ? settings.get_string('palette-panel-capsule-strength') : 'balanced',
    });

    // 1. Primary activation & live status
    const group = new Adw.PreferencesGroup({
        title: 'Duvar kâğıdından tema',
        description: 'Açık ve koyu duvar kâğıtlarından GNOME Shell, üst panel ve uygulamalar için canlı renk üretir.',
    });
    const active = new Adw.SwitchRow({
        title: 'Duvar temasını kullan',
        subtitle: 'User Themes olmadan da çalışır; başka bir Shell tema yöneticisiyle birlikte kullanma.',
    });
    settings.bind('palette-enabled', active, 'active', Gio.SettingsBindFlags.DEFAULT);
    const status = new Adw.ActionRow({title: 'Tema durumu'});
    status.add_prefix(new Gtk.Image({icon_name: 'emblem-default-symbolic', css_classes: ['dim-label']}));
    const updateStatus = () => {
        const txt = settings.get_string('palette-status');
        status.subtitle = txt || (settings.get_boolean('palette-enabled') ? 'Etkin' : 'Kapalı');
    };
    updateStatus();
    watch(settings, 'palette-status', updateStatus);
    watch(settings, 'palette-enabled', updateStatus);
    group.add(active);
    group.add(status);
    page.add(group);

    // 2. Live Visual Preview (Light + Dark side-by-side with top-panel capsule preview)
    const preview = new Adw.PreferencesGroup({title: 'Önizleme', visible: false});
    const swatches = new Gtk.Box({spacing: 12, homogeneous: true});
    swatches.append(previewCard('duvar-preview-light', 'Açık görünüm'));
    swatches.append(previewCard('duvar-preview-dark', 'Koyu görünüm'));
    preview.add(swatches);
    page.add(preview);

    // 3. Top Panel & Floating Glass Capsules (instant live switching in both light & dark modes)
    const panelGroup = new Adw.PreferencesGroup({
        title: 'Üst panel ve kapsül görünümü',
        description: 'Hem açık hem koyu temada üst panel öğelerinin okunabilirlik stilini anlık olarak seç.',
    });
    const panelStyleRow = new Adw.ComboRow({
        title: 'Üst panel stili',
        model: Gtk.StringList.new(PANEL_STYLES.map(([, label]) => label)),
        subtitle_lines: 2,
        sensitive: hasPanelStyle,
    });
    const capsuleStrengthRow = new Adw.ComboRow({
        title: 'Kapsül cam yoğunluğu',
        model: Gtk.StringList.new(CAPSULE_STRENGTHS.map(([, label]) => label)),
        subtitle_lines: 2,
        sensitive: hasCapsuleStrength,
    });
    panelGroup.add(panelStyleRow);
    panelGroup.add(capsuleStrengthRow);
    page.add(panelGroup);

    // 4. Palette Character Selection
    const draft = new Adw.PreferencesGroup({title: 'Renk karakteri', description: 'Görseller inceleniyor…'});
    const model = Gtk.StringList.new(profiles.map(p => p[0]));
    const pick = new Adw.ComboRow({title: 'Palet', model, subtitle_lines: 2});
    const apply = new Adw.ButtonRow({
        title: 'Seçili paleti kaydet',
        start_icon_name: 'emblem-ok-symbolic',
        sensitive: false,
        css_classes: ['suggested-action'],
    });
    const prepare = new Adw.ButtonRow({title: '10 paleti hazırla', start_icon_name: 'view-refresh-symbolic'});
    for (const row of [pick, apply, prepare]) draft.add(row);
    page.add(draft);

    // 5. Active Light & Dark Wallpapers
    const backgrounds = new Adw.PreferencesGroup({
        title: 'Duvar kâğıtları',
        description: addWallpaperAction ? 'Her düğme yalnızca adını taşıdığı temanın görselini değiştirir.' :
            'Keşfet veya İndirilenler sekmelerinden açık ve koyu duvar kâğıtlarını doğrudan atayabilirsin.',
    });
    const mode = () => desktop.get_string('color-scheme') === 'prefer-dark' ? 'dark' : 'light';
    for (const [targetMode, key, title, icon] of [
        ['light', 'picture-uri', 'Açık tema', 'weather-clear-symbolic'],
        ['dark', 'picture-uri-dark', 'Koyu tema', 'weather-clear-night-symbolic'],
    ]) {
        const row = new Adw.ActionRow({title, subtitle_lines: 1});
        row.add_prefix(new Gtk.Image({icon_name: icon}));
        const badge = new Gtk.Label({label: 'Etkin', valign: Gtk.Align.CENTER, css_classes: ['caption', 'accent']});
        row.add_suffix(badge);
        const update = () => {
            const uri = bg.get_string(key);
            row.subtitle = GLib.markup_escape_text(uri ? Gio.File.new_for_uri(uri).get_basename() ?? 'Görsel yok' : 'Görsel yok', -1);
            badge.visible = mode() === targetMode;
        };
        update();
        watch(bg, key, update);
        watch(desktop, 'color-scheme', update);
        backgrounds.add(row);
        addWallpaperAction?.(backgrounds, window, settings, targetMode, title, () => closed);
    }
    page.add(backgrounds);

    // 6. Application Layers & Automation
    const follow = new Adw.SwitchRow({
        title: 'Duvar kâğıdı değişince renkleri güncelle',
        subtitle: 'Yeni görsel seçildiğinde mevcut palet karakteriyle renkleri otomatik yeniler.',
    });
    settings.bind('palette-follow', follow, 'active', Gio.SettingsBindFlags.DEFAULT);
    const gtkTheme = new Adw.SwitchRow({
        title: 'GTK uygulamalarını da renklendir',
        subtitle: 'GTK 3 ve klasik GTK 4 uygulamalarına uyumlu palet uygular.',
    });
    settings.bind('palette-gtk', gtkTheme, 'active', Gio.SettingsBindFlags.DEFAULT);
    const adwaita = new Adw.SwitchRow({
        title: 'Libadwaita uygulamalarını renklendir',
        subtitle: 'GTK 4.12+ · Kullanıcı CSS’sine geri alınabilir bir bağlantı ekler.',
        sensitive: Gtk.check_version(4, 12, 0) === null,
    });
    settings.bind('palette-adwaita', adwaita, 'active', Gio.SettingsBindFlags.DEFAULT);
    const refresh = new Adw.SwitchRow({
        title: 'Açık libadwaita pencerelerini yenile',
        subtitle: 'Yüksek kontrastı anlık açıp kapatarak açık pencereleri tazeler.',
    });
    settings.bind('palette-adwaita-refresh', refresh, 'active', Gio.SettingsBindFlags.DEFAULT);
    settings.bind('palette-adwaita', refresh, 'sensitive', Gio.SettingsBindFlags.GET);
    const options = new Adw.PreferencesGroup({
        title: 'Uygulama katmanları ve takip',
        description: 'Renklerin hangi masaüstü katmanlarına yansıtılacağını belirle.',
    });
    for (const row of [follow, gtkTheme, adwaita, refresh]) options.add(row);
    page.add(options);

    // Live CSS provider for preview cards
    const provider = new Gtk.CssProvider();
    const display = window.get_display() ?? Gdk.Display.get_default();
    if (display)
        Gtk.StyleContext.add_provider_for_display(display, provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION);

    let analysis, snapshot, proc, saved, timeout, autoTried = '';
    let syncingPanelUi = false;
    const hasPrimaryColor = Boolean(bg.settings_schema?.has_key('primary-color'));
    const current = () => wallpaperPair(bg.get_string('picture-uri'), bg.get_string('picture-uri-dark'),
        hasPrimaryColor ? bg.get_string('primary-color') : '');
    const samePair = pair => Array.isArray(pair) && pair.length === 2 && current().every((uri, i) => uri === pair[i]);
    const readSaved = () => { try { return JSON.parse(settings.get_string('palette-colors')); } catch { return null; } };

    const syncPanelControls = () => {
        syncingPanelUi = true;
        try {
            const {panelStyle, capsuleStrength} = currentPanelOptions();
            const styleIdx = Math.max(0, PANEL_STYLES.findIndex(([id]) => id === panelStyle));
            const strengthIdx = Math.max(0, CAPSULE_STRENGTHS.findIndex(([id]) => id === capsuleStrength));
            if (panelStyleRow.selected !== styleIdx) panelStyleRow.selected = styleIdx;
            panelStyleRow.subtitle = PANEL_STYLES[styleIdx]?.[2] ?? '';
            if (capsuleStrengthRow.selected !== strengthIdx) capsuleStrengthRow.selected = strengthIdx;
            capsuleStrengthRow.subtitle = CAPSULE_STRENGTHS[strengthIdx]?.[2] ?? '';
            capsuleStrengthRow.sensitive = hasCapsuleStrength && (panelStyle === 'capsule' || panelStyle === 'smart');
        } finally {
            syncingPanelUi = false;
        }
    };

    const showPreview = () => {
        const i = pick.selected;
        const pair = analysis && i < profiles.length ? [colors(analysis[0], i, false), colors(analysis[1], i, true)] :
            [saved?.light, saved?.dark];
        preview.title = analysis ? 'Önizleme' : 'Son kayıt';
        pick.subtitle = profiles[i]?.[1] ?? '';
        pick.sensitive = Boolean(analysis);
        apply.sensitive = Boolean(analysis && i < profiles.length &&
            (!samePair(saved?.wallpapers) || saved.revision !== paletteRevision || i !== saved.profile ||
            !validColors(saved.light) || !validColors(saved.dark)));
        preview.visible = Boolean(!proc && pair.every(validColors));
        if (!preview.visible) return;
        provider.load_from_string(previewCss(pair, currentPanelOptions()));
    };

    const startAnalyze = () => {
        if (proc) return;
        analysis = null;
        showPreview();
        apply.sensitive = false;
        snapshot = current();
        const uris = snapshot;
        try {
            proc = Gio.Subprocess.new(['gjs', '-m', `${path}/palette-worker.js`, ...uris],
                Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE);
        } catch {
            draft.description = 'Renk analiz aracı başlatılamadı.';
            return;
        }
        prepare.sensitive = false;
        showPreview();
        draft.description = 'Açık ve koyu görseller inceleniyor…';
        timeout = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 12, () => {
            timeout = 0;
            proc?.force_exit();
            return GLib.SOURCE_REMOVE;
        });
        proc.communicate_utf8_async(null, null, (child, res) => {
            if (timeout) GLib.Source.remove(timeout);
            timeout = 0;
            proc = null;
            if (closed) return;
            prepare.sensitive = true;
            if (!samePair(uris)) {
                sync();
                return;
            }
            try {
                const [, out] = child.communicate_utf8_finish(res);
                if (!child.get_successful()) throw new Error();
                const parsed = JSON.parse(out);
                if (!validAnalysis(parsed) && !validHues(parsed))
                    throw new Error();
                analysis = parsed;
                sync();
            } catch {
                draft.description = 'Görsel okunamadı veya analiz zaman aşımına uğradı. Yerel bir görsel seçip tekrar dene.';
                showPreview();
            }
        });
    };

    const maybeAutoAnalyze = () => {
        if (closed || proc || analysis) return;
        const uris = current();
        const key = uris.join('\0');
        if (!uris.every(localFileExists) || autoTried === key) return;
        autoTried = key;
        startAnalyze();
    };

    const sync = () => {
        saved = readSaved();
        if (!analysis && Number.isInteger(saved?.profile) && saved.profile >= 0 && saved.profile < profiles.length)
            pick.selected = saved.profile;
        if (analysis && !samePair(snapshot))
            analysis = null;
        if (!proc && !analysis && saved?.revision === paletteRevision && samePair(saved.wallpapers) &&
            (validAnalysis(saved.analysis) || validHues(saved.hues))) {
            analysis = validAnalysis(saved.analysis) ? saved.analysis : saved.hues;
            snapshot = saved.wallpapers;
        }
        draft.description = proc ? 'Açık ve koyu görseller inceleniyor…' : analysis ?
            'Üstteki açık ve koyu önizlemeleri karşılaştırıp istediğin karakteri kaydet.' :
            'Bu duvar kâğıtlarına ait renkleri görmek için paletleri hazırla.';
        syncPanelControls();
        showPreview();
        if (window.get_mapped())
            maybeAutoAnalyze();
    };

    sync();
    watch(bg, 'picture-uri', sync);
    watch(bg, 'picture-uri-dark', sync);
    if (hasPrimaryColor) watch(bg, 'primary-color', sync);
    watch(settings, 'palette-colors', sync);
    if (hasPanelStyle) watch(settings, 'palette-panel-style', () => { syncPanelControls(); showPreview(); });
    if (hasCapsuleStrength) watch(settings, 'palette-panel-capsule-strength', () => { syncPanelControls(); showPreview(); });

    panelStyleRow.connect('notify::selected', () => {
        if (syncingPanelUi || !hasPanelStyle) return;
        const entry = PANEL_STYLES[panelStyleRow.selected];
        if (!entry) return;
        settings.set_string('palette-panel-style', entry[0]);
        syncPanelControls();
        showPreview();
    });
    capsuleStrengthRow.connect('notify::selected', () => {
        if (syncingPanelUi || !hasCapsuleStrength) return;
        const entry = CAPSULE_STRENGTHS[capsuleStrengthRow.selected];
        if (!entry) return;
        settings.set_string('palette-panel-capsule-strength', entry[0]);
        syncPanelControls();
        showPreview();
    });

    pick.connect('notify::selected', showPreview);
    prepare.connect('activated', () => {
        autoTried = '';
        startAnalyze();
    });
    window.connect('map', maybeAutoAnalyze);
    apply.connect('activated', () => {
        if (!analysis || current().some((uri, i) => uri !== snapshot[i])) {
            apply.sensitive = false;
            draft.description = 'Duvar kâğıdı değişti. Önce paletleri yeniden hazırla.';
            return;
        }
        if (pick.selected >= profiles.length) return;
        const hues = analysis.map(item => (item && typeof item === 'object' ? item.hue : item));
        const next = {
            light: colors(analysis[0], pick.selected, false), dark: colors(analysis[1], pick.selected, true),
            profile: pick.selected, hues,
            ...(validAnalysis(analysis) ? {analysis} : {}),
            wallpapers: snapshot, revision: paletteRevision,
        };
        if (!settings.set_string('palette-colors', JSON.stringify(next))) {
            window.add_toast(new Adw.Toast({title: 'Palet kaydedilemedi; ayar yazmaya kapalı.'}));
            return;
        }
        sync();
        window.add_toast(new Adw.Toast({title: active.active ? 'Renk karakteri kaydedildi.' : 'Karakter kaydedildi. Kullanmak için Duvar temasını aç.'}));
    });

    window.add(page);
    window.connect('destroy', () => {
        closed = true;
        if (timeout) GLib.Source.remove(timeout);
        timeout = 0;
        proc?.force_exit();
        for (const [source, id] of subscriptions) source.disconnect(id);
        if (display)
            Gtk.StyleContext.remove_provider_for_display(display, provider);
    });
}
