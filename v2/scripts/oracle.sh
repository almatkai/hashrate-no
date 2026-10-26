#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
. "$SCRIPT_DIR/env.sh"
export TZ=UTC
exec node "$SCRIPT_DIR/oracle.cjs"
