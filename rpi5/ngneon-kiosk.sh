#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
INTRO_VIDEO="$PROJECT_DIR/assets/intro/ngneon-intro.mp4"
RUNTIME_DIR=${NGNEON_RUNTIME_DIR:-/run/ngneon-session}
INTRO_MARKER="$RUNTIME_DIR/intro-played"

export HOME=${HOME:-/home/neo}
export SDL_VIDEODRIVER=${SDL_VIDEODRIVER:-kmsdrm}
export SDL_AUDIODRIVER=${SDL_AUDIODRIVER:-alsa}

if test -x "$PROJECT_DIR/ngneon-frontend"; then
    FRONTEND="$PROJECT_DIR/ngneon-frontend"
else
    FRONTEND="$PROJECT_DIR/target/release/ngneon-frontend"
fi

test -x "$FRONTEND" || {
    echo "Error: no existe el frontend compilado: $FRONTEND" >&2
    exit 1
}

mkdir -p "$RUNTIME_DIR"

# /run se limpia en cada arranque. El marcador evita repetir la intro si
# systemd reinicia solamente el frontend despues de un fallo.
if test ! -e "$INTRO_MARKER"; then
    : >"$INTRO_MARKER"
    if test -r "$INTRO_VIDEO" && command -v ffplay >/dev/null 2>&1; then
        ffplay -hide_banner -loglevel warning -nostats -autoexit -fs -noborder \
            -window_title "NGNEON-EMU" "$INTRO_VIDEO" || true
    else
        echo "Aviso: intro o ffplay no disponible; arrancando el frontend." >&2
    fi
fi

exec "$FRONTEND"
