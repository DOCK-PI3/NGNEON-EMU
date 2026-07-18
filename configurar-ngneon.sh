#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)

# Alias de compatibilidad: el configurador forma parte del frontend nativo.
exec "$SCRIPT_DIR/iniciar-ngneon.sh" "$@"
