// The fixture has deliberately mismatched on-disk metadata. The guard must still reject it.
import Adw from 'gi://Adw?version=1';
import Gio from 'gi://Gio';
import GIRepository from 'gi://GIRepository?version=3.0';
Adw.init();
const directory=ARGV[0];
Gio.Resource.load('/usr/share/gnome-shell/org.gnome.Shell.Extensions.src.gresource')._register();
const repo=GIRepository.Repository.dup_default();
repo.prepend_search_path(`${directory}/native`);repo.prepend_library_path(`${directory}/native`);
const {default:Duvar}=await import('gi://Duvar?version=1.0');
const {default:Preferences}=await import(`file://${directory}/prefs.js`);
const metadata=JSON.parse(new TextDecoder().decode(Gio.File.new_for_path(`${directory}/metadata.json`).load_contents(null)[1]));
const cached={...metadata,'version-name':'0.0.0-stale-shell-cache'};
const prefs=new Preferences({...cached,metadata:cached,path:directory,dir:Gio.File.new_for_path(directory)});
const app=new Adw.Application({application_id:'local.mo4tal.Duvar.VersionGuard',flags:Gio.ApplicationFlags.NON_UNIQUE});
let failure;
app.connect('activate',()=>{app.hold();(async()=>{
 const window=new Adw.PreferencesWindow({application:app});
 try{
  let rejected=false;
  try{await prefs.fillPreferencesWindow(window);}catch(e){rejected=String(e).includes('999.0.0')&&String(e).includes(Duvar.version());}
  if(!rejected)throw Error('A genuine package/native version mismatch was not diagnosed');
  print(JSON.stringify({real_version_mismatch_rejected:true}));
 }catch(e){failure=e;printerr(String(e));}
 finally{window.close();app.release();app.quit();}
})().catch(e=>{failure=e;app.release();app.quit();});});
await app.runAsync([]);if(failure)throw failure;
