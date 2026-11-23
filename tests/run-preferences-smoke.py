import os,tempfile,subprocess,time,json,zlib,struct,signal,zipfile,hashlib,shutil
from pathlib import Path
root=Path(__file__).resolve().parents[1]
wayland_name=os.environ.get('WAYLAND_DISPLAY','wayland-0')
wayland_socket=Path(wayland_name) if Path(wayland_name).is_absolute() else Path(os.environ.get('XDG_RUNTIME_DIR',f'/run/user/{os.getuid()}'))/wayland_name
native=os.environ.get('DUVAR_SMOKE_BACKEND','wayland' if wayland_socket.is_socket() else 'broadway')=='wayland'
if native and not wayland_socket.is_socket():raise RuntimeError('Wayland test socket unavailable')
extension_override=os.environ.get('DUVAR_EXTENSION_DIR')
test_parent=Path.home()/'.cache'
test_parent.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='duvar-test-runtime-',dir=test_parent) as d:
    if extension_override:
        extension=Path(extension_override)
    else:
        extension=Path(d)/'extension'
        archive=root/'dist/duvar@mo4tal.github.io.shell-extension.zip'
        if not archive.exists():raise RuntimeError('Önce duvar-package ile ZIP oluştur.')
        with zipfile.ZipFile(archive) as z:
            if any(Path(n).is_absolute() or '..' in Path(n).parts for n in z.namelist()):raise RuntimeError('Geçersiz ZIP yolu')
            z.extractall(extension)
        subprocess.run(['glib-compile-schemas','--strict',str(extension/'schemas')],check=True)
    runtime=Path(d)/'runtime';runtime.mkdir(mode=0o700)
    home=Path(d)/'home';home.mkdir()
    env=dict(os.environ,HOME=str(home),XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(home/'.config'),XDG_DATA_HOME=str(home/'.local/share'),XDG_CACHE_HOME=str(home/'.cache'),XDG_STATE_HOME=str(home/'.local/state'),GSETTINGS_BACKEND='memory',GIO_USE_VFS='local',GTK_A11Y='none',GDK_BACKEND='broadway',BROADWAY_DISPLAY=':41',GSK_RENDERER='broadway')
    def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    pixels=b''.join(b'\x00'+bytes([25+y//2,85,125])*360 for y in range(200))
    image=home/'fixture.png';image.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',360,200,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(pixels))+chunk(b'IEND',b''))
    thumbnail=home/'fixture-thumb.png';thumbnail.write_bytes(image.read_bytes())
    previous=home/'previous.png';previous.write_bytes(image.read_bytes())
    previous_dark=home/'previous-dark.png';previous_dark.write_bytes(image.read_bytes())
    fullpixels=b''.join(b'\x00'+bytes([25+y//8,85,125])*1800 for y in range(1000))
    image.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1800,1000,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(fullpixels))+chunk(b'IEND',b''))
    favorites=[dict(id=f'fixture:{i}',provider='Yerel test koleksiyonu',title=['Sessiz ufuk','Gece mavisi','Sakin çizgiler'][i%3]+f' {i+1}',thumbnail=thumbnail.as_uri(),image=image.as_uri(),landing='',rating='safe',tags=['mountain'] if i==11 else ['nature'],width=1800+i,height=1000,author='Duvar test fixture',license='Test görseli',local=True) for i in range(12)]
    env['GI_TYPELIB_PATH']='/usr/lib/gnome-shell/girepository-1.0'
    env['LD_LIBRARY_PATH']='/usr/lib/gnome-shell'
    env['DUVAR_TEST_CONFIGURATION']=json.dumps(dict(wallhaven=False,openverse=False,favorites=favorites,local_directory=str(home/'local-images'),download_directory=str(home/'custom-downloads'),sort='latest'))
    env['DUVAR_TEST_ORIGINAL']=previous.as_uri()
    env['DUVAR_TEST_ORIGINAL_DARK']=previous_dark.as_uri()
    smoke_out=Path(os.environ.get('DUVAR_SMOKE_OUT',str(root/'target/smoke')))
    smoke_out.mkdir(parents=True,exist_ok=True)
    env['DUVAR_TEST_SCREENSHOT']=os.environ.get('DUVAR_TEST_SCREENSHOT',str(smoke_out/'preview.png'))
    for key in ['DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','GNOME_KEYRING_CONTROL','SSH_AUTH_SOCK','SESSION_MANAGER']:env.pop(key,None)
    server=None;browser=None;browser_log=None
    if native:
        env['GDK_BACKEND']='wayland';env['WAYLAND_DISPLAY']=str(wayland_socket)
        env.pop('GSK_RENDERER',None);env.pop('BROADWAY_DISPLAY',None)
    else:
        server=subprocess.Popen(['/usr/bin/gtk4-broadwayd','--address','127.0.0.1','--port','18097',':41'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    try:
        if not native:
            time.sleep(.25)
            if server.poll() is not None:raise RuntimeError(server.communicate())
            browser_log=(Path(d)/'browser.log').open('w')
            browser=subprocess.Popen(['/usr/bin/helium-browser','--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--disable-sync','--disable-background-networking','--disable-component-update','--user-data-dir='+str(home/'headless-profile'),'--remote-debugging-port=0','--window-size=1280,1000','http://127.0.0.1:18097/'],env=env,stdout=browser_log,stderr=browser_log,start_new_session=True)
            time.sleep(.8)
        log=Path(d)/'gjs.log'
        with log.open('w') as output:
            process=subprocess.Popen(['/usr/bin/dbus-run-session','--','/usr/bin/gjs','-m',str(root/'tests/preferences-smoke.js'),str(extension)],env=env,stdout=output,stderr=output,text=True,start_new_session=True)
            try:
                code=process.wait(timeout=25)
            except subprocess.TimeoutExpired:
                code=124
            finally:
                try:os.killpg(process.pid,signal.SIGTERM)
                except ProcessLookupError:pass
                try:process.wait(timeout=3)
                except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait()
        if code==0:
            # Negative control: use a separate code fixture, never alter a live/override extension.
            version_fixture=Path(d)/'version-fixture';shutil.copytree(extension,version_fixture)
            version_metadata=json.loads((version_fixture/'metadata.json').read_text());version_metadata['version-name']='999.0.0-fixture';(version_fixture/'metadata.json').write_text(json.dumps(version_metadata))
            with log.open('a') as output:
                rejected=subprocess.run(['/usr/bin/dbus-run-session','--','/usr/bin/gjs','-m',str(root/'tests/preferences-version.js'),str(version_fixture)],env=env,stdout=output,stderr=output,timeout=8)
                code=rejected.returncode
        if code==0:
            # Different GJS process, empty memory GSettings: only durable config can restore these.
            env['DUVAR_REOPEN_DIRECTORY']=str(home/'local-images')
            env['DUVAR_REOPEN_DOWNLOADS']=str(home/'custom-downloads')
            with log.open('a') as output:
                reopened=subprocess.run(['/usr/bin/dbus-run-session','--','/usr/bin/gjs','-m',str(root/'tests/preferences-reopen.js'),str(extension)],env=env,stdout=output,stderr=output,timeout=8)
                code=reopened.returncode
        if code==0:
            queue=home/'.local/state/duvar/download-queue.json'
            jobs=[dict(id=1000+i,item=favorites[i],directory=str(home/'resumed-downloads'),scope='both',status='active' if i==1 else 'queued',error='',path='') for i in [1,2]]
            queue.write_text(json.dumps(jobs))
            with log.open('a') as output:
                queued=subprocess.run(['/usr/bin/dbus-run-session','--','/usr/bin/gjs','-m',str(root/'tests/preferences-queue.js'),str(extension)],env=env,stdout=output,stderr=output,timeout=10)
                code=queued.returncode
        if code==0:
            fixtures=home/'scroll-fixtures';fixtures.mkdir()
            for i in range(72):os.link(image,fixtures/f'{i:03}.png')
            (fixtures/'bad.png').write_bytes(b'corrupt fixture')
            env['DUVAR_SCROLL_FIXTURES']=str(fixtures)
            env['XDG_CONFIG_HOME']=str(home/'scroll-config')
            with log.open('a') as output:
                scrolled=subprocess.run(['/usr/bin/dbus-run-session','--','/usr/bin/gjs','-m',str(root/'tests/preferences-scroll.js'),str(extension)],env=env,stdout=output,stderr=output,timeout=10)
                code=scrolled.returncode
        if code==0:
            transfer=home/'transfer-tests';transfer.mkdir()
            for kind in ['config','data','cache','state']:(transfer/kind).mkdir()
            env.update(XDG_CONFIG_HOME=str(transfer/'config'),XDG_DATA_HOME=str(transfer/'data'),XDG_CACHE_HOME=str(transfer/'cache'),XDG_STATE_HOME=str(transfer/'state'))
            config=transfer/'config/duvar';config.mkdir();cache=transfer/'cache/duvar';cache.mkdir()
            remote=[]
            for i in range(3):
                item=dict(favorites[i]);item.update(id=f'remote:{i}',provider='Offline fixture',title=f'Remote fixture {i+1}',local=False,image=f'https://example.invalid/original-{i}.png',thumbnail=f'https://example.invalid/thumb-{i}.png',width=0,height=0)
                remote.append(item)
                digest=hashlib.sha256(item['image'].encode()).hexdigest();shutil.copyfile(image,cache/(digest+'.original'))
                if i==1:
                    with (cache/(digest+'.original')).open('ab') as extra:extra.write(bytes(8*1024*1024))
                thumbhash=hashlib.sha256(item['id'].encode()).hexdigest();(cache/(thumbhash+'.rgba')).write_bytes(struct.pack('<II',360,200)+bytes([30,85,125,255])*360*200)
            (config/'preferences.json').write_text(json.dumps(dict(wallhaven=False,openverse=False,favorites=remote,download_directory=str(transfer/'downloads'),policy=dict(safe=True))))
            with log.open('a') as output:
                transferred=subprocess.run(['/usr/bin/dbus-run-session','--','/usr/bin/gjs','-m',str(root/'tests/preferences-transfers.js'),str(extension)],env=env,stdout=output,stderr=output,timeout=20)
                code=transferred.returncode
        if browser:
            try:os.killpg(browser.pid,signal.SIGTERM)
            except ProcessLookupError:pass
            try:browser.wait(timeout=3)
            except subprocess.TimeoutExpired:os.killpg(browser.pid,signal.SIGKILL);browser.wait()
            browser_log.close()
            (smoke_out/'smoke-browser-last.log').write_text((Path(d)/'browser.log').read_text())
        print('GJS result:',code,'backend:','wayland' if native else 'broadway',flush=True)
        (smoke_out/'smoke-last.log').write_text(log.read_text())
        report=log.read_text()
        print(report[-1500:])
        if code==0 and ('"filter_hides_cached":true' not in report or '"reopened":true' not in report):code=1
        # Only this test's private document mount may be unmounted.
        doc=runtime/'doc'
        if doc.is_mount():subprocess.run(['/usr/bin/fusermount3','-u',str(doc)],capture_output=True,timeout=3)
        raise SystemExit(code)
    finally:
        if server:
            server.terminate()
            try:server_out,server_err=server.communicate(timeout=3)
            except subprocess.TimeoutExpired:server.kill();server_out,server_err=server.communicate()
            (smoke_out/'smoke-broadway-last.log').write_text(server_out+server_err)
            if server.returncode not in (0,-15):print('Broadway diagnostic:',server_err[-1800:])
