#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
RUNTIME_DIR=/run/ngneon-session
REQUEST_FILE="$RUNTIME_DIR/game-request.args"

test -x "$ROOT_DIR/ngneon-emu.real" || {
    echo "Error: no existe $ROOT_DIR/ngneon-emu.real" >&2
    exit 1
}

test -d "$RUNTIME_DIR" && test -w "$RUNTIME_DIR" || {
    echo "Error: no se puede escribir en $RUNTIME_DIR" >&2
    exit 1
}

request_tmp=$(mktemp "$RUNTIME_DIR/game-request.XXXXXX")
cleanup() {
    rm -f -- "$request_tmp"
}
trap cleanup EXIT HUP INT TERM

for argument in "$@"; do
    case "$argument" in
        *'
'*)
            echo "Error: un argumento contiene un salto de linea." >&2
            exit 1
            ;;
    esac
    printf '%s\n' "$argument" >>"$request_tmp"
done

test -s "$request_tmp" || {
    echo "Error: falta la ROM que debe ejecutar NGNEON-EMU." >&2
    exit 1
}

mv -- "$request_tmp" "$REQUEST_FILE"
trap - EXIT HUP INT TERM

# La unidad de juego entra en conflicto con el frontend. systemd detiene el
# kiosco (liberando DRM y ALSA), ejecuta la ROM y restaura el frontend al salir.
sudo -n systemctl start --no-block ngneon-game.service

# Normalmente systemd termina este wrapper junto con el frontend. Si llega a
# continuar por una carrera de planificacion, devuelve exito sin mostrar un
# falso "status 1" mientras la unidad toma el control de tty1.
exit 0

