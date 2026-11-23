"""Self-contained install/share/rollback tests; never reads a live installation."""
import hashlib,json,os,shutil,subprocess,tempfile,zipfile,tarfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
bins=Path(os.environ.get('DUVAR_BIN_DIR',Path(os.environ.get('CARGO_TARGET_DIR',root/'target'))/'release'))
archive=root/'dist/duvar@mo4tal.github.io.shell-extension.zip'
version=json.loads((root/'dist/manifest.json').read_text())['version']
sharing=next((root/'dist').glob(f'duvar-{version}-linux-*.tar.gz'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert (sharing.parent/(sharing.name+'.sha256')).read_text()==f'{sha(sharing)}  {sharing.name}\n'
with tempfile.TemporaryDirectory(prefix='duvar-install-test-') as tmp:
    t=Path(tmp);data=t/'data';state=t/'state';config=t/'config';target=data/'gnome-shell/extensions/duvar@mo4tal.github.io'
    target.mkdir(parents=True)
    with zipfile.ZipFile(archive) as z:z.extractall(target)
    subprocess.run(['glib-compile-schemas','--strict',str(target/'schemas')],check=True)
    # A valid, distinguishable old ELF. Appended bytes do not alter loadable segments.
    library=target/'native/libduvar_native.so'
    with library.open('ab') as f:f.write(b'rollback-fixture')
    build=target/'native/duvar-build.json';manifest=json.loads(build.read_text());manifest['artifacts']['native/libduvar_native.so']=sha(library);build.write_text(json.dumps(manifest))
    old=sha(library)
    keep=data/'duvar/personal-keep.txt';keep.parent.mkdir(parents=True);keep.write_text('private fixture: must not be packaged or changed')
    preferences=config/'duvar/preferences.json';preferences.parent.mkdir(parents=True);preferences.write_text('{"query":"PRIVATE_FIXTURE_QUERY"}')
    env=dict(os.environ,XDG_DATA_HOME=str(data),XDG_STATE_HOME=str(state),XDG_CONFIG_HOME=str(config))
    with tarfile.open(sharing) as package:package.extractall(t,filter='data')
    extracted=next(t.glob(f'duvar-{version}-linux-*'));shared=t/'paylaşım klasörü';extracted.rename(shared);binary=shared/'duvar-install'
    subprocess.run(['sha256sum','-c','SHA256SUMS'],cwd=shared,capture_output=True,text=True,check=True)
    assert os.access(binary,os.X_OK)
    def invoke(*args,ok=True):
        result=subprocess.run([str(binary),*map(str,args)],env=env,cwd=t,capture_output=True,text=True)
        if ok and result.returncode:raise RuntimeError(result.stdout+result.stderr)
        if not ok:assert result.returncode!=0
        return result
    invoke('--check');assert sha(library)==old
    fresh=t/'fresh-data'
    subprocess.run([str(binary)],env=dict(env,XDG_DATA_HOME=str(fresh),XDG_STATE_HOME=str(t/'fresh-state')),cwd=t,capture_output=True,text=True,check=True)
    fresh_target=fresh/target.relative_to(data)
    assert sha(fresh_target/'native/libduvar_native.so')==json.loads((root/'dist/manifest.json').read_text())['library_sha256']
    for key,expected in {'palette-enabled':'false','palette-follow':'true','palette-gtk':'true',
                         'palette-adwaita':'false','palette-adwaita-refresh':'false','ui-language':"'tr'"}.items():
        actual=subprocess.check_output(['gsettings','--schemadir',str(fresh_target/'schemas'),
                                        'get','org.gnome.shell.extensions.duvar',key],
                                       env=dict(env,GSETTINGS_BACKEND='memory'),text=True).strip()
        assert actual==expected,(key,actual)
    invoke();assert sha(library)==json.loads((root/'dist/manifest.json').read_text())['library_sha256']
    checkpoint=next((state/'duvar/checkpoints').glob('installed-before-*'))
    invoke('--rollback',checkpoint);assert sha(library)==old
    for name,link in [('../escape',False),('link',True)]:
        bad=t/'bad.zip'
        with zipfile.ZipFile(bad,'w') as z:
            info=zipfile.ZipInfo(name)
            if link:info.create_system=3;info.external_attr=0o120777<<16
            z.writestr(info,'/tmp/unrelated' if link else 'invalid')
        invoke('--install',bad,ok=False);assert sha(library)==old
    def repack(path,edit):
        with zipfile.ZipFile(archive) as original,zipfile.ZipFile(path,'w') as out:
            for name in original.namelist():out.writestr(name,edit(name,original.read(name)))
    corrupt=t/'corrupt.zip';repack(corrupt,lambda n,b:b+b'broken' if n=='prefs.js' else b);invoke('--install',corrupt,ok=False);assert sha(library)==old
    def wrong_arch(name,body):
        if name=='native/duvar-build.json':
            v=json.loads(body);v['architecture']='unsupported-fixture';return json.dumps(v).encode()
        return body
    def wrong_shell(name,body):
        if name=='metadata.json':
            v=json.loads(body);v['shell-version']=['999'];return json.dumps(v).encode()
        return body
    unsupported=t/'wrong-shell.zip';repack(unsupported,wrong_shell);invoke('--install',unsupported,ok=False);assert sha(library)==old
    wrong=t/'wrong-arch.zip';repack(wrong,wrong_arch);invoke('--install',wrong,ok=False);assert sha(library)==old
    assert not (t/'escape').exists() and keep.read_text().startswith('private fixture') and 'PRIVATE_FIXTURE_QUERY' in preferences.read_text()
    # Doctor runs from a foreign CWD with no dependency on the original project path.
    invoke();doctor=shared/'duvar-doctor'
    health=subprocess.run([str(doctor),'--json'],env=env,cwd=t,capture_output=True,text=True,check=True)
    report=json.loads(health.stdout);assert report['ok'],report
    share=sharing
    with tarfile.open(share) as tar:
        for member in tar.getmembers():
            assert not Path(member.name).is_absolute() and '..' not in Path(member.name).parts
            assert not member.issym() and not member.islnk()
            if member.name.endswith(('.json','.md','.txt')):
                assert b'PRIVATE_FIXTURE_QUERY' not in tar.extractfile(member).read()
    source=next(shared.glob('duvar-source-*.tar.gz'))
    with tarfile.open(source) as src:
        assert any(n.endswith('/Cargo.lock') for n in src.getnames())
        assert not any('/docs/0.5/' in n or '/.git/' in n or '/target/' in n for n in src.getnames())
    with zipfile.ZipFile(archive) as z:
        for name in ('i18n.js','palette.js','palette-prefs.js','palette-worker.js','gallery-wallpaper.js','user-theme-policy.js','setup-palette.sh'):
            assert z.read(name)==(root/'extension'/name).read_bytes()
        for lang in ('tr','en','es','de','fr','pt','ru','ja','zh','ar'):
            assert z.read(f'locales/{lang}.json')==(root/'locales'/f'{lang}.json').read_bytes()
        assert not any(n.startswith('development/') or n.endswith(('.log','health.json','verification.json','preferences.json')) for n in z.namelist())
        assert 'third-party/index.json' in z.namelist()
    print(json.dumps(dict(atomic_install=True,rollback_integrity=True,autodiscovery_foreign_cwd=True,unicode_spaces=True,check_does_not_replace=True,invalid_package_preserves_install=True,corruption_rejected=True,wrong_architecture_rejected=True,wrong_shell_rejected=True,personal_data_preserved=True,portable_doctor=True,clean_sharing_archive=True,source_allowlist=True,bundle_hashes=True)))
