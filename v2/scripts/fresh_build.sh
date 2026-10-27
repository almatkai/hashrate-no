#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
. "$SCRIPT_DIR/env.sh"
BUILD_TAG="fresh-$(date -u +%Y%m%dT%H%M%SZ)-$$"
export CARGO_HOME="$PORT_ROOT/.qa/$BUILD_TAG/cargo" CARGO_TARGET_DIR="$PORT_ROOT/.qa/$BUILD_TAG/target"
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
mkdir -p "$CARGO_HOME" "$CARGO_TARGET_DIR"
[ -z "$(ls -A "$CARGO_HOME")" ] && [ -z "$(ls -A "$CARGO_TARGET_DIR")" ]
cd "$PORT_ROOT"
cargo build --release --locked --bin hashrate-no > "$PORT_ROOT/.qa/$BUILD_TAG/build.log" 2>&1
printf '%s\n' "$CARGO_TARGET_DIR/release/hashrate-no" > "$PORT_ROOT/.qa/fresh-binary-path"
python3 - "$PORT_ROOT" "$BUILD_TAG" <<'PY'
import pathlib,json,sys,hashlib,subprocess
r=pathlib.Path(sys.argv[1]);tag=sys.argv[2];b=r/'.qa'/tag/'target/release/hashrate-no';source=next((r/'.qa'/tag/'cargo/git/checkouts').glob('*/e955911'))
info=subprocess.check_output([str(b),'--protocol-info']).decode();assert json.loads(info)['commands']==['coins','gpu_estimates']
record={'emptyCargoHome':True,'emptyTarget':True,'locked':True,'sdkHTTPS':'https://github.com/almatkai/tezbar-extension-sdk-rust','sdkRev':subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD']).decode().strip(),'binarySha256':hashlib.sha256(b.read_bytes()).hexdigest(),'binaryInfo':subprocess.check_output(['file',str(b)]).decode().split(': ',1)[1].strip(),'protocolInfo':json.loads(info)}
assert record['sdkRev']=='e955911f0bf5b558f9d3eb5183df0adc95c22af2';assert 'arm64' in record['binaryInfo']
(r/'evidence/fresh-build.json').write_text(json.dumps(record,indent=2)+'\n');print('Empty Cargo home + target locked HTTPS build passed:',record['binaryInfo'])
PY
