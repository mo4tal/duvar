// Run in an isolated environment with compiled extension schemas and a GTK display.
import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk?version=4.0';
import {addPalettePage} from '../extension/palette-prefs.js';
import {profiles, colors, validColors, nativeAccent} from '../extension/palette.js';

function assert(ok, message) {
    if (!ok) throw new Error(message);
}
const [path, fixture] = ARGV;
const source = Gio.SettingsSchemaSource.new_from_directory(`${path}/schemas`,
    Gio.SettingsSchemaSource.get_default(), false);
const settings = new Gio.Settings({
    settings_schema: source.lookup('org.gnome.shell.extensions.duvar', true),
    backend: Gio.memory_settings_backend_new(),
});
assert(!settings.get_boolean('palette-enabled'), 'Theme must start disabled');
for (let hue = 0; hue < 360; hue += 15) {
    const pairs = profiles.map((_, i) => [colors(hue, i, false), colors(hue, i, true)]);
    assert(new Set(pairs.map(p => p[0].accent)).size === 10, 'Expected 10 distinct profiles');
    assert(pairs.flat().every(c => validColors(c) && nativeAccent(c.accent)), 'Invalid color');
}
const bg = new Gio.Settings({schema_id: 'org.gnome.desktop.background'});
const uri = Gio.File.new_for_path(fixture).get_uri();
bg.set_string('picture-uri', uri);
bg.set_string('picture-uri-dark', uri);
Adw.init();
const window = new Adw.PreferencesWindow();
addPalettePage(window, settings, path);
window.present();

function widgets(w) {
    const all = [w];
    for (let child = w.get_first_child(); child; child = child.get_next_sibling())
        all.push(...widgets(child));
    return all;
}
const all = widgets(window);
const row = title => all.find(w => w instanceof Adw.PreferencesRow && w.title === title);
const prepare = row('10 paleti hazırla'), save = row('Seçili paleti kaydet'), picker = row('Palet');
assert(prepare && save && picker?.model.get_n_items() === 10 && !save.sensitive, 'Unexpected initial page state');
prepare.emit('activated');
const loop = new GLib.MainLoop(null, false);
let result = '';
GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, () => {
    if (!prepare.sensitive) return GLib.SOURCE_CONTINUE;
    if (save.sensitive) {
        picker.selected = 7;
        save.emit('activated');
        result = settings.get_string('palette-colors');
    }
    loop.quit();
    return GLib.SOURCE_REMOVE;
});
loop.run();
const pair = JSON.parse(result || '{}');
assert(validColors(pair.light) && validColors(pair.dark), 'Expected a saved light/dark pair');
assert(pair.hues?.length === 2 && pair.hues.every(hue => hue === null), 'Expected neutral analysis in both modes');
assert(!settings.get_boolean('palette-enabled'), 'Saving must not enable theme');
window.destroy();
bg.set_string('picture-uri', Gio.File.new_for_path(`${fixture}.changed`).get_uri());
const reopened = new Adw.PreferencesWindow();
addPalettePage(reopened, settings, path);
const restored = widgets(reopened).find(w => w instanceof Adw.ComboRow && w.title === 'Palet');
assert(restored?.selected === 7, 'Saved profile not restored');
const preview = widgets(reopened).find(w => w instanceof Adw.PreferencesGroup &&
    w.title === 'Son kayıt' && w.visible);
const previewCards = preview ? widgets(preview).filter(w => w.has_css_class?.('duvar-preview-card')) : [];
assert(previewCards.length === 2 && previewCards.every(card => {
    const labels = widgets(card).filter(w => w instanceof Gtk.Label).map(w => w.label);
    return ['Kart', 'İkincil', 'Pasif', 'Etkin', 'Seçim', 'Odak'].every(role => labels.includes(role));
}), 'Saved light/dark role previews not restored');
reopened.destroy();
print('PASS: 10 profiles, GTK preferences, worker, save both modes while disabled');
