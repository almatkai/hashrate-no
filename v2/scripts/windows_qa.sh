#!/bin/sh
set -u
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
. "$SCRIPT_DIR/env.sh"
cd "$PORT_ROOT"
CARGO_TARGET_DIR="$PORT_ROOT/target/windows-gnu" cargo check --locked --all-targets --all-features --target x86_64-pc-windows-gnu > .qa/windows-check.log 2>&1
WINDOWS_CHECK_CODE=$?
CARGO_TARGET_DIR="$PORT_ROOT/target/windows-gnu" cargo clippy --locked --all-targets --all-features --target x86_64-pc-windows-gnu -- -D warnings > .qa/windows-clippy.log 2>&1
WINDOWS_CLIPPY_CODE=$?
python3 - "$PORT_ROOT" "$WINDOWS_CHECK_CODE" "$WINDOWS_CLIPPY_CODE" <<'PY'
from pathlib import Path
import hashlib,json,sys
r=Path(sys.argv[1]);check=int(sys.argv[2]);clippy=int(sys.argv[3]);logs=[r/'.qa/windows-check.log',r/'.qa/windows-clippy.log'];missing=all('failed to find tool "x86_64-w64-mingw32-gcc"' in p.read_text() for p in logs)
record={'target':'x86_64-pc-windows-gnu','checkExit':check,'clippyExit':clippy,'status':'passed' if check==clippy==0 else 'blocked' if missing else 'failed','reason':'Existing GNU Rust target present; MinGW x86_64-w64-mingw32-gcc missing. No toolchain/compiler installed.' if missing else 'See bounded-attempt logs.','logs':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in logs},'msvc':'not-run','gui':'not-run'}
(r/'evidence/windows-qa.json').write_text(json.dumps(record,indent=2)+'\n');print('Windows GNU:',record['status'],record['reason'])
PY
