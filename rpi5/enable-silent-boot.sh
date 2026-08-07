#!/bin/sh
set -eu

BOOT_DIR=/boot/firmware
CMDLINE_FILE="$BOOT_DIR/cmdline.txt"
CONFIG_FILE="$BOOT_DIR/config.txt"
CMDLINE_BACKUP="$BOOT_DIR/cmdline.txt.ngneon-backup"
CONFIG_BACKUP="$BOOT_DIR/config.txt.ngneon-backup"

if test "$(id -u)" -ne 0; then
    echo "Error: ejecuta este script con sudo." >&2
    exit 1
fi

test -f "$CMDLINE_FILE" || {
    echo "Error: no existe $CMDLINE_FILE" >&2
    exit 1
}

test -f "$CONFIG_FILE" || {
    echo "Error: no existe $CONFIG_FILE" >&2
    exit 1
}

cp -a --no-clobber "$CMDLINE_FILE" "$CMDLINE_BACKUP"
cp -a --no-clobber "$CONFIG_FILE" "$CONFIG_BACKUP"

original=$(sed -n '1p' "$CMDLINE_FILE")
case " $original " in
    *" root="*) ;;
    *)
        echo "Error: cmdline.txt no contiene un parametro root= valido." >&2
        exit 1
        ;;
esac

filtered=
for token in $original; do
    case "$token" in
        console=tty1|quiet|splash|loglevel=*|systemd.show_status=*|rd.systemd.show_status=*|udev.log_level=*|vt.global_cursor_default=*|logo.nologo|consoleblank=*)
            continue
            ;;
    esac
    if test -n "$filtered"; then
        filtered="$filtered $token"
    else
        filtered=$token
    fi
done

cmdline_tmp=$(mktemp "$BOOT_DIR/cmdline.txt.ngneon.XXXXXX")
config_tmp=$(mktemp "$BOOT_DIR/config.txt.ngneon.XXXXXX")
cleanup() {
    rm -f -- "$cmdline_tmp" "$config_tmp"
}
trap cleanup EXIT HUP INT TERM

printf '%s\n' "$filtered quiet splash loglevel=3 systemd.show_status=false rd.systemd.show_status=false udev.log_level=3 vt.global_cursor_default=0 logo.nologo consoleblank=0" >"$cmdline_tmp"
chmod --reference="$CMDLINE_FILE" "$cmdline_tmp"
chown --reference="$CMDLINE_FILE" "$cmdline_tmp"

if grep -q '^disable_splash=' "$CONFIG_FILE"; then
    sed 's/^disable_splash=.*/disable_splash=1/' "$CONFIG_FILE" >"$config_tmp"
else
    cp -- "$CONFIG_FILE" "$config_tmp"
    printf '\n%s\n%s\n' '# NGNEON silent boot' 'disable_splash=1' >>"$config_tmp"
fi
chmod --reference="$CONFIG_FILE" "$config_tmp"
chown --reference="$CONFIG_FILE" "$config_tmp"

mv -- "$cmdline_tmp" "$CMDLINE_FILE"
mv -- "$config_tmp" "$CONFIG_FILE"
trap - EXIT HUP INT TERM

grep -Eq '(^| )quiet( |$)' "$CMDLINE_FILE" || {
    echo "Error: no se pudo activar quiet en cmdline.txt." >&2
    exit 1
}

if grep -q 'console=tty1' "$CMDLINE_FILE"; then
    echo "Error: console=tty1 sigue presente." >&2
    exit 1
fi

grep -q '^disable_splash=1$' "$CONFIG_FILE" || {
    echo "Error: no se pudo desactivar el splash del firmware." >&2
    exit 1
}

echo "Arranque silencioso NGNEON activado. Reinicia para aplicarlo."
