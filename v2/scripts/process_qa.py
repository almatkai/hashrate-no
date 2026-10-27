#!/usr/bin/env python3
"""Real SDK JSONL child pipes and actual reqwest -> owned HTTPS only.
The production binary is tested from cache with both commands; it receives no
network-capable credentials or transport overrides. No product API is contacted.
"""
from pathlib import Path
import copy,hashlib,http.server,json,os,queue,selectors,ssl,subprocess,threading,time,uuid
ROOT=Path(__file__).resolve().parents[1]
assert Path(os.environ.get("HOME","")).resolve()==ROOT/".qa/home", "Use the isolated .sh entrypoint"
QA=ROOT/'.qa/process';QA.mkdir(parents=True,exist_ok=True)
ENV={k:os.environ[k] for k in ['HOME','TMPDIR','TMP','TEMP','XDG_CACHE_HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','PATH'] if k in os.environ}
ENV['TZ']='UTC'
ORACLE=json.loads((ROOT/'tests/source-oracle.json').read_bytes())
COINS=next(c['input'] for c in ORACLE['views'] if c['command']=='coins')
GPU=next(c['input'] for c in ORACLE['views'] if c['command']=='gpu_estimates')
ID='com.tezbar.native.hashrate-no';KEY='OWNED-SYNTHETIC-API-KEY'
checks=[]
def check(name,condition=True):
 assert condition,name
 checks.append(name)
def digest(s):return hashlib.sha256(s.encode()).hexdigest()
def cache_key(secret,command,power):return digest(f'{digest(secret)}:{command}:{digest(power)}')
class Child:
 def __init__(self,binary,extra=(),name=None,deny_network=False):
  self.root=QA/(name or str(uuid.uuid4()));self.root.mkdir(exist_ok=True)
  self.data=self.root/'data';self.data.mkdir(exist_ok=True)
  self.temp=self.root/'tmp';self.temp.mkdir(exist_ok=True)
  self.stderr=open(self.root/'stderr','wb')
  prefix=['/usr/bin/sandbox-exec','-p','(version 1)(allow default)(deny network*)'] if deny_network else []
  self.p=subprocess.Popen([*prefix,str(binary),*map(str,extra)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.stderr,env=ENV,cwd=self.root)
  self.sel=selectors.DefaultSelector();self.sel.register(self.p.stdout,selectors.EVENT_READ);self.buffer=b'';self.messages={}
  hello=self.read();assert hello['type']=='hello' and hello['id']==ID and hello['sdkVersion']=='0.1.1'
  self.rpc('initialize',dict(protocol=1,extensionId=ID,platform='macOS',packageDir=str(ROOT),dataDir=str(self.data),tempDir=str(self.temp),grantedCapabilities=[],migrationPaths={}))
 def send(self,frame):self.p.stdin.write(json.dumps(frame,separators=(',',':')).encode()+b'\n');self.p.stdin.flush()
 def request(self,method,params):
  ident='h:'+str(uuid.uuid4());self.send(dict(type='request',id=ident,method=method,params=params));return ident
 def read(self,timeout=15):
  deadline=time.monotonic()+timeout
  while b'\n' not in self.buffer:
   assert self.sel.select(max(0,deadline-time.monotonic())),'child response timeout'
   b=os.read(self.p.stdout.fileno(),65536);assert b,'unexpected EOF'
   self.buffer+=b;assert len(self.buffer)<=16*1024*1024
  line,self.buffer=self.buffer.split(b'\n',1)
  message=json.loads(line)
  assert message['type']!='request','unexpected reverse RPC (no host effects in source)'
  return message
 def wait(self,ident):
  while ident not in self.messages:
   message=self.read();assert message['type']=='response',message['type'];self.messages[message['id']]=message
  return self.messages.pop(ident)
 def rpc(self,method,params):return self.wait(self.request(method,params))
 def open(self,session='session',command='coins',power='.5',key=KEY):
  return self.rpc('session.open',dict(sessionId=session,commandName=command,preferences={'api_key':key},argumentValues={'power_cost':power}))
 def call(self,session,action,values=None):return self.rpc('session.call',dict(sessionId=session,operation='extension:invoke-action',payload={'actionId':action,'formValues':values or {}}))
 def dispose(self,s):return self.rpc('session.dispose',{'sessionId':s})
 def close(self):
  try:
   if self.p.poll() is None:self.rpc('shutdown',{});self.p.stdin.close();self.p.wait(timeout=5)
   assert self.p.returncode==0
  finally:
   if self.p.poll() is None:self.p.kill();self.p.wait()
   self.stderr.close();self.sel.close()
  # No raw synthetic credential or URL-with-key may escape into output/error/cache.
  assert KEY.encode() not in (self.root/'stderr').read_bytes()
  for p in self.data.rglob('*'):
   if p.is_file():assert KEY.encode() not in p.read_bytes(),p.name
 def __enter__(self):return self
 def __exit__(self,*exc):self.close()
def result(reply):assert 'error' not in reply,reply.get('error',{});return reply['result']
def rowlist(view):return view['root']['children'][0]['children']
def act(view,index):return view['actions'][index]['id']
class Server(http.server.ThreadingHTTPServer):
 daemon_threads=True
 def __init__(self):
  super().__init__(('127.0.0.1',0),Handler);self.requests=[];self.responses=queue.Queue();self.arrived=queue.Queue()
class Handler(http.server.BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def do_GET(self):
  self.server.requests.append(self.path)
  code,headers,body,gate=self.server.responses.get(timeout=15)
  self.server.arrived.put(True)
  if gate:gate.wait(timeout=15)
  try:
   self.send_response(code)
   for k,v in headers.items():self.send_header(k,v)
   self.end_headers();self.wfile.write(body)
  except (BrokenPipeError,ConnectionResetError,ssl.SSLError):pass
 def setup(self):super().setup();self.connection.settimeout(15)
def offer(server,data=None,code=200,headers=None,raw=None,gate=None):
 body=json.dumps(data,separators=(',',':')).encode() if raw is None else raw
 headers=headers or {'Content-Type':'application/json','Content-Length':str(len(body))}
 server.responses.put((code,headers,body,gate))
def tls_setup():
 cert=QA/'cert.pem';key=QA/'key.pem';conf=QA/'openssl.cnf'
 conf.write_text('[req]\ndistinguished_name=dn\nx509_extensions=ext\nprompt=no\n[dn]\nCN=localhost\n[ext]\nsubjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n')
 with open(QA/'openssl.log','wb') as log:subprocess.run(['/usr/bin/openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1','-sha256','-config',str(conf),'-keyout',str(key),'-out',str(cert)],check=True,stdout=log,stderr=log,env=ENV)
 key.chmod(0o600);server=Server();ctx=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);ctx.load_cert_chain(cert,key);server.socket=ctx.wrap_socket(server.socket,server_side=True)
 thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start();return server,thread,cert

def production_cached(binary):
 # All positive launches have source-derived projected cache; a production network
 # request would block/fail these tests, and no live API request is authorized.
 name='production-'+str(uuid.uuid4());d=QA/name/'data';d.mkdir(parents=True)
 # Use source projected data, preserving the JS numeric-key enumeration order.
 gpu=[]
 for k in ['2','10','z']:gpu.append({'id':k,'estimate':GPU[k]})
 value={'version':1,'entries':{},'settings':{}}
 for command,power,data in [('coins','',COINS),('gpu_estimates','.5',gpu)]:
  value['entries'][cache_key(KEY,command,power)]={'data':data,'date':'1/2/2026','serial':0}
  value['settings'][digest(KEY)+':'+command]={'detail':command!='coins','lastPowerCost':digest(power)}
 (d/'hashrate-cache.json').write_text(json.dumps(value))
 with Child(binary,name=name,deny_network=True) as c:
  coin=result(c.open('coins','coins',''));gpuview=result(c.open('gpu','gpu_estimates'))
  check('production both commands real OS pipes',len(rowlist(coin))==2 and len(rowlist(gpuview))==3)
  check('production no polling',coin['refreshMs'] is None and gpuview['refreshMs'] is None)
  check('hidden coin pane adapted to pinned renderer','detail' not in rowlist(coin)[0]['props'])
  new=result(c.call('coins',act(coin,0)));check('production toggle details',rowlist(new)[0]['props']['detail']['type']=='List.Item.Detail')
  check('production stale action inert',result(c.call('coins',act(coin,0)))['mode']=='view')
  check('production cross session ID inert',result(c.call('gpu',act(new,0)))['mode']=='view')
  for command in ['coins','gpu_estimates']:
   assert result(c.rpc('session.call',dict(sessionId='gpu' if command!='coins' else 'coins',operation='extension:refresh-session',payload={})))['mode']=='unchanged'
  (ROOT/'.qa/production-views.json').write_text(json.dumps([coin,new,gpuview]))
  check('production GPU zero source error',result(c.open('zero','gpu_estimates','0'))['message']=='Power Cost must be a number!')
  check('production missing preference sanitized','API Key is required' in c.open('no-key','coins',key='')['error']['message'])
  check('production unknown command error','error' in c.open('unknown','unknown'))
  c.dispose('coins');c.dispose('gpu');c.dispose('zero')

def network(binary):
 server,thread,cert=tls_setup();port=server.server_address[1]
 try:
  with Child(binary,[port,cert]) as c:
   offer(server,COINS);first=result(c.open());assert not first.get('message'),first.get('message');check('TLS actual source coins route',server.requests[-1]=='/v1/coins?apiKey='+KEY)
   assert result(c.open('cached'))['mode']=='view';check('coins cache hit no request',len(server.requests)==1)
   second=result(c.call('session',act(first,0)));check('toggle no request',len(server.requests)==1)
   offer(server,COINS);third=result(c.call('session',act(second,1)));check('manual Refresh one request',len(server.requests)==2)
   check('old refresh ID inert',result(c.call('session',act(second,1)))['mode']=='view' and len(server.requests)==2)
   offer(server,GPU);gpu=result(c.open('gpu','gpu_estimates',' .5 '));check('raw power query encoding',server.requests[-1].endswith('&powerCost=+.5+'))
   offer(server,GPU);result(c.open('gpu2','gpu_estimates',' .5 '));check('source lastPowerCost first warm relaunch refetch',len(server.requests)==4)
   result(c.open('gpu3','gpu_estimates',' .5 '));check('source subsequent GPU warm relaunch cached',len(server.requests)==4)
   offer(server,COINS);result(c.open('other','coins',key='SECOND-OWNED-SYNTHETIC'));check('credentials isolated',len(server.requests)==5)
   # Every completed failure remains editable/actionable with data and two fresh IDs.
   failures=[(429,{},b'private '+KEY.encode()),(200,{'Content-Type':'text/html'},b'<html>owned</html>'),(200,{'Content-Type':'application/json'},b'{bad'),(200,{},json.dumps({'title':KEY,'detail':'https://example.test/?apiKey='+KEY}).encode()),(200,{},b'['+b' '*524289+b']'),(200,{},b'null'),(200,{},json.dumps([{}]*513).encode())]
   for index,(code,headers,raw) in enumerate(failures):
    offer(server,code=code,headers=headers or None,raw=raw);third=result(c.call('session',act(third,1)));check('sanitized failure '+str(index),bool(third.get('message')) and KEY not in json.dumps(third) and len(rowlist(third))==2)
    n=len(server.requests);result(c.rpc('session.call',dict(sessionId='session',operation='extension:refresh-session',payload={})));assert len(server.requests)==n
   offer(server,COINS);third=result(c.call('session',act(third,1)));check('explicit retry restores data',not third.get('message'))
   offer(server,code=302,headers={'Location':'https://denied.invalid/never-read','Content-Length':'0'},raw=b'');third=result(c.call('session',act(third,1)));check('cross-origin HTTPS redirect rejected',bool(third.get('message')))
   offer(server,code=302,headers={'Location':'http://127.0.0.1:1/never-read','Content-Length':'0'},raw=b'');third=result(c.call('session',act(third,1)));check('HTTP redirect rejected',bool(third.get('message')))
   offer(server,code=302,headers={'Location':f'https://localhost:{port}/v1/coins?apiKey={KEY}','Content-Length':'0'},raw=b'');offer(server,COINS);third=result(c.call('session',act(third,1)));check('same-origin redirect supported',not third.get('message'))
   # Drop an in-flight refresh after the action is consumed outside staged state.
   gate=threading.Event();stale=copy.deepcopy(COINS);stale[0]['name']='STALE-CANCELLED-RESPONSE';offer(server,stale,gate=gate)
   ident=c.request('session.call',dict(sessionId='session',operation='extension:invoke-action',payload={'actionId':act(third,1)}))
   # Drain previous arrival markers, then wait for the new request by count.
   deadline=time.monotonic()+5;before=len(server.requests)
   while len(server.requests)<before+1 and time.monotonic()<deadline:time.sleep(.005)
   c.send({'type':'cancel','requestId':ident});reply=c.wait(ident);gate.set();check('cancel refresh drops network and staged view','error' in reply)
   n=len(server.requests);renewed=result(c.call('session',act(third,1)));assert renewed['mode']=='view';check('cancelled refresh ID cannot replay',len(server.requests)==n)
   assert result(c.rpc('session.call',dict(sessionId='session',operation='extension:refresh-session',payload={})))['mode']=='unchanged';check('cancelled action recovery issues fresh IDs without refetch',len(server.requests)==n)
   offer(server,COINS);fresh=result(c.call('session',act(renewed,1)));check('new deliberate Refresh after cancellation succeeds');check('late cancelled HTTP result cannot overwrite current view','STALE-CANCELLED-RESPONSE' not in json.dumps(fresh) and 'STALE-CANCELLED-RESPONSE' not in (c.data/'hashrate-cache.json').read_text())
   gate=threading.Event();offer(server,COINS,gate=gate);n=len(server.requests);timed=result(c.call('session',act(fresh,1)));gate.set();check('actual reqwest total timeout produces recoverable error',bool(timed.get('message')) and len(server.requests)==n+1)
   offer(server,COINS);result(c.call('session',act(timed,1)));check('explicit retry after timeout succeeds')
   # Arguments/prefs in session.call are SDK snapshots and cannot be overwritten.
   before=len(server.requests)
   assert result(c.rpc('session.call',dict(sessionId='gpu',operation='extension:search-text-changed',payload={'searchText':'A','argumentValues':{'power_cost':'bad'},'preferences':{'api_key':'tamper'}})))['mode']=='view'
   check('search no network, args/prefs not replaced',len(server.requests)==before)
  # Reload the completed private checkpoint in a new process. A persisted old
  # serial from a previous process must never suppress a deliberate refresh.
  saved_name=c.root.name
  saved=json.loads((c.data/'hashrate-cache.json').read_bytes())
  for entry in saved['entries'].values():entry['serial']=2**53
  (c.data/'hashrate-cache.json').write_text(json.dumps(saved))
  with Child(binary,[port,cert],name=saved_name) as c:
   n=len(server.requests);reopened=result(c.open());check('completed checkpoint reused across process restart',len(server.requests)==n)
   offer(server,COINS);result(c.call('session',act(reopened,1)));check('old persisted serial cannot suppress explicit refresh',len(server.requests)==n+1)
  # EOF must cancel an in-flight owned TLS request without a dispose RPC.
  with Child(binary,[port,cert]) as c:
   gate=threading.Event();offer(server,COINS,gate=gate);n=len(server.requests)
   c.request('session.open',dict(sessionId='eof',commandName='coins',preferences={'api_key':KEY},argumentValues={}))
   deadline=time.monotonic()+5
   while len(server.requests)==n and time.monotonic()<deadline:time.sleep(.002)
   assert len(server.requests)==n+1;c.p.stdin.close();c.p.wait(timeout=5);gate.set()
   check('EOF cancels actual pending HTTPS without dispose',c.p.returncode==0 and not (c.data/'hashrate-cache.json').exists())
  # Sequentially cancelled session opens must not leak SDK/adapter registrations.
  with Child(binary,[port,cert]) as c:
   for i in range(140):
    gate=threading.Event();offer(server,COINS,gate=gate);n=len(server.requests)
    ident=c.request('session.open',dict(sessionId=f'cancel:{i}',commandName='coins',preferences={'api_key':KEY},argumentValues={}))
    deadline=time.monotonic()+5
    while len(server.requests)==n and time.monotonic()<deadline:time.sleep(.002)
    assert len(server.requests)==n+1
    c.send({'type':'cancel','requestId':ident});assert 'error' in c.wait(ident);gate.set()
   check('140 cancelled opens require no dispose')
   offer(server,COINS);result(c.open('capacity:0'))
   for i in range(1,128):result(c.open(f'capacity:{i}'))
   check('128 sessions still available after cancelled opens')
   assert 'error' in c.open('overflow');check('129th session rejected explicitly')
   for i in range(128):c.dispose(f'capacity:{i}')
   result(c.open('after-dispose'));check('capacity restored after dispose')
  # Concurrent opens deduplicate while the network result is in flight.
  with Child(binary,[port,cert]) as c:
   gate=threading.Event();offer(server,COINS,gate=gate);n=len(server.requests)
   a=c.request('session.open',dict(sessionId='A',commandName='coins',preferences={'api_key':KEY},argumentValues={}))
   deadline=time.monotonic()+5
   while len(server.requests)==n and time.monotonic()<deadline:time.sleep(.002)
   b=c.request('session.open',dict(sessionId='B',commandName='coins',preferences={'api_key':KEY},argumentValues={}))
   time.sleep(.02);gate.set();result(c.wait(a));result(c.wait(b));check('concurrent launch dedup',len(server.requests)==n+1)
  check('no fixture responses left pending',server.responses.empty())
 finally:server.shutdown();server.server_close();thread.join(timeout=5)

if __name__=='__main__':
 import sys
 production_cached(Path(sys.argv[1]) if len(sys.argv)>1 else ROOT/'target/debug/hashrate-no')
 if len(sys.argv)==1:network(ROOT/'target/debug/qa-process')
 (ROOT/('evidence/process-qa.json' if len(sys.argv)==1 else 'evidence/extracted-process-qa.json')).write_text(json.dumps({'checks':checks,'count':len(checks),'productionBinary':'OS pipes; both commands; owned preloaded cache; macOS sandbox-exec deny network* enforced','networkBinary':'qa-process, explicit compile-only qa feature; actual SDK + reqwest; owned TLS','gui':'not-run'},indent=2)+'\n')
 print('Process QA:',len(checks),'checks passed')
