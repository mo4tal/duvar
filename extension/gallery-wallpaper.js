import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk';

function canonical(path) {
    return new Promise((resolve, reject) => {
        const child = Gio.Subprocess.new(['realpath', '-ez', '--', path],
            Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_SILENCE);
        child.communicate_async(null, null, (self, result) => {
            try {
                const [, bytes] = self.communicate_finish(result);
                if (!self.get_successful()) throw new Error('Path could not be resolved');
                resolve(new TextDecoder().decode(bytes.get_data()).replace(/\0$/, ''));
            } catch (error) { reject(error); }
        });
    });
}

export function addWallpaperAction(group, window, settings, mode, title, isClosed, t = null) {
    const label = t ? t('colors.pick_for_theme', {title}) : `${title} için uygula`;
    const dialogTitle = t ? t('colors.choose_for_theme', {title}) : `${title} için görsel seç`;
    const action = new Adw.ButtonRow({title: label, start_icon_name: 'folder-open-symbolic'});
    action.connect('activated', () => {
        let config = {};
        try { config = JSON.parse(settings.get_string('configuration')) ?? {}; } catch {}
        const pictures = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_PICTURES) ||
            GLib.build_filenamev([GLib.get_home_dir(), 'Pictures']);
        const folder = Gio.File.new_for_path(config.local_directory || config.download_directory ||
            GLib.build_filenamev([pictures, 'Duvar']));
        const dialog = new Gtk.FileDialog({title: dialogTitle, initial_folder: folder, accept_label: action.title});
        const filter = new Gtk.FileFilter();
        filter.add_pixbuf_formats();
        const filters = new Gio.ListStore({item_type: Gtk.FileFilter});
        filters.append(filter);
        dialog.set_filters(filters);
        dialog.open(window, null, async (self, res) => {
            try {
                const file = self.open_finish(res);
                const [root, target] = file.get_path()
                    ? await Promise.all([canonical(folder.get_path()), canonical(file.get_path())]) : [];
                if (isClosed()) return;
                if (!root || !target.startsWith(`${root.replace(/\/$/, '')}/`)) {
                    window.add_toast(new Adw.Toast({title: 'Duvar klasörünün içinden bir görsel seç.'}));
                    return;
                }
                if (!window.activate_action('wallpaper.apply-file', new GLib.Variant('(ss)', [mode, target])))
                    throw new Error('Wallpaper action unavailable');
            } catch (e) {
                if (!isClosed() && !e.matches?.(Gtk.DialogError, Gtk.DialogError.DISMISSED))
                    window.add_toast(new Adw.Toast({title: 'Görsel seçilemedi.'}));
            }
        });
    });
    group.add(action);
}
