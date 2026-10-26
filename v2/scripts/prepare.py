#!/usr/bin/env python3
"""Extract pinned evidence through Git objects, never the host's working tree."""
from pathlib import Path
import hashlib,json,os,subprocess,urllib.request
ROOT=Path(__file__).resolve().parents[1]
assert Path(os.environ.get("HOME","")).resolve()==ROOT/".qa/home", "Use the isolated .sh entrypoint"
REPO=ROOT.parent
SDK='e955911f0bf5b558f9d3eb5183df0adc95c22af2'
HOST='ea100cee3f69711de12485ba061950c5fc65258f'
SOURCES={
 'sdk': (REPO.parent/'tezbar-extension-sdk-rust',SDK,['SDK_API.md','PROTOCOL.md','HOST_INTEGRATION.md','src/runner.rs','src/context.rs','src/lib.rs','LICENSE','NOTICE','Cargo.toml']),
 'host': (REPO.parents[1]/'Tezbar',HOST,['docs/EXTERNAL_RUST_EXTENSIONS.md','schemas/extension-v3.schema.json','src/renderer/ExtensionRuntimeView.tsx','src/shared/extensionRuntime.ts','src/renderer/src/raycast-api/list-runtime.tsx','src/renderer/src/raycast-api/detail-runtime.tsx','src/renderer/src/raycast-api/index.tsx','src-tauri/src/native_core/extensions/process_host.rs','src-tauri/src/native_core/extensions/registry.rs','src-tauri/src/native_core/extensions/loved_extensions.json'])}
records=[]
for name,(repo,sha,files) in SOURCES.items():
 for file in files:
  data=subprocess.check_output(['git','-C',str(repo),'show',sha+':'+file]); target=ROOT/'.qa'/name/file
  target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data)
  records.append(dict(kind=name,commit=sha,path=file,sha256=hashlib.sha256(data).hexdigest(),bytes=len(data)))
for f in subprocess.check_output(['git','-C',str(REPO),'ls-tree','-r','--name-only','48f081909c2b07ab791af1949ad38325c0d1fe6c']).decode().splitlines():
 data=subprocess.check_output(['git','-C',str(REPO),'show','48f081909c2b07ab791af1949ad38325c0d1fe6c:'+f]); records.append(dict(kind='v1',commit='48f081909c2b07ab791af1949ad38325c0d1fe6c',path=f,sha256=hashlib.sha256(data).hexdigest(),bytes=len(data)))
icon_url='https://raw.githubusercontent.com/raycast/extensions/6734d85992290066fd31a7f5bee3cca3d3545a1a/extensions/hashrate-no/assets/hashrate-no.png'
data=urllib.request.urlopen(icon_url,timeout=30).read()
assert data==(REPO/'assets/hashrate-no.png').read_bytes()==(ROOT/'resources/hashrate-no.png').read_bytes()
records.append(dict(kind='icon-upstream',url=icon_url,sha256=hashlib.sha256(data).hexdigest(),bytes=len(data)))
map_path=ROOT/'.qa/v1/node_modules/@raycast/utils/dist/module.js.map'
if map_path.exists():
 m=json.loads(map_path.read_bytes()); records.append(dict(kind='locked-utils',path='@raycast/utils/dist/module.js.map',sha256=hashlib.sha256(map_path.read_bytes()).hexdigest(),bytes=map_path.stat().st_size))
 for name,text in zip(m['sources'],m['sourcesContent']):
  if text is not None:
   name=name.removeprefix('../src/').removeprefix('../').removeprefix('src/');target=ROOT/'.qa/utils'/name
   assert '..' not in Path(name).parts
   target.parent.mkdir(parents=True,exist_ok=True);target.write_text(text)
(ROOT/'evidence/inputs.json').write_text(json.dumps(records,indent=2)+'\n')
print('Pinned input hashes:',len(records))
