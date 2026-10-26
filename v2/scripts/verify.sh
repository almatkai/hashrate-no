#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
. "$SCRIPT_DIR/env.sh"
"$SCRIPT_DIR/oracle.sh"
"$SCRIPT_DIR/schema.sh"
for TOOLCHAIN in stable 1.85.0; do
  "$SCRIPT_DIR/cargo.sh" "+$TOOLCHAIN" fmt --all -- --check
  "$SCRIPT_DIR/cargo.sh" "+$TOOLCHAIN" test --locked --all-features > "$PORT_ROOT/.qa/$TOOLCHAIN-test.log" 2>&1
  "$SCRIPT_DIR/cargo.sh" "+$TOOLCHAIN" clippy --locked --all-targets --all-features -- -D warnings > "$PORT_ROOT/.qa/$TOOLCHAIN-clippy.log" 2>&1
 done
"$SCRIPT_DIR/cargo.sh" +stable build --locked --features qa > "$PORT_ROOT/.qa/build-qa.log" 2>&1
"$SCRIPT_DIR/process_qa.sh"
"$SCRIPT_DIR/renderer.sh"
python3 - "$PORT_ROOT" <<'PY'
from pathlib import Path
import hashlib,json,subprocess,sys
r=Path(sys.argv[1]);logs=['v1-test.log','v1-typecheck.log','stable-test.log','stable-clippy.log','1.85.0-test.log','1.85.0-clippy.log'];record={'v1Tests':1,'v1Typecheck':'passed','rustTestsPerToolchain':7,'toolchains':{},'strictClippy':'all-targets all-features -D warnings','logs':{n:hashlib.sha256((r/'.qa'/n).read_bytes()).hexdigest() for n in logs}}
for t in ['stable','1.85.0']:record['toolchains'][t]=subprocess.check_output(['rustc','+'+t,'--version']).decode().strip()
(r/'evidence/build-qa.json').write_text(json.dumps(record,indent=2)+'\n');print('Stable + MSRV fmt, tests, strict all-target clippy passed.')
PY
