import Adw from 'gi://Adw?version=1';
import Gtk from 'gi://Gtk?version=4.0';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GIRepository from 'gi://GIRepository?version=3.0';
const directory=ARGV[0];
Adw.init();
Gio.Resource.load('/usr/share/gnome-shell/org.gnome.Shell.Extensions.src.gresource')._register();
const repo=GIRepository.Repository.dup_default();
repo.prepend_search_path(`${directory}/native`);repo.prepend_library_path(`${directory}/native`);
const {default: Preferences}=await import(`file://${directory}/prefs.js`);
const metadata=JSON.parse(new TextDecoder().decode(GLib.file_get_contents(`${directory}/metadata.json`)[1]));
const preferences=new Preferences({...metadata,metadata,path:directory,dir:Gio.File.new_for_path(directory)});
const app=new Adw.Application({application_id:'local.mo4tal.Duvar.Reopen',flags:Gio.ApplicationFlags.NON_UNIQUE});
let failure=null;
function widgets(w){let a=[w];for(let c=w.get_first_child();c;c=c.get_next_sibling())a.push(...widgets(c));return a;}
app.connect('activate',()=>{
 const window=new Adw.PreferencesWindow({application:app});
 preferences.fillPreferencesWindow(window).then(()=>{
  const all=widgets(window);
  for(const [title,value] of [['Resim klasörü',GLib.getenv('DUVAR_REOPEN_DIRECTORY')],['İndirme klasörü',GLib.getenv('DUVAR_REOPEN_DOWNLOADS')]]){
   const row=all.find(w=>w instanceof Adw.ActionRow && w.title===title);
   if(row?.subtitle!==value)throw Error('Cross-process setting lost: '+title);
  }
  const sorting=all.find(w=>w instanceof Gtk.DropDown);
  if(sorting?.selected!==2)throw Error('Gallery sort setting lost');
  print(JSON.stringify({fresh_process_persistence:true,folders:true,sort:true}));
  window.close();app.quit();
 }).catch(e=>{failure=e;printerr(String(e)+'\n'+e.stack);window.close();app.quit();});
});
await app.runAsync([]);
if(failure)throw failure;
