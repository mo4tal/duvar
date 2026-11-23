// Loaded only by palette-shell-smoke.sh in its private GNOME Shell.
import Gio from 'gi://Gio';
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as MessageTray from 'resource:///org/gnome/shell/ui/messageTray.js';
import {QuickSlider} from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import {samplePerformance} from './palette-perf-probe.js';

export default class RenderProbe extends Extension {
    enable() {
        const base = GLib.get_user_cache_dir();
        const request = Gio.File.new_for_path(`${base}/capture`);
        const gallery = Gio.File.new_for_path(`${base}/gallery`);
        const closeWindow = Gio.File.new_for_path(`${base}/close-window`);
        const pulseArm = Gio.File.new_for_path(`${base}/arm-pulse-disable`);
        const perfArm = Gio.File.new_for_path(`${base}/arm-performance`);
        const a11y = new Gio.Settings({schema_id: 'org.gnome.desktop.a11y.interface'});
        this._seat = global.stage.get_context().get_backend().get_default_seat();
        this._a11y = a11y;
        this._pulseWatch = a11y.connect('changed::high-contrast', () => {
            const arm = Gio.File.new_for_path(`${base}/arm-pulse-frame`);
            if (!a11y.get_boolean('high-contrast') || !arm.query_exists(null)) return;
            arm.delete(null);
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 110, () => {
                this._screenshot(base, 'flatpak-pulse').catch(logError);
                return GLib.SOURCE_REMOVE;
            });
        });
        this._source = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 150, () => {
            if (perfArm.query_exists(null)) {
                const [, bytes] = perfArm.load_contents(null);
                perfArm.delete(null);
                samplePerformance(base, new TextDecoder().decode(bytes).trim());
            }
            if (pulseArm.query_exists(null)) {
                pulseArm.delete(null);
                const id = a11y.connect('changed::high-contrast', () => {
                    if (!a11y.get_boolean('high-contrast')) return;
                    a11y.disconnect(id);
                    GLib.idle_add(GLib.PRIORITY_HIGH_IDLE, () => {
                        const uuid = 'duvar@mo4tal.local';
                        const instance = Main.extensionManager.lookup(uuid).stateObj;
                        const disable = instance.disable;
                        instance.disable = function () {
                            this.disable = disable;
                            const pulsing = Boolean(this._contrastPulse);
                            disable.call(this);
                            GLib.file_set_contents(`${base}/pulse-disable`, `${pulsing} ${a11y.get_boolean('high-contrast')}`);
                        };
                        Main.extensionManager.disableExtension(uuid);
                        return GLib.SOURCE_REMOVE;
                    });
                });
                GLib.file_set_contents(`${base}/pulse-armed`, '');
            }
            if (gallery.query_exists(null)) {
                const fallback = global.get_window_actors().find(actor =>
                    (actor.meta_window.get_window_type() === Meta.WindowType.NORMAL ||
                        [Meta.WindowType.DIALOG, Meta.WindowType.MODAL_DIALOG].includes(actor.meta_window.get_window_type()) &&
                        actor.meta_window.get_title() === 'Duvar') &&
                    !actor.meta_window.minimized);
                if (closeWindow.query_exists(null)) {
                    const [, bytes] = closeWindow.load_contents(null);
                    closeWindow.delete(null);
                    const id = new TextDecoder().decode(bytes).trim();
                    if (String(fallback?.meta_window.get_id()) === id)
                        fallback.meta_window.delete(global.get_current_time());
                }
                if (fallback && Main.overview.visible)
                    Main.overview.hide();
                const frame = !Main.overview.visible && fallback?.mapped ?
                    fallback.meta_window.get_frame_rect() : null;
                this._galleryWindow = frame ? fallback.meta_window : null;
                const geometry = frame ? `${frame.x} ${frame.y} ${frame.width} ${frame.height}` : '-';
                if (this._geometry !== geometry) {
                    this._geometry = geometry;
                    GLib.file_set_contents(`${base}/window`, `${geometry}\n`);
                }
                const windowId = frame ? String(fallback.meta_window.get_id()) : '-';
                if (this._windowId !== windowId) {
                    this._windowId = windowId;
                    GLib.file_set_contents(`${base}/window-id`, windowId);
                }
            }
            if (!request.query_exists(null)) return GLib.SOURCE_CONTINUE;
            const [, bytes] = request.load_contents(null);
            request.delete(null);
            const [mode, action, x, y, button] = new TextDecoder().decode(bytes).trim().split(/\s+/);
            if (!/^[a-z0-9-]+$/.test(mode)) return GLib.SOURCE_CONTINUE;
            if (mode.startsWith('surface-') && Main.overview.visible)
                Main.overview.hide();
            if (mode === 'surface-dark-quick') {
                // The headless test session has no audio output or backlight.
                this._testSlider = new QuickSlider({iconName: 'audio-volume-medium-symbolic'});
                this._testSlider.slider.value = 0.5;
                Main.panel.statusArea.quickSettings.menu.addItem(this._testSlider, 2);
            }
            const date = action?.startsWith('date');
            const weather = action === 'weather';
            const menu = weather ? Main.extensionManager.lookup('weatherpanel@attentivecoder')
                ?.stateObj?._controller?._panelButton?.menu : date ? Main.panel.statusArea.dateMenu.menu :
                ['quick', 'quick-keep', 'quick-existing'].includes(action) || ['light', 'dark'].includes(mode) ?
                    Main.panel.statusArea.quickSettings.menu : null;
            if (action === 'quick-existing' && !menu.isOpen)
                GLib.file_set_contents(`${base}/${mode}.error`, 'Quick Settings closed during the change\n');
            if (weather && !menu)
                GLib.file_set_contents(`${base}/${mode}.error`, 'WeatherPanel menu unavailable\n');
            if (!menu) {
                Main.panel.statusArea.dateMenu.menu.close({animate: false});
                Main.panel.statusArea.quickSettings.menu.close({animate: false});
            }
            const overlap = action === 'overlap-4' || action === 'overlap-adw';
            const overlapWindows = () => ['4', 'adw'].map(kind =>
                global.get_window_actors().map(a => a.meta_window)
                    .find(w => w.get_title() === `Duvar overlap-${kind}`));
            if (overlap) {
                const [gtk, adw] = overlapWindows();
                if (gtk && adw) {
                    gtk.move_frame(false, 250, 170);
                    adw.move_frame(false, 460, 255);
                    (action === 'overlap-4' ? gtk : adw).activate(global.get_current_time());
                }
            }
            if (action === 'click' || action === 'select-line') {
                this._galleryWindow?.activate(global.get_current_time());
                const px = Number(x);
                const py = Number(y);
                const mouseButton = Number(button ?? 1);
                const after = (delay, fn) => GLib.timeout_add(GLib.PRIORITY_DEFAULT, delay, () => {
                    fn();
                    return GLib.SOURCE_REMOVE;
                });
                after(80, () => this._move(px, py));
                after(160, () => this._pointer.notify_button(GLib.get_monotonic_time(), mouseButton, Clutter.ButtonState.PRESSED));
                if (action === 'select-line') after(240, () => this._move(px + 275, py));
                after(320, () => this._pointer.notify_button(GLib.get_monotonic_time(), mouseButton, Clutter.ButtonState.RELEASED));
            }
            if (menu && !menu.isOpen) menu.open();
            if (action === 'notification') {
                // Exercise the real banner without relying on the isolated
                // session's optional freedesktop notification bridge. Keep
                // the pointer off the banner to measure its normal state.
                this._move(100, 300);
                const source = MessageTray.getSystemSource();
                this._notification = new MessageTray.Notification({
                    source, title: 'Duvar', body: 'Duvar kâğıdından gelen etkin renk ailesi.',
                    isTransient: true,
                });
                source.addNotification(this._notification);
            }
            const clockCard = actor => actor.has_style_class_name?.('world-clocks-button') ? actor :
                actor.get_children().map(clockCard).find(Boolean);
            if (action === 'date-hover' || action === 'date-focus') {
                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 250, () => {
                    const card = clockCard(menu.actor);
                    if (!card) return GLib.SOURCE_REMOVE;
                    if (action === 'date-focus') {
                        this._move(100, 100);
                        card.grab_key_focus();
                    } else {
                        const [cx, cy] = card.get_transformed_position();
                        const [width, height] = card.get_transformed_size();
                        this._move(cx + width / 2, cy + height / 2);
                    }
                    return GLib.SOURCE_REMOVE;
                });
            }
            this._capture = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 550, () => {
                this._capture = 0;
                if (overlap) {
                    const rows = overlapWindows().map(w => {
                        if (!w) return null;
                        const {x, y, width, height} = w.get_frame_rect();
                        return {id: w.get_id(), x, y, width, height, focused: w.has_focus()};
                    });
                    GLib.file_set_contents(`${base}/${mode}.geometry`, JSON.stringify(rows));
                }
                if (action === 'click' &&
                    (Main.panel.statusArea.dateMenu.menu.isOpen || Main.panel.statusArea.quickSettings.menu.isOpen)) {
                    console.warn(`Duvar render ${mode}: click (${x}, ${y}) opened a Shell menu`);
                    GLib.file_set_contents(`${base}/${mode}.error`, 'Shell menu opened instead of app menu\n');
                }
                const lineage = actor => {
                    const classes = [];
                    for (let node = actor; node; node = node.get_parent())
                        classes.push(node.get_style_class_name?.() ?? '');
                    return classes.join(' < ');
                };
                const inspect = actor => {
                    const classes = actor.get_style_class_name?.() ?? '';
                    const has = name => actor.has_style_class_name?.(name);
                    if (mode.startsWith('surface-') && actor.mapped &&
                        ['avatar-username-label', 'avatar-realname-label', 'quick-toggle', 'slider',
                         'calendar-month-label', 'message-list-placeholder', 'datemenu-today-button'].some(has)) {
                        const node = actor.get_theme_node();
                        const color = property => {
                            const [found, value] = node.lookup_color(property, true);
                            return found ? value.to_string() : '-';
                        };
                        const role = has('quick-toggle') ? (actor.has_style_pseudo_class('checked') ? 'active' : 'passive') :
                            classes.split(' ').find(name => [
                                'avatar-username-label', 'avatar-realname-label', 'slider',
                                'calendar-month-label', 'message-list-placeholder', 'datemenu-today-button',
                            ].includes(name));
                        const bars = role === 'slider' ?
                            `${color('-barlevel-active-background-color')} ${color('-barlevel-background-color')}` : '- -';
                        console.log(`Duvar role ${mode} ${role} ${node.get_background_color().to_string()} ${node.get_foreground_color().to_string()} ${bars}`);
                    }
                    if (mode.startsWith('surface-') && actor.mapped &&
                        /(?:^| )(?:bms-popup-blurred|popup-menu-content|quick-settings|notification-banner|weatherpanel-(?:hero-summary|status-label|offline-label|forecast-loading))(?: |$)/.test(classes)) {
                        const node = actor.get_theme_node();
                        const bounds = [...actor.get_transformed_position(), ...actor.get_transformed_size()].map(Math.round);
                        console.log(`Duvar surface ${mode} ${node.get_background_color().to_string()} ${node.get_foreground_color().to_string()} ${bounds.join(',')} ${lineage(actor)}`);
                    }
                    if (actor.mapped && actor.has_style_class_name?.('quick-toggle') &&
                        actor.has_style_pseudo_class('checked')) {
                        console.log(`Duvar render ${actor.get_style_pseudo_class()} ${actor.get_theme_node().get_background_color().to_string()} ${lineage(actor)}`);
                    }
                    if (date && actor.mapped &&
                        ['world-clocks-button', 'weather-button', 'events-button'].some(name => actor.has_style_class_name?.(name))) {
                        const node = actor.get_theme_node();
                        console.log(`Duvar date ${mode} ${actor.get_style_class_name()} ${node.get_background_color().to_string()}`);
                        if (action === 'date-focus' && actor === global.stage.get_key_focus()) {
                            const shadow = node.get_box_shadow();
                            if (shadow) console.log(`Duvar focus ${mode} ${shadow.color.to_string()} ${shadow.inset} ${shadow.spread}`);
                        }
                    }
                    if (action === 'notification' && actor.mapped &&
                        actor.has_style_class_name?.('notification-banner')) {
                        console.log(`Duvar notification ${mode} ${actor.get_theme_node().get_background_color().to_string()} ${lineage(actor)}`);
                    }
                    actor.get_children().forEach(inspect);
                };
                if (menu) inspect(menu.actor);
                else if (action === 'notification') inspect(global.stage);
                if (menu && !date) {
                    const panel = Main.panel;
                    const button = Main.panel.statusArea.quickSettings;
                    const activities = Main.panel.statusArea.activities;
                    const ov = panel.has_style_pseudo_class('overview') ? 'overview=yes' : 'overview=no';
                    const pillColor = actor => {
                        const node = actor.get_theme_node();
                        const shadow = node.get_box_shadow();
                        return shadow?.inset && shadow.spread >= 50 ? shadow.color.to_string() : node.get_background_color().to_string();
                    };
                    const actBg = activities ? pillColor(activities) : '-';
                    const btnNode = button.get_theme_node();
                    const btnBg = btnNode.get_background_color().to_string();
                    const btnShadow = btnNode.get_box_shadow();
                    const btnPill = btnShadow?.inset && btnShadow.spread >= 50 ? btnShadow.color.to_string() : 'none';
                    console.log(`Duvar panel ${mode} ${panel.get_style_class_name()} ${panel.get_theme_node().get_background_color().to_string()} ${ov} activities=${actBg} ${button.get_style_pseudo_class()} bg=${btnBg} ${btnPill}`);
                }
                this._screenshot(base, mode, weather || action === 'date-focus' || (!date && action !== 'quick-keep') ?
                    menu : null).catch(logError);
                return GLib.SOURCE_REMOVE;
            });
            return GLib.SOURCE_CONTINUE;
        });
    }

    _move(x, y) {
        this._pointer ??= this._seat.create_virtual_device(Clutter.InputDeviceType.POINTER_DEVICE);
        this._pointer.notify_absolute_motion(GLib.get_monotonic_time(), x, y);
    }

    async _screenshot(base, mode, menu) {
        const stream = Gio.File.new_for_path(`${base}/${mode}.png`).replace(null, false, Gio.FileCreateFlags.NONE, null);
        try {
            await new Shell.Screenshot().screenshot(false, stream);
        } finally {
            stream.close(null);
            menu?.close({animate: false});
            if (mode.endsWith('-notification')) {
                this._notification?.destroy();
                this._notification = null;
            }
        }
        GLib.file_set_contents(`${base}/${mode}.done`, '');
    }

    disable() {
        this._testSlider?.destroy();
        this._testSlider = null;
        GLib.Source.remove(this._source);
        if (this._capture) GLib.Source.remove(this._capture);
        this._notification?.destroy();
        this._a11y.disconnect(this._pulseWatch);
    }
}
