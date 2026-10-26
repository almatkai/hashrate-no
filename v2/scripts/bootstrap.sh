#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
. "$SCRIPT_DIR/env.sh"
python3 - "$PORT_ROOT" <<'PY'
from pathlib import Path
import subprocess,sys
r=Path(sys.argv[1]);source=r.parent;dest=r/'.qa/v1';dest.mkdir(parents=True,exist_ok=True)
for name in subprocess.check_output(['git','-C',str(source),'ls-tree','-r','--name-only','48f081909c2b07ab791af1949ad38325c0d1fe6c']).decode().splitlines():
 data=subprocess.check_output(['git','-C',str(source),'show','48f081909c2b07ab791af1949ad38325c0d1fe6c:'+name]);p=dest/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data)
for f in ['package.json','package-lock.json']:
 p=r/'.qa/tooling'/f;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes((r/'qa/tooling'/f).read_bytes())
PY
(cd "$PORT_ROOT/.qa/v1" && npm ci --ignore-scripts --no-audit --no-fund > ../npm-ci.log 2>&1 && npm test > ../v1-test.log 2>&1 && npm run typecheck > ../v1-typecheck.log 2>&1)
(cd "$PORT_ROOT/.qa/tooling" && npm ci --ignore-scripts --no-audit --no-fund > ../qa-tooling.log 2>&1)
"$SCRIPT_DIR/prepare.sh"
"$SCRIPT_DIR/cargo.sh" fetch --locked
"$SCRIPT_DIR/licenses.sh"
