#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
cd "$SCRIPT_DIR"

FRONTEND="$SCRIPT_DIR/ngneon-frontend"
if [ ! -x "$FRONTEND" ]; then
    FRONTEND="$SCRIPT_DIR/target/release/ngneon-frontend"
fi

if [ ! -x "$FRONTEND" ]; then
    if [ ! -f "$SCRIPT_DIR/Cargo.toml" ]; then
        printf '%s\n' "ERROR: No se encuentra ngneon-frontend ni Cargo.toml." >&2
        exit 1
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        printf '%s\n' "ERROR: Cargo no está instalado o no está disponible en PATH." >&2
        printf '%s\n' "Instala Rust desde https://rustup.rs/ y vuelve a intentarlo." >&2
        exit 1
    fi

    if command -v pkg-config >/dev/null 2>&1; then
        if ! pkg-config --exists sdl2; then
            printf '%s\n' "AVISO: No se detectan las cabeceras de SDL2 mediante pkg-config." >&2
        fi
        if ! pkg-config --exists alsa; then
            printf '%s\n' "AVISO: No se detectan las cabeceras de ALSA mediante pkg-config." >&2
        fi
    fi

    printf '%s\n' "Compilando ngneon-frontend en modo release..."
    cargo build --release -p frontend --bin ngneon-frontend
    FRONTEND="$SCRIPT_DIR/target/release/ngneon-frontend"
fi

if [ ! -x "$FRONTEND" ]; then
    printf '%s\n' "ERROR: La compilación terminó sin generar un ngneon-frontend ejecutable." >&2
    exit 1
fi

exec "$FRONTEND" "$@"
