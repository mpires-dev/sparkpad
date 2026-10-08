#!/usr/bin/env python3
"""Real native-client/server synchronization regression; synthetic temporary databases only."""
import subprocess,os,tempfile,pathlib,secrets,json,time,sqlite3,uuid,urllib.request,urllib.error,sys
root=pathlib.Path(__file__).resolve().parents[1]; temp=pathlib.Path(tempfile.mkdtemp(prefix='sparkpad-network-')); token=secrets.token_urlsafe(48);port=17348; url=f'http://127.0.0.1:{port}';processes=[]
def wait(fn,label,timeout=15):
 end=time.time()+timeout
 while time.time()<end:
  try:
   v=fn()
   if v:return v
  except Exception:pass
  time.sleep(.05)
 raise AssertionError(label)
def server():
 env=os.environ|{'SPARKPAD_SERVER_TOKEN':token,'SPARKPAD_SERVER_BIND':f'127.0.0.1:{port}','SPARKPAD_SERVER_DATA':str(temp/'server'),'SPARKPAD_SERVER_WEB':str(root/'apps/sync-web/dist')}
 p=subprocess.Popen([str(root/'target/debug/sparkpad-sync-server')],env=env,stdout=open(temp/'server.log','a'),stderr=subprocess.STDOUT);processes.append(p);wait(lambda:urllib.request.urlopen(url+'/health').status==200,'server ready');return p
paths=[temp/'a/notes.sqlite3',temp/'b/notes.sqlite3']
def worker(i):
 path=paths[i];path.parent.mkdir(exist_ok=True);config=path.with_suffix('.sync.json');config.write_text(json.dumps({'url':url,'token':token}));config.chmod(0o600)
 p=subprocess.Popen([str(root/'target/debug/sparkpad'),'sync','run'],env=os.environ|{'SPARKPAD_DB':str(path)},stdout=open(temp/f'client{i}.log','a'),stderr=subprocess.STDOUT);processes.append(p)
 wait(lambda:sqlite3.connect(path).execute("SELECT enabled FROM sync_control").fetchone()==(1,),'client enabled');return p
def body(i,id):return sqlite3.connect(paths[i]).execute('SELECT markdown FROM notes WHERE id=?',(id,)).fetchone()[0]
def edit(i,id,value):
 with sqlite3.connect(paths[i]) as db:db.execute('UPDATE notes SET markdown=?,revision=revision+1 WHERE id=?',(value,id))
try:
 s=server();a=worker(0);b=worker(1);id=str(uuid.uuid4());initial='Hello 🌱 world\n- [ ] Task'
 with sqlite3.connect(paths[0]) as db:db.execute('INSERT INTO notes(id,title,markdown) VALUES(?,?,?)',(id,'Sync regression',initial))
 wait(lambda:body(1,id)==initial,'initial native sync')
 b.terminate();b.wait();edit(1,id,'Hello 🌱 world\n- [x] Task');edit(0,id,'Hello 🌱 brave world\n- [ ] Task');b=worker(1)
 expected='Hello 🌱 brave world\n- [x] Task';wait(lambda:body(0,id)==expected and body(1,id)==expected,'offline native merge')
 s.terminate();s.wait();edit(0,id,expected+'!');s=server();wait(lambda:body(1,id)==expected+'!','server restart catches offline update',40)
 try:urllib.request.urlopen(urllib.request.Request(url+'/api/session',headers={'Authorization':'Bearer wrong'}));raise AssertionError('bad key accepted')
 except urllib.error.HTTPError as e:assert e.code==401
 (temp/'connection.json').write_text(json.dumps({'url':url,'token':token,'id':id}));(temp/'connection.json').chmod(0o600)
 print('PASS: native↔native sync, offline merge, server restart and authentication. Fixture:',temp,flush=True)
 # Leave the test server running for browser tests; clients run concurrently.
 (pathlib.Path('/tmp/sparkpad-network-fixture')).write_text(str(temp));
 if '--keep' in sys.argv:
  while not (temp/'stop').exists():time.sleep(.3)
finally:
 for p in processes:
  if p.poll() is None:p.terminate()
 for p in processes:
  try:p.wait(timeout=3)
  except subprocess.TimeoutExpired:p.kill()
