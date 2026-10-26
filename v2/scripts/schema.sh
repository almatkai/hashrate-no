#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
. "$SCRIPT_DIR/env.sh"
exec node "$SCRIPT_DIR/schema.cjs" "$@"
