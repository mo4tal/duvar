import Adw from 'gi://Adw?version=1';
import Gtk from 'gi://Gtk?version=4.0';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GIRepository from 'gi://GIRepository?version=3.0';
const directory=ARGV[0];Adw.init();
Gio.Resource.load('/usr/share/gnome-shell/org.gnome.Shell.Extensions.src.gresource')._register();
const repo=GIRepository.Repository.dup_default();repo.prepend_search_path(`${directory}/native`);repo.prepend_library_path(`${directory}/native`);
const {default: Preferences}=await import(`file://${directory}/prefs.js`);
const metadata=JSON.parse(new TextDecoder().decode(GLib.file_get_contents(`${directory}/metadata.json`)[1]));
const preferences=new Preferences({...metadata,metadata,path:directory,dir:Gio.File.new_for_path(directory)});
const app=new Adw.Application({application_id:'io.github.mo4tal.Duvar.Queue',flags:Gio.ApplicationFlags.NON_UNIQUE});
let failure=null;
function widgets(w){let a=[w];for(let c=w.get_first_child();c;c=c.get_next_sibling())a.push(...widgets(c));return a;}
app.connect('activate',()=>{
 const window=new Adw.PreferencesWindow({application:app});
 preferences.fillPreferencesWindow(window).then(()=>{
  window.present();let ticks=0;
  const page=widgets(window).find(w=>w instanceof Adw.PreferencesPage&&w.title==='İndirilenler');
  if(!page)throw Error('Missing downloaded tab');window.set_visible_page(page);
  GLib.timeout_add(GLib.PRIORITY_DEFAULT,100,()=>{
   try{
    if(++ticks>60)throw Error('Persistent queue test timed out');
    const path=GLib.getenv('XDG_STATE_HOME')+'/duvar/download-queue.json';
    const saved=JSON.parse(new TextDecoder().decode(GLib.file_get_contents(path)[1]));
    const queue=Array.isArray(saved)?saved:saved.jobs;
    if(queue.filter(q=>q.status==='done').length!==2)return GLib.SOURCE_CONTINUE;
    const grid=widgets(page).find(w=>w instanceof Gtk.GridView);
    if(grid.get_model().get_n_items()<2)return GLib.SOURCE_CONTINUE;
    if(!widgets(grid).some(w=>w instanceof Gtk.Picture&&w.get_paintable()!==null))return GLib.SOURCE_CONTINUE;
    for(const item of queue){if(!Gio.File.new_for_path(item.path).query_exists(null))throw Error('Resumed file absent');if(Gio.File.new_for_path(item.path+'.json').query_exists(null))throw Error('Resumed file has sidecar');}
    const latest=queue.find(q=>q.id===1002);
    const bg=Gio.Settings.new('org.gnome.desktop.background');
    if(['picture-uri','picture-uri-dark'].some(key=>bg.get_string(key)!==Gio.File.new_for_path(latest.path).get_uri()))throw Error('Older queued apply replaced latest choice');
    grid.emit('activate',0);
    const preview=window.get_visible_dialog();
    if(!preview)throw Error('Downloaded preview missing');
    for(const label of ['İndir','İndir ve Uygula','Sadece Uygula'])if(widgets(preview).some(w=>w instanceof Gtk.Button&&w.label===label))throw Error('Downloaded image still offers '+label);
    for(const label of ['Açık tema için uygula','Koyu tema için uygula'])if(!widgets(preview).some(w=>w instanceof Gtk.Button&&w.get_tooltip_text()===label))throw Error('Downloaded apply missing: '+label);
    preview.close();
    print(JSON.stringify({downloaded_actions:true,latest_apply_wins:true,persistent_queue:true,resumed:2,downloaded_tab:true,downloaded_items:grid.get_model().get_n_items(),local_thumbnails:true}));
    window.close();app.quit();return GLib.SOURCE_REMOVE;
   }catch(e){failure=e;printerr(String(e)+'\n'+e.stack);window.close();app.quit();return GLib.SOURCE_REMOVE;}
  });
 }).catch(e=>{failure=e;printerr(String(e)+'\n'+e.stack);window.close();app.quit();});
});
await app.runAsync([]);if(failure)throw failure;
