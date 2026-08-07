# Inicio automatico en Raspberry Pi 5

La unidad `ngneon-kiosk.service` reserva `tty1`, reproduce una vez por arranque
`assets/intro/ngneon-intro.mp4` y despues ejecuta `ngneon-frontend` desde
`/home/neo/NGNEON-EMU-linux-aarch64`. Si el frontend falla, systemd lo reinicia
sin repetir la intro.

En KMSDRM el frontend y el emulador no pueden poseer el dispositivo grafico a
la vez. `ngneon-game.service` detiene el frontend al lanzar una ROM, ejecuta
`ngneon-emu.real` como sesion exclusiva y restaura la biblioteca al salir.

## Instalacion

```sh
sudo apt install ffmpeg
chmod +x rpi5/*.sh
./rpi5/generate-intro.sh
sudo install -m 0644 rpi5/ngneon-tmpfiles.conf /etc/tmpfiles.d/ngneon.conf
sudo systemd-tmpfiles --create /etc/tmpfiles.d/ngneon.conf
test -e ngneon-emu.real || mv ngneon-emu ngneon-emu.real
install -m 0755 rpi5/ngneon-game-wrapper.sh ngneon-emu
sudo install -m 0644 rpi5/ngneon-kiosk.service /etc/systemd/system/
sudo install -m 0644 rpi5/ngneon-game.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl disable --now getty@tty1.service
sudo systemctl enable --now ngneon-kiosk.service
```

## Recuperar la consola de texto

Desde SSH:

```sh
sudo systemctl disable --now ngneon-kiosk.service
sudo systemctl enable --now getty@tty1.service
```

Los registros se consultan con:

```sh
journalctl -u ngneon-kiosk.service -b
```

## Arranque silencioso

El script conserva la consola serie, retira la consola de texto HDMI, oculta
mensajes del kernel/systemd y desactiva el splash del firmware:

```sh
sudo ./rpi5/enable-silent-boot.sh
sudo reboot
```

Se guardan copias recuperables en:

- `/boot/firmware/cmdline.txt.ngneon-backup`
- `/boot/firmware/config.txt.ngneon-backup`

Para restaurarlas desde SSH:

```sh
sudo cp -a /boot/firmware/cmdline.txt.ngneon-backup /boot/firmware/cmdline.txt
sudo cp -a /boot/firmware/config.txt.ngneon-backup /boot/firmware/config.txt
sudo reboot
```
