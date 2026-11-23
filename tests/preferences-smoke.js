import Adw from 'gi://Adw?version=1';
import Gtk from 'gi://Gtk?version=4.0';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GIRepository from 'gi://GIRepository?version=3.0';
const directory=ARGV[0];Adw.init();
Gio.Resource.load('/usr/share/gnome-shell/org.gnome.Shell.Extensions.src.gresource')._register();
const repo=GIRepository.Repository.dup_default();repo.prepend_search_path(`${directory}/native`);repo.prepend_library_path(`${directory}/native`);
const {default: Preferences}=await import(`file://${directory}/prefs.js`);
const source=Gio.SettingsSchemaSource.new_from_directory(`${directory}/schemas`,Gio.SettingsSchemaSource.get_default(),false);
Gio.Settings.new_full(source.lookup('org.gnome.shell.extensions.duvar',false),null,null).set_string('configuration',GLib.getenv('DUVAR_TEST_CONFIGURATION'));
const background=Gio.Settings.new('org.gnome.desktop.background');
const original=GLib.getenv('DUVAR_TEST_ORIGINAL'), originalDark=GLib.getenv('DUVAR_TEST_ORIGINAL_DARK');
background.set_string('picture-uri',original);background.set_string('picture-uri-dark',originalDark);
function checkPair(light,dark){if(background.get_string('picture-uri')!==light||background.get_string('picture-uri-dark')!==dark)throw Error('Unexpected light/dark wallpaper pair');}
const app=new Adw.Application({application_id:'io.github.mo4tal.Duvar.Smoke',flags:Gio.ApplicationFlags.NON_UNIQUE});let failed=false;
function widgets(w){const a=[w];for(let c=w.get_first_child();c;c=c.get_next_sibling())a.push(...widgets(c));return a;}
function delay(ms){return new Promise(resolve=>GLib.timeout_add(GLib.PRIORITY_DEFAULT,ms,()=>{resolve();return GLib.SOURCE_REMOVE;}));}
async function wait(check,message){for(let i=0;i<60;i++){if(check())return;await delay(50);}throw Error(message);}
function button(w,label){const b=widgets(w).find(w=>w instanceof Gtk.Button&&(w.label===label||w.get_child() instanceof Gtk.Box&&widgets(w.get_child()).some(c=>c instanceof Gtk.Label&&c.label===label)));if(!b)throw Error('Missing button '+label);return b;}
async function snapshot(window,suffix=''){for(let i=0;i<10;i++){const widget=window;const p=new Gtk.WidgetPaintable({widget});const snap=new Gtk.Snapshot();p.snapshot(snap,widget.get_width(),widget.get_height());const node=snap.to_node();if(node){window.get_renderer().render_texture(node,null).save_to_png(GLib.getenv('DUVAR_TEST_SCREENSHOT').replace('.png',suffix+'.png'));return;}await delay(50);}print('Snapshot unavailable for Cairo: '+suffix);}
function saved(){return JSON.parse(new TextDecoder().decode(GLib.file_get_contents(GLib.getenv('XDG_CONFIG_HOME')+'/duvar/preferences.json')[1]));}
app.connect('activate',()=>{app.hold();(async()=>{
 const metadata={...JSON.parse(new TextDecoder().decode(GLib.file_get_contents(`${directory}/metadata.json`)[1])), 'version-name':'0.0.0-stale-shell-cache'};
 const prefs=new Preferences({...metadata,metadata,path:directory,dir:Gio.File.new_for_path(directory)});
 const window=new Adw.PreferencesWindow({application:app});
 try{
  await prefs.fillPreferencesWindow(window);window.present();print("test renderer: "+window.get_renderer().constructor.name);
  widgets(window).find(w=>w instanceof Gtk.ToggleButton&&w.get_tooltip_text()?.startsWith('Favoriler')).set_active(true);
  const grid=widgets(window).find(w=>w instanceof Gtk.GridView);
  await wait(()=>grid.get_model().get_n_items()===12&&widgets(grid).some(w=>w instanceof Gtk.Picture&&w.get_height()>10&&w.get_paintable()),'Gallery did not render');await delay(200);await snapshot(window);
  const sorting=widgets(window).find(w=>w instanceof Gtk.DropDown);sorting.set_selected(2);
  await wait(()=>saved().gallery_order==='resolution','Sort setting did not persist');await delay(200);
  if(grid.get_parent().get_vadjustment().get_value()>1)throw Error('Sort did not return to the first result');
  window.set_default_size(480,700);await delay(300);if(grid.get_width()>600)throw Error('Narrow layout did not adapt');await snapshot(window,'-narrow');
  button(grid,'Açığa uygula');button(grid,'Koyuya uygula');
  grid.emit('activate',0);await wait(()=>window.get_visible_dialog()!==null,'No preview');const preview=window.get_visible_dialog();
  await wait(()=>widgets(preview).some(w=>w instanceof Gtk.Picture&&w.get_paintable()?.get_intrinsic_width()===1800),'Preview did not load original 1800px image');
  for(const label of ['İndir','İndir ve Uygula','Sadece Uygula'])if(widgets(preview).some(w=>w instanceof Gtk.Button&&w.label===label))throw Error('Local image exposes download action: '+label);
  const zoom=widgets(preview).find(w=>w instanceof Gtk.ToggleButton&&w.label==='100%');zoom.set_active(true);await delay(100);
  const picture=widgets(preview).find(w=>w instanceof Gtk.Picture);if(picture.width_request!==1800)throw Error('100% view is not native pixels');zoom.set_active(false);await delay(100);await snapshot(window,'-dialog');
  const fixture=Gio.File.new_for_path(`${GLib.get_home_dir()}/fixture.png`).get_uri();
  button(preview,'Açık tema için uygula').emit('clicked');checkPair(fixture,originalDark);
  button(window,'Geri al').emit('clicked');checkPair(original,originalDark);
  button(preview,'Koyu tema için uygula').emit('clicked');checkPair(original,fixture);
  button(window,'Geri al').emit('clicked');checkPair(original,originalDark);preview.close();await delay(250);
  const colors=widgets(window).find(w=>w instanceof Adw.PreferencesPage&&w.title==='Renkler');
  window.set_visible_page(colors);await delay(200);
  for(const [title,name] of [['Açık tema','previous.png'],['Koyu tema','previous-dark.png']]){
   if(!widgets(colors).some(w=>w instanceof Adw.ActionRow&&w.title===title&&w.subtitle===name)||
      !widgets(colors).some(w=>w instanceof Adw.ButtonRow&&w.title===`${title} için uygula`))throw Error('Colors wallpaper row missing: '+title);
  }
  const explore=widgets(window).find(w=>w instanceof Adw.PreferencesPage&&w.title==='Keşfet');
  await snapshot(window,'-colors-narrow');
  window.set_default_size(1000,760);await delay(300);await snapshot(window,'-colors-wide');
  window.set_visible_page(explore);await delay(200);
  button(window,'Kaynakları yönet').emit('clicked');await delay(550);const providers=window.get_visible_dialog();
  const titles=widgets(providers).filter(w=>w instanceof Adw.SwitchRow).map(w=>w.title);
  for(const name of ['Danbooru','Konachan','Gelbooru'])if(titles.includes(name))throw Error('Removed source still displayed: '+name);
  for(const name of ['Yande.re','AIBooru','NekosAPI','Cleveland Museum of Art','The Met','Victoria and Albert Museum'])if(!titles.includes(name))throw Error('Source absent: '+name);
  await snapshot(window,'-sources');providers.close();await delay(250);
  print('phase: open category dialog');button(window,'Kategoriler').emit('clicked');await delay(250);const categories=window.get_visible_dialog();
  print('phase: inspect category lists');const lists=widgets(categories).filter(w=>w instanceof Gtk.ListView&&w.has_css_class('boxed-list'));if(lists.length!==2||lists.reduce((n,w)=>n+w.get_model().get_n_items(),0)!==200)throw Error('Category catalogue must contain 100+100 topics');
  print('phase: search categories');const search=widgets(categories).find(w=>w instanceof Gtk.SearchEntry);search.set_text('Dağlar');await wait(()=>lists[0].get_model().get_n_items()===1,'Category search did not filter');
  print('phase: select mountain');widgets(categories).find(w=>w instanceof Gtk.CheckButton&&w.label==='Dağlar').set_active(true);
  print('phase: switch category tab');const stack=widgets(categories).find(w=>w instanceof Adw.ViewStack);search.set_text('');stack.set_visible_child_name('sensitive');await delay(200);
  await wait(()=>widgets(categories).some(w=>w instanceof Gtk.CheckButton&&w.label==='Sanatsal nü'),'Sensitive catalogue did not render');widgets(categories).find(w=>w instanceof Gtk.CheckButton&&w.label==='Sanatsal nü').set_active(true);
  print('phase: apply categories');await snapshot(window,'-categories');button(categories,'Uygula').emit('clicked');await delay(200);
  await wait(()=>saved().policy.general_topics.includes('g001')&&saved().policy.sensitive_topics.includes('s000'),'Topic selection did not persist');
  print('phase: inspect filtered results');if(grid.get_model().get_n_items()!==1)throw Error('Topic filter must keep only matching safe fixture');
  print('phase: clear categories');button(window,'Kategoriler').emit('clicked');await delay(200);const reset=window.get_visible_dialog();button(reset,'Temizle').emit('clicked');button(reset,'Uygula').emit('clicked');await delay(200);
  widgets(window).find(w=>w instanceof Adw.SwitchRow&&w.title==='SFW').set_active(false);if(grid.get_model().get_n_items()!==0)throw Error('Old images survived disabled rating');
  print(JSON.stringify({native_preferences:true,stale_shell_metadata_supported:true,responsive:true,original_preview:true,native_zoom:true,local_actions:true,apply_targets_independent:true,colors_wallpaper_rows:true,undo:true,topics:200,topic_filter:true,removed_sources:true,filter_hides_cached:true}));
  window.close();await delay(100);const again=new Adw.PreferencesWindow({application:app});await prefs.fillPreferencesWindow(again);again.present();
  if(!widgets(again).find(w=>w instanceof Adw.SwitchRow&&w.title==='SFW').active)throw Error('Safe defaults lost');
  print(JSON.stringify({reopened:true,safe_start:true}));again.close();
 }catch(e){failed=true;printerr(String(e)+'\n'+e.stack);window.close();}
 finally{app.release();app.quit();}
})().catch(e=>{failed=true;printerr(String(e)+'\n'+e.stack);app.release();app.quit();});});
await app.runAsync([]);if(failed)throw Error('Preferences smoke failed');
