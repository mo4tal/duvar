// Runs in the private Wayland session without GTK_THEME overrides.
import GLib from 'gi://GLib';

const [kind, socket, probeId = kind] = ARGV;
GLib.setenv('WAYLAND_DISPLAY', socket, true);
const gtk3 = kind === '3';
const adw = kind === 'adw';
const {default: Gtk} = gtk3 ? await import('gi://Gtk?version=3.0') :
    await import('gi://Gtk?version=4.0');
const Adw = adw ? (await import('gi://Adw')).default : null;
if (adw) Adw.init();
else if (gtk3) Gtk.init(null);
else Gtk.init();

const window = adw ? new Adw.Window() : new Gtk.Window();
if (probeId.startsWith('overlap-')) {
    window.set_title(`Duvar ${probeId}`);
    window.set_default_size(490, 320);
    if (!adw) window.set_titlebar(new Gtk.HeaderBar());
}
const button = new Gtk.Button({label: 'Duvar GTK probe'});
button.get_style_context().add_class('suggested-action');
const accentProbe = adw ? new Gtk.Label({label: 'Custom accent'}) : null;
if (adw) {
    const provider = new Gtk.CssProvider();
    provider.load_from_string('label.duvar-accent-probe { color: var(--accent-color); }');
    Gtk.StyleContext.add_provider_for_display(window.get_display(), provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION);
    accentProbe.add_css_class('duvar-accent-probe');
}
if (gtk3) {
    window.add(button);
    window.show_all();
} else {
    if (adw) {
        const box = new Gtk.Box({orientation: Gtk.Orientation.VERTICAL});
        if (probeId.startsWith('overlap-')) box.append(new Adw.HeaderBar());
        box.append(button);
        box.append(accentProbe);
        window.set_content(box);
    }
    else if (probeId.startsWith('overlap-')) {
        const box = new Gtk.Box({orientation: Gtk.Orientation.VERTICAL});
        box.append(button);
        window.set_child(box);
    } else window.set_child(button);
    window.present();
}

const color = widget => (gtk3 ? widget.get_style_context().get_color(0) :
    widget.get_color()).to_string();
GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
    const value = `${Gtk.Settings.get_default().gtk_theme_name} ${color(window)} ${color(button)}`;
    GLib.file_set_contents(`${GLib.get_user_cache_dir()}/gtk-${probeId}`, value);
    if (adw) GLib.file_set_contents(`${GLib.get_user_cache_dir()}/gtk-${probeId}-accent`, color(accentProbe));
    if (adw) GLib.file_set_contents(`${GLib.get_user_cache_dir()}/gtk-${probeId}-contrast`,
        String(Adw.StyleManager.get_default().high_contrast));
    return GLib.SOURCE_CONTINUE;
});
new GLib.MainLoop(null, false).run();
