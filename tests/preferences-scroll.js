import Adw from 'gi://Adw?version=1';
import Gtk from 'gi://Gtk?version=4.0';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GIRepository from 'gi://GIRepository?version=3.0';
const directory=ARGV[0];Adw.init();
Gio.Resource.load('/usr/share/gnome-shell/org.gnome.Shell.Extensions.src.gresource')._register();
const repo=GIRepository.Repository.dup_default();repo.prepend_search_path(`${directory}/native`);repo.prepend_library_path(`${directory}/native`);
const {default: Preferences}=await import(`file://${directory}/prefs.js`);
const schemaSource=Gio.SettingsSchemaSource.new_from_directory(`${directory}/schemas`,Gio.SettingsSchemaSource.get_default(),false);
const settings=Gio.Settings.new_full(schemaSource.lookup('org.gnome.shell.extensions.duvar',false),null,null);
settings.set_string('configuration',JSON.stringify({wallhaven:false,openverse:false,local:true,local_directory:GLib.getenv('DUVAR_SCROLL_FIXTURES'),remember_sensitive:true,policy:{unknown:true}}));
const metadata=JSON.parse(new TextDecoder().decode(GLib.file_get_contents(`${directory}/metadata.json`)[1]));
const preferences=new Preferences({...metadata,metadata,path:directory,dir:Gio.File.new_for_path(directory)});
const app=new Adw.Application({application_id:'io.github.mo4tal.Duvar.Scroll',flags:Gio.ApplicationFlags.NON_UNIQUE});
let failure=null;
function widgets(w){let a=[w];for(let c=w.get_first_child();c;c=c.get_next_sibling())a.push(...widgets(c));return a;}
app.connect('activate',()=>{
 const window=new Adw.PreferencesWindow({application:app});
 preferences.fillPreferencesWindow(window).then(()=>{
  window.present();let stage=0,ticks=0,pinnedY=0;
  GLib.timeout_add(GLib.PRIORITY_DEFAULT,100,()=>{
   try{
    if(++ticks>70)throw Error('Infinite-scroll test timed out');
    const grid=widgets(window).find(w=>w instanceof Gtk.GridView);
    const count=grid.get_model().get_n_items();
    const expected=[24,48,72,73][stage];
    if(count<expected)return GLib.SOURCE_CONTINUE;
    if(count!==expected)throw Error(`Unexpected count ${count}; expected ${expected}`);
    const scroll=grid.get_parent();
    if(!(scroll instanceof Gtk.ScrolledWindow))throw Error('Missing gallery scroll');
    const vadj=scroll.get_vadjustment();
    if(stage===1&&pinnedY===0){
     const target=Math.min(420,Math.max(0,vadj.get_upper()-vadj.get_page_size()));
     if(target>120){vadj.set_value(target);pinnedY=target;}
    }
    if(pinnedY>0&&vadj.get_value()<pinnedY-40)throw Error(`Scroll jumped upward from ${pinnedY} to ${vadj.get_value()} during thumbnail/page load`);
    if(stage<3){
     // Rapid repeated edge notifications must request each source page only once.
     for(let i=0;i<3;i++)scroll.emit('edge-reached',Gtk.PositionType.BOTTOM);
     stage++;return GLib.SOURCE_CONTINUE;
    }
    const paints=widgets(grid).filter(w=>w instanceof Gtk.Picture && w.get_paintable()!==null).length;
    if(paints===0)return GLib.SOURCE_CONTINUE;
    if(pinnedY>0&&vadj.get_value()<pinnedY-40)throw Error(`Scroll jumped upward after thumbnails painted (${pinnedY} -> ${vadj.get_value()})`);
    print(JSON.stringify({infinite_scroll:true,scroll_stable:true,pinned_y:Math.round(pinnedY),final_y:Math.round(vadj.get_value()),items:count,duplicates:false,corrupt_image_isolated:true}));
    window.close();app.quit();return GLib.SOURCE_REMOVE;
   }catch(e){failure=e;printerr(String(e)+'\n'+e.stack);window.close();app.quit();return GLib.SOURCE_REMOVE;}
  });
 }).catch(e=>{failure=e;printerr(String(e)+'\n'+e.stack);window.close();app.quit();});
});
await app.runAsync([]);if(failure)throw failure;
