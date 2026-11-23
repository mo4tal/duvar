import Gdk from 'gi://Gdk';
import Gtk from 'gi://Gtk';
import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';
import {addPalettePage} from './palette-prefs.js';

export default class DuvarPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const display = Gdk.Display.get_default();
        if (display)
            Gtk.IconTheme.get_for_display(display).add_search_path(`${this.path}/icons`);
        window.set_icon_name('io.github.mo4tal.Duvar');
        addPalettePage(window, this.getSettings(), this.path);
    }
}
