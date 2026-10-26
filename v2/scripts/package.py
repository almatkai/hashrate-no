#!/usr/bin/env python3
"""Deterministic ARM64 development ZIP, closed member map, extracted process QA."""
from pathlib import Path
import hashlib,json,os,stat,subprocess,zipfile
ROOT=Path(__file__).resolve().parents[1]
assert Path(os.environ.get("HOME","")).resolve()==ROOT/".qa/home", "Use the isolated .sh entrypoint"
BINARY=Path((ROOT/'.qa/fresh-binary-path').read_text().strip())
info=json.loads(subprocess.check_output([str(BINARY),'--protocol-info']))
assert info['id']=='com.tezbar.native.hashrate-no' and info['commands']==['coins','gpu_estimates']
assert 'arm64' in subprocess.check_output(['file',str(BINARY)]).decode()
sha=lambda b:hashlib.sha256(b).hexdigest()
files={'bin/hashrate-no':BINARY.read_bytes(),'LICENSE':(ROOT/'LICENSE').read_bytes(),'NOTICE':(ROOT/'NOTICE').read_bytes()}
for p in sorted((ROOT/'resources').rglob('*')):
 if p.is_file():assert not p.is_symlink();files[p.relative_to(ROOT).as_posix()]=p.read_bytes()
manifest=json.loads((ROOT/'extension.json').read_bytes());assert manifest['artifacts']=={}
manifest['files']={p:sha(b) for p,b in sorted(files.items())}
files['extension.json']=(json.dumps(manifest,indent=2)+'\n').encode()
assert len(files)<4096 and all(not p.startswith('/') and '..' not in Path(p).parts and '\\' not in p for p in files)
dist=ROOT/'dist';dist.mkdir(exist_ok=True)
archive=dist/'hashrate-no-2.0.0-aarch64-apple-darwin-dev.zip';second=ROOT/'.qa/deterministic-second.zip'
def write_zip(path):
 with zipfile.ZipFile(path,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
  for name,data in sorted(files.items()):
   entry=zipfile.ZipInfo(name,(1980,1,1,0,0,0));entry.create_system=3;entry.compress_type=zipfile.ZIP_DEFLATED;entry.external_attr=(stat.S_IFREG|(0o755 if name.startswith('bin/') else 0o644))<<16
   z.writestr(entry,data,compress_type=zipfile.ZIP_DEFLATED,compresslevel=9)
write_zip(archive);write_zip(second);assert archive.read_bytes()==second.read_bytes()
zip_sha=sha(archive.read_bytes());extract=dist/('extracted-'+zip_sha[:16]);extract.mkdir(exist_ok=True)
with zipfile.ZipFile(archive) as z:
 names=z.namelist();assert len(names)==len(set(names)) and set(names)==set(files)
 for name in names:
  data=z.read(name);assert data==files[name]
  p=extract/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data);p.chmod(0o755 if name.startswith('bin/') else 0o644)
 actual={p.relative_to(extract).as_posix() for p in extract.rglob('*') if p.is_file()};assert actual==set(files)
 assert {name:sha(z.read(name)) for name in names if name!='extension.json'}==manifest['files']
subprocess.run([str(ROOT/'scripts/schema.sh'),str(extract/'extension.json')],check=True)
subprocess.run([str(ROOT/'scripts/process_qa.sh'),str(extract/'bin/hashrate-no')],check=True)
record={'archive':archive.relative_to(ROOT).as_posix(),'sha256':zip_sha,'bytes':archive.stat().st_size,'deterministicByteComparison':True,'fullPinnedPackagedSchemaValidation':True,'extractedProductionProcess':'passed; both commands; no product API','target':'aarch64-apple-darwin','members':{p:sha(b) for p,b in sorted(files.items())},'memberCount':len(files),'executableSha256':sha(files['bin/hashrate-no']),'artifacts':{},'published':False}
(ROOT/'evidence/archive.json').write_text(json.dumps(record,indent=2)+'\n')
print('Deterministic dev ZIP:',len(files),'closed members;',zip_sha)
