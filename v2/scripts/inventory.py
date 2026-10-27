#!/usr/bin/env python3
"""All Git-visible V2 files + root README, without staging or self-hashing."""
from pathlib import Path
import hashlib,json,os,subprocess
ROOT=Path(__file__).resolve().parents[1]
assert Path(os.environ.get("HOME","")).resolve()==ROOT/".qa/home", "Use the isolated .sh entrypoint";REPO=ROOT.parent
assert subprocess.check_output(['git','-C',str(REPO),'rev-parse','--show-toplevel']).decode().strip()==str(REPO)
assert subprocess.check_output(['git','-C',str(REPO),'branch','--show-current']).decode().strip()=='feat/native-v2-rust'
assert subprocess.check_output(['git','-C',str(REPO),'rev-parse','HEAD']).decode().strip()=='48f081909c2b07ab791af1949ad38325c0d1fe6c'
assert subprocess.check_output(['git','-C',str(REPO),'diff','--cached','--name-only'])==b'', 'Index must remain empty'
names=set(subprocess.check_output(['git','-C',str(REPO),'ls-files','--cached','--others','--exclude-standard','-z']).decode().strip('\0').split('\0'))
selected=sorted(n for n in names if (n=='README.md' or n.startswith('v2/')) and n!='v2/REVIEW-INVENTORY.json')
assert all(not any(part in {'.qa','target','dist','node_modules','__pycache__'} for part in Path(n).parts) and not n.endswith(('.zip','.pyc')) for n in selected)
# Every V1 tracked byte is unchanged except this exact discovery suffix.
suffix=b'\n\nNative Rust V2 source and verification: [v2/README.md](v2/README.md).\n'
for name in subprocess.check_output(['git','-C',str(REPO),'ls-tree','-r','--name-only','HEAD']).decode().splitlines():
 data=subprocess.check_output(['git','-C',str(REPO),'show','HEAD:'+name]);assert (REPO/name).read_bytes()==data+(suffix if name=='README.md' else b''),name
inventory={name:{'sha256':hashlib.sha256((REPO/name).read_bytes()).hexdigest(),'bytes':(REPO/name).stat().st_size} for name in selected}
(ROOT/'REVIEW-INVENTORY.json').write_text(json.dumps({'baseCommit':'48f081909c2b07ab791af1949ad38325c0d1fe6c','branch':'feat/native-v2-rust','scope':'all Git-visible v2 files and root README, excluding this inventory','count':len(inventory),'files':inventory},indent=2)+'\n')
print('Review inventory:',len(inventory),'files; all V1 bytes preserved except exact README suffix; index empty.')
