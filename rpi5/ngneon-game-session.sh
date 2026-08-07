#!/bin/sh
set -eu

ROOT_DIR=/home/neo/NGNEON-EMU-linux-aarch64
RUNTIME_DIR=/run/ngneon-session
REQUEST_FILE="$RUNTIME_DIR/game-request.args"
RUNNING_FILE="$RUNTIME_DIR/game-running.args"
EMULATOR="$ROOT_DIR/ngneon-emu.real"

test -x "$EMULATOR" || {
    echo "Error: no existe el emulador real: $EMULATOR" >&2
    exit 1
}

test -s "$REQUEST_FILE" || {
    echo "Error: no existe una peticion de juego valida." >&2
    exit 1
}

mv -- "$REQUEST_FILE" "$RUNNING_FILE"
set --
while IFS= read -r argument || test -n "$argument"; do
    set -- "$@" "$argument"
done <"$RUNNING_FILE"
rm -f -- "$RUNNING_FILE"

test "$#" -gt 0 || {
    echo "Error: la peticion de juego no contiene argumentos." >&2
    exit 1
}

exec "$EMULATOR" "$@"

