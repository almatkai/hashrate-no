#!/usr/bin/env python3
"""Copy authentic license grants from checksum-verified locked crate sources."""
from pathlib import Path
import hashlib,json,os,re,tarfile,urllib.request
ROOT=Path(__file__).resolve().parents[1]
assert Path(os.environ.get("HOME","")).resolve()==ROOT/".qa/home", "Use the isolated .sh entrypoint"
CARGO=Path(os.environ['CARGO_HOME'])
lock={'package':[dict(re.findall(r'^(name|version|source|checksum) = "([^"]*)"$', block, re.M)) for block in (ROOT/'Cargo.lock').read_text().split('[[package]]')[1:]]}
metadata_packages=json.loads((ROOT/'.qa/metadata.json').read_bytes())['packages']
records=[]
for package in lock['package']:
 name,version=package['name'],package['version']
 if name=='hashrate-no-native':continue
 source=package['source']
 if source.startswith('registry+'):
  candidates=list((CARGO/'registry/src').glob(f'*/{name}-{version}'));assert len(candidates)==1,(name,version)
  directory=candidates[0]
  archives=list((CARGO/'registry/cache').glob(f'*/{name}-{version}.crate'));assert len(archives)==1
  archive=archives[0];assert hashlib.sha256(archive.read_bytes()).hexdigest()==package['checksum']
  tar=tarfile.open(archive);archive_files={m.name.split('/',1)[1]:m for m in tar.getmembers() if m.isfile() and '/' in m.name}
 else:
  assert name=='tezbar-extension-sdk' and 'e955911f0bf5b558f9d3eb5183df0adc95c22af2' in source
  candidates=list((CARGO/'git/checkouts').glob('*/e955911'));assert len(candidates)==1
  directory=candidates[0];tar=None;archive_files={}
 metadata=next(p for p in metadata_packages if p['name']==name and p['version']==version)
 files=[]
 for path in sorted(directory.rglob('*')):
  if not path.is_file():continue
  basename=path.name.upper()
  if not any(word in basename for word in ['LICENSE','LICENCE','COPYING','COPYRIGHT','NOTICE','AUTHORS']):continue
  # Source code names such as license.rs are not stand-alone grants.
  if path.suffix in ['.rs','.c','.h','.js']:continue
  relative=path.relative_to(directory);data=path.read_bytes();sha=hashlib.sha256(data).hexdigest()
  if tar is not None:assert tar.extractfile(archive_files[relative.as_posix()]).read()==data,(name,str(relative))
  dest=ROOT/'resources/licenses'/f'{name}-{version}'/relative
  dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
  files.append({'sourcePath':relative.as_posix(),'path':dest.relative_to(ROOT).as_posix(),'sha256':sha,'bytes':len(data)})
 if tar is not None:tar.close()
 assert files,(name,'No authentic license evidence found')
 records.append({'name':name,'version':version,'source':source,'checksum':package.get('checksum'),'license':metadata.get('license'),'licenseFile':metadata.get('license_file'),'files':files})
url='https://raw.githubusercontent.com/raycast/extensions/6734d85992290066fd31a7f5bee3cca3d3545a1a/LICENSE'
data=urllib.request.urlopen(url,timeout=30).read();dest=ROOT/'resources/licenses/raycast-extensions-MIT.txt';dest.write_bytes(data)
records.append({'name':'raycast-extensions','commit':'6734d85992290066fd31a7f5bee3cca3d3545a1a','url':url,'files':[{'path':dest.relative_to(ROOT).as_posix(),'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)}]})
(ROOT/'resources/locked-dependencies.json').write_text(json.dumps(records,indent=2)+'\n')
print('Authentic license inventories:',len(records),'packages/upstream;',sum(len(r['files']) for r in records),'files')
