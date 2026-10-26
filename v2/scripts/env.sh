#!/bin/sh
# Source from a script that sets SCRIPT_DIR to its own absolute directory.
: "${SCRIPT_DIR:?set SCRIPT_DIR to the calling script directory}"
PORT_ROOT=$(CDPATH='' cd -- "$SCRIPT_DIR/.." && pwd -P)
PORT_RUSTUP_HOME=${RUSTUP_HOME:-$HOME/.rustup}
export HOME="$PORT_ROOT/.qa/home" TMPDIR="$PORT_ROOT/.qa/tmp" TMP="$PORT_ROOT/.qa/tmp" TEMP="$PORT_ROOT/.qa/tmp"
export XDG_CACHE_HOME="$PORT_ROOT/.qa/xdg-cache" XDG_CONFIG_HOME="$PORT_ROOT/.qa/xdg-config" XDG_DATA_HOME="$PORT_ROOT/.qa/xdg-data"
export CARGO_HOME="$PORT_ROOT/.qa/cargo" CARGO_TARGET_DIR="$PORT_ROOT/target" npm_config_cache="$PORT_ROOT/.qa/npm-cache"
# Existing toolchains only; never install or update them.
export RUSTUP_HOME="$PORT_RUSTUP_HOME"
export CARGO_BUILD_JOBS=2 CARGO_NET_GIT_FETCH_WITH_CLI=true
export TZ=UTC
export PYTHONDONTWRITEBYTECODE=1
export NODE_COMPILE_CACHE="$PORT_ROOT/.qa/node-compile-cache"
mkdir -p "$HOME" "$TMPDIR" "$XDG_CACHE_HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$CARGO_HOME" "$npm_config_cache"
