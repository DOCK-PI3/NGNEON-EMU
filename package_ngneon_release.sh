#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
ARCH=$(uname -m 2>/dev/null || printf '%s' "unknown")
OUT_NAME=${NGNEON_PACKAGE_OUT:-"NGNEON-EMU-linux-$ARCH"}

usage() {
    cat <<'EOF'
Uso: ./package_ngneon_release.sh

Compila el workspace en modo release y crea un paquete portátil para Linux.

Variables opcionales:
  NGNEON_PACKAGE_OUT=nombre     Nombre del directorio de salida dentro del proyecto.
  NGNEON_ALLOW_PACKAGE_CLEAN=1 Permite limpiar una salida con datos personales.
EOF
}

case "${1:-}" in
    -h|--help)
        usage
        exit 0
        ;;
    "")
        ;;
    *)
        printf '%s\n' "ERROR: Argumento desconocido: $1" >&2
        usage >&2
        exit 2
        ;;
esac

case "$OUT_NAME" in
    ""|"."|".."|*/*|*\\*)
        printf '%s\n' "ERROR: NGNEON_PACKAGE_OUT debe ser un nombre de directorio simple." >&2
        exit 2
        ;;
esac

OUT="$ROOT/$OUT_NAME"
EMU="$ROOT/target/release/ngneon-emu"
FRONTEND="$ROOT/target/release/ngneon-frontend"
CONFIG_TEMPLATE="$ROOT/config/ngneon.example.conf"
TEMP_CONFIG=""
TEMP_KEYS=""

cleanup() {
    if [ -n "$TEMP_CONFIG" ]; then
        rm -f -- "$TEMP_CONFIG"
    fi
    if [ -n "$TEMP_KEYS" ]; then
        rm -f -- "$TEMP_KEYS"
    fi
}
trap cleanup EXIT HUP INT TERM

fail() {
    printf '%s\n' "ERROR: $*" >&2
    exit 1
}

assert_output_path() {
    case "$1" in
        "$OUT"|"$OUT"/*)
            ;;
        *)
            fail "Operación de paquete fuera del directorio de salida: $1"
            ;;
    esac
}

reset_dir() {
    target=$1
    assert_output_path "$target"
    if [ -e "$target" ]; then
        rm -rf -- "$target"
    fi
    mkdir -p -- "$target"
}

remove_output_path() {
    target=$1
    assert_output_path "$target"
    if [ -e "$target" ]; then
        rm -rf -- "$target"
    fi
}

set_config_key() {
    file=$1
    key=$2
    value=$3
    temp_file="${file}.tmp.$$"

    awk -v wanted="$key" -v replacement="$value" '
        BEGIN { found = 0 }
        {
            line = $0
            separator = index(line, "=")
            if (separator > 0) {
                current = substr(line, 1, separator - 1)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", current)
                if (current == wanted) {
                    print wanted "=" replacement
                    found = 1
                    next
                }
            }
            print line
        }
        END {
            if (!found) {
                print wanted "=" replacement
            }
        }
    ' "$file" > "$temp_file"
    mv -- "$temp_file" "$file"
}

config_has_key() {
    file=$1
    wanted=$2
    awk -v wanted="$wanted" '
        BEGIN { found = 0 }
        {
            separator = index($0, "=")
            if (separator > 0) {
                current = substr($0, 1, separator - 1)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", current)
                if (current == wanted) {
                    found = 1
                    exit
                }
            }
        }
        END { exit(found ? 0 : 1) }
    ' "$file"
}

config_value() {
    file=$1
    wanted=$2
    awk -v wanted="$wanted" '
        {
            separator = index($0, "=")
            if (separator > 0) {
                current = substr($0, 1, separator - 1)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", current)
                if (current == wanted) {
                    value = substr($0, separator + 1)
                    gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
                    print value
                    exit
                }
            }
        }
    ' "$file"
}

apply_portable_config() {
    file=$1
    set_config_key "$file" "rom_path" "roms"
    set_config_key "$file" "bios_dir" "bios"
    set_config_key "$file" "media_dir" "auto"
    set_config_key "$file" "gamepad" "off"
    set_config_key "$file" "cpu_overclock" "off"
    set_config_key "$file" "sprite_limit" "96"
    set_config_key "$file" "disable_adpcm_wrap" "off"
    set_config_key "$file" "memcard" "on"
    set_config_key "$file" "memcard_write_protect" "off"
    set_config_key "$file" "ra_token" ""
    set_config_key "$file" "ra_password" ""
    set_config_key "$file" "ra_username" ""
}

output_has_personal_files() {
    for protected_dir in roms media musica saves screenshots logs; do
        directory="$OUT/$protected_dir"
        if [ ! -d "$directory" ]; then
            continue
        fi

        found=$(
            find "$directory" -type f \
                ! -path "$OUT/roms/readme.txt" \
                ! -path "$OUT/musica/README.md" \
                -print -quit
        )
        if [ -n "$found" ]; then
            printf '%s\n' "Datos detectados: $found" >&2
            return 0
        fi
    done

    return 1
}

output_config_is_modified() {
    actual="$OUT/config/ngneon.conf"
    if [ ! -f "$actual" ]; then
        return 1
    fi

    TEMP_CONFIG=$(mktemp "${TMPDIR:-/tmp}/ngneon-config.XXXXXX")
    TEMP_KEYS=$(mktemp "${TMPDIR:-/tmp}/ngneon-keys.XXXXXX")
    cp -- "$CONFIG_TEMPLATE" "$TEMP_CONFIG"
    apply_portable_config "$TEMP_CONFIG"

    awk '
        /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
        {
            separator = index($0, "=")
            if (separator > 0) {
                key = substr($0, 1, separator - 1)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", key)
                if (key != "") {
                    print key
                }
            }
        }
    ' "$TEMP_CONFIG" > "$TEMP_KEYS"

    while IFS= read -r key; do
        if ! config_has_key "$actual" "$key"; then
            printf '%s\n' "Configuración personalizada: falta la clave $key" >&2
            return 0
        fi

        expected=$(config_value "$TEMP_CONFIG" "$key")
        current=$(config_value "$actual" "$key")
        if [ "$current" != "$expected" ]; then
            printf '%s\n' "Configuración personalizada: $key=$current" >&2
            return 0
        fi
    done < "$TEMP_KEYS"

    return 1
}

command -v cargo >/dev/null 2>&1 || fail "Cargo no está instalado o no está disponible en PATH."
command -v awk >/dev/null 2>&1 || fail "Se necesita awk para generar la configuración portátil."
[ -f "$ROOT/Cargo.toml" ] || fail "No se encuentra Cargo.toml en $ROOT"
[ -f "$CONFIG_TEMPLATE" ] || fail "No se encuentra $CONFIG_TEMPLATE"

if [ -L "$OUT" ] || [ -L "$OUT/themes" ] || [ -L "$OUT/bios" ]; then
    fail "La salida, themes o bios no pueden ser enlaces simbólicos."
fi

if [ -d "$OUT" ] && [ "${NGNEON_ALLOW_PACKAGE_CLEAN:-0}" != "1" ]; then
    if output_has_personal_files || output_config_is_modified; then
        printf '\n%s\n' "Se ha cancelado el empaquetado para proteger datos del usuario." >&2
        printf '%s\n' "Revisa $OUT o ejecuta conscientemente:" >&2
        printf '%s\n' "  NGNEON_ALLOW_PACKAGE_CLEAN=1 ./package_ngneon_release.sh" >&2
        exit 3
    fi
fi

printf '%s\n' "Compilando NGNEON-EMU para Linux ($ARCH)..."
cd "$ROOT"
cargo build --release --workspace

[ -x "$EMU" ] || fail "No se generó $EMU"
[ -x "$FRONTEND" ] || fail "No se generó $FRONTEND"

mkdir -p -- "$OUT"
reset_dir "$OUT/config"
reset_dir "$OUT/roms"
reset_dir "$OUT/musica"
reset_dir "$OUT/saves"
reset_dir "$OUT/screenshots"
reset_dir "$OUT/logs"
remove_output_path "$OUT/media"
remove_output_path "$OUT/target"
remove_output_path "$OUT/runtime"
remove_output_path "$OUT/configurator"

cp -- "$EMU" "$OUT/ngneon-emu"
cp -- "$FRONTEND" "$OUT/ngneon-frontend"
cp -- "$ROOT/iniciar-ngneon.sh" "$OUT/iniciar-ngneon.sh"
cp -- "$ROOT/configurar-ngneon.sh" "$OUT/configurar-ngneon.sh"
chmod 0755 \
    "$OUT/ngneon-emu" \
    "$OUT/ngneon-frontend" \
    "$OUT/iniciar-ngneon.sh" \
    "$OUT/configurar-ngneon.sh"

if [ -d "$ROOT/bios" ]; then
    mkdir -p -- "$OUT/bios"
    cp -a -- "$ROOT/bios/." "$OUT/bios/"
fi

if [ -d "$ROOT/config" ]; then
    cp -a -- "$ROOT/config/." "$OUT/config/"
    find "$OUT/config" -maxdepth 1 -type f -name 'ngneon*.conf' -delete
fi
cp -- "$CONFIG_TEMPLATE" "$OUT/config/ngneon.conf"
apply_portable_config "$OUT/config/ngneon.conf"

mkdir -p -- "$OUT/themes"
if [ -d "$ROOT/themes" ]; then
    cp -a -n -- "$ROOT/themes/." "$OUT/themes/"
fi

for media_type in \
    videos marquees logos wheelhd screenshots fanart mix \
    box2d box3d cartridges titles manuals
do
    mkdir -p -- "$OUT/roms/media/$media_type"
done

cat > "$OUT/roms/readme.txt" <<'EOF'
Coloca aquí tus ROMs .neo o .zip.

El frontend busca por defecto:
  roms/gamelist.xml
  roms/media/

Dentro de media puedes usar: videos, marquees, logos, wheelhd, screenshots,
fanart, mix, box2d, box3d, cartridges, titles y manuals.

Las BIOS se colocan en ../bios/.
Inicia la biblioteca con: ./iniciar-ngneon.sh
EOF

if [ -f "$ROOT/musica/README.md" ]; then
    cp -- "$ROOT/musica/README.md" "$OUT/musica/README.md"
else
    cat > "$OUT/musica/README.md" <<'EOF'
# Música del frontend

Coloca aquí archivos `.mp3`. El frontend los reproduce aleatoriamente cuando
`frontend_music=on`.
EOF
fi

for document in README.md CHANGELOG.md LICENSE LICENSE.txt; do
    if [ -f "$ROOT/$document" ]; then
        cp -- "$ROOT/$document" "$OUT/$document"
    fi
done

cat > "$OUT/LINUX-DEPENDENCIES.txt" <<'EOF'
NGNEON-EMU — dependencias de Linux
=================================

Los ejecutables enlazan con las bibliotecas del sistema. Instala SDL2, ALSA y
OpenGL, además de sus paquetes de desarrollo si vas a compilar.

Debian / Ubuntu:
  sudo apt install build-essential pkg-config libsdl2-dev libasound2-dev libgl1-mesa-dev

Fedora:
  sudo dnf install @development-tools pkgconf-pkg-config SDL2-devel alsa-lib-devel mesa-libGL-devel

Arch Linux:
  sudo pacman -S --needed base-devel pkgconf sdl2 alsa-lib mesa

Compilar:
  cargo build --release --workspace

Ejecutar:
  ./iniciar-ngneon.sh
EOF

printf '\n%s\n' "Paquete Linux creado correctamente:"
printf '  %s\n' "$OUT"
printf '%s\n' "Incluye ngneon-emu, ngneon-frontend, scripts, configuración, temas y directorios vacíos de usuario."
