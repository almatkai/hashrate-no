#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
. "$SCRIPT_DIR/env.sh"
"$SCRIPT_DIR/cargo.sh" metadata --locked --format-version 1 > "$PORT_ROOT/.qa/metadata.json"
exec python3 "$SCRIPT_DIR/licenses.py"
