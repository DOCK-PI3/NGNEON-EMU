# NGNEON-EMU

Emulador NeoGeo nativo y multiplataforma en Rust. El proyecto evita tecnologías web: el frontend actual usa SDL2 y el núcleo de emulación se mantiene separado en `core-emulator`.

## Ejecutar

```powershell
cargo run --release --bin ngneon-emu -- --demo
```

También puedes pasar una ROM directamente:

```powershell
cargo run --release --bin ngneon-emu -- C:\ruta\juego.zip
```

Tras compilar en release, también se puede ejecutar el binario directamente:

```powershell
target\release\ngneon-emu.exe --demo
target\release\ngneon-emu.exe roms\aof.neo
target\release\ngneon-emu.exe --no-dump-rom-banks roms\aof.neo
target\release\ngneon-emu.exe --help
```

Si no pasas argumentos, se abre un selector de archivos. Si cancelas el selector, arranca la demo interna.

## BIOSEl emulador busca BIOS reales de forma opcional en el directorio configurado con la clave `bios_dir` (por defecto `bios/`). Acepta archivos sueltos `.bin`, `.rom`, `.sp1` y ZIPs como `neogeo.zip` o `aes.zip`.

**Resolución del directorio BIOS**: `resolve_bios_directory()` busca en orden: `bios_dir` de `config/ngneon.conf` → `<exe_dir>/bios` → `./bios` → fallback `bios/`. La pestaña PATHS del menú de configuración (Ctrl+S) muestra la ruta resuelta.

Si encuentra varias BIOS, prioriza una BIOS oficial MVS como `sp-s2.sp1`; si no encuentra ninguna, usa la BIOS diagnóstica interna para mantener las pruebas sintéticas funcionando.

Al arrancar `frontend` o `probe_rom`, la consola muestra la BIOS activa:

```powershell
[INFO] BIOS activa: neogeo.zip:sp-s2.sp1
```

Si el ZIP de BIOS contiene `000-lo.lo`, también se carga como tabla L0 de shrink/zoom de sprites:

```powershell
[INFO] Tabla L0 activa: neogeo.zip:000-lo.lo
```

Si el ZIP de BIOS contiene `sfix.sfix`, se carga como SFIX de placa para respetar el latch que conmuta entre fix de BIOS/placa y S-ROM de cartucho:

```powershell
[INFO] SFIX activa: neogeo.zip:sfix.sfix
```

Para forzar una BIOS concreta sin renombrar archivos, se puede usar una pista de nombre:

```powershell
$env:NGNEON_BIOS_HINT='uni-bios_4_0'
cargo run --release --manifest-path core-emulator/Cargo.toml --bin probe_rom -- roms/aof.neo screenshots/aof_probe_unibios4.bmp 5000 coin,start 5000
```

## Generar ROM .neo de prueba

```powershell
cargo run --release --manifest-path core-emulator/Cargo.toml --bin make_test_neo
cargo run --release --manifest-path core-emulator/Cargo.toml --bin render_test_neo
cargo run --release --manifest-path core-emulator/Cargo.toml --bin probe_rom -- roms/aof.neo screenshots/aof_probe.bmp 10
cargo run --release --manifest-path core-emulator/Cargo.toml --bin probe_rom -- roms/aof.neo screenshots/aof_probe_coin_start.bmp 5000 coin,start 5000
cargo run --release --manifest-path frontend/Cargo.toml -- examples/ngneon_test.neo
```

La ROM generada contiene una P-ROM mínima con vector reset 68000 válido. El programa escribe la paleta en `0x400000`, configura VRAM mediante registros LSPC (`0x3C0000/2/4`) y después entra en un bucle `NOP; BRA`. También incluye S-ROM/C-ROM sintéticas con un sprite y un tile fix visibles. Sirve para probar el contenedor `.neo`, loader, mapa de memoria, ejecución básica de CPU, escritura de palette RAM, escritura de VRAM por LSPC, sprites SCB, capa fix y salida de video sin BIOS ni ROM comercial.
`render_test_neo` genera una captura headless en `screenshots/ngneon_test_headless.bmp`.

## Controles

### Juego
- Flechas: mover demo / direcciones del jugador 1.
- `Z`, `X`, `C`, `V`: botones A, B, C, D.
- `Enter`: Start.
- `Space`: Coin, mapeado en `STATUS_A` (`0x320001`, bit 0 active-low).

### Gestión de ROM
- `Esc`: salir (con selector BIOS, Save Manager o ROM Browser abierto, cierra el overlay).
- `P`: pausar/reanudar el juego.
- `F1`: cargar una ROM desde el selector nativo.
- `F5`: volver a la demo interna.
- `F8`: reiniciar la máquina emulada.
- Al ejecutar una ROM real, SDL2 oculta automáticamente el cursor del sistema operativo; al volver a la demo interna con `F5`, lo muestra de nuevo.

### Navegador de ROMs (Ctrl+O)
- `Ctrl+O`: abrir/cerrar el navegador de ROMs en cuadrícula.
  - Muestra los juegos disponibles en un grid de 2 columnas con **box art/cartucho** grande (150×84 px).
  - `↑`/`↓`: navegar entre filas.
  - `←`/`→`: navegar entre columnas.
  - `Enter`: cargar la ROM seleccionada.
  - `Esc`: cerrar el navegador.
  - Indicador de página: "PAG X/Y - N JUEGOS".
  - Si un juego no tiene box art, se muestra un rectángulo oscuro con el nombre del juego.

  **Carpeta `media/`:**
  - Coloca tus imágenes de caja frontal/cartucho en `media/` dentro del directorio del emulador, o cambia la carpeta desde `Ctrl+S` → `RUTAS` → `Dir. Media`.
  - Las imágenes deben tener **el mismo nombre que la ROM**, con extensión `.png`.
    - Ejemplo: `roms/aof.neo` → `media/aof.png`
    - Ejemplo: `roms/mslugx.neo` → `media/mslugx.png`
  - Formato recomendado: **PNG**, cualquier resolución. Se ajusta automáticamente y de forma proporcional dentro del recuadro 150×84.
  - El ROM Browser se muestra sin CRT/scanlines/bloom para mantener las carátulas nítidas.
  - El emulador también escanea el directorio configurado en `rom_path` (clave `rom_path` de `config/ngneon.conf`).

### Efectos CRT (GPU — OpenGL 3.3)
- `F2`: toggle scanlines (persiste en config global + per-game).
- `Shift+F2`: reset per-game scanlines → revierte al valor global.
- `F3`: toggle CRT curvature (persiste en config global + per-game).
- `Shift+F3`: reset per-game CRT curvature → revierte al valor global.
- `F4`: toggle phosphor bloom (persiste en config global + per-game).
- `Shift+F4`: reset per-game bloom → revierte al valor global.
- `F11`: alternar pantalla completa (persiste en config).

### Save States
- `F9`: guardar estado en el slot actual.
- `F10`: cargar estado del slot actual.
- `F7`: slot anterior (shift+F7: slot siguiente).
- `Ctrl+F12`: abrir/cerrar el Gestor de Save States (overlay completo con miniaturas).
  - Dentro del gestor: `F9` guarda, `F10` carga, `Supr` elimina, flechas navegan slots.

### Capturas y diagnóstico
- `F6`: toggle debug overlay (FPS, PC, SR, ciclos CPU, label ROM).
- `F12`: guardar captura BMP del framebuffer en `screenshots/`.
- Al cargar una ROM: se genera auto-captura BMP y auto-volcado WAV en `screenshots/`.
- Al cargar una ROM, también se generan volcados de diagnóstico de los bancos ROM (P-ROM, C-ROM, S-ROM, M-ROM, V-ROM) en `screenshots/{label}_{bank}_dump.bin`. Para bancos grandes (>64 KiB) se muestra un indicador de progreso en consola. Se pueden desactivar con `--no-dump-rom-banks` o la clave de configuración `diagnostic_dumps`.

### BIOS Selector
- `Ctrl+B`: abrir/cerrar el selector de BIOS.
  - `Enter`: aplicar la BIOS seleccionada y reiniciar la CPU (persiste en config).
  - Flechas `↑`/`↓`: navegar la lista.
  - `Esc`: cerrar.

### Idioma
- `Ctrl+L`: alternar entre Español/Inglés (persiste en config).
- `--help` muestra el texto de ayuda en el idioma activo.

### Menú de configuración (Ctrl+S)
- `Ctrl+S`: abrir/cerrar el menú de configuración con **6 pestañas**:
  - **VIDEO**: scanlines, CRT curvature, phosphor bloom, fullscreen, **window scale** (2x/3x/4x).
  - **AUDIO**: volumen + **mute** (ON/OFF). El volumen usa un **modo de ajuste dedicado**: presiona `Enter` sobre «Volume» para entrar, luego `←`/`→` ajustan ±5%, `Enter`/`Esc` salen. La barra muestra bloques `█`/`░`.
  - **SYSTEM**: idioma (ES/EN), volcado diagnóstico de ROMs, **auto-save**, selector de BIOS.
  - **CONTROLS**: backend **Gamepad SDL2** (ON/OFF, requiere reinicio), configuración de gamepad (`>`), **configuración de teclado** (`>`).
  - **PATHS**: rutas de ROMs, BIOS, screenshots, saves (solo lectura, **rutas resueltas**).
  - `←`/`→`: **siempre** cambian de pestaña (sin sobrecarga de ajuste de volumen).
  - `↑`/`↓`: navegar entre opciones (si estabas en modo ajuste de volumen, sale automáticamente).
  - `Enter`: activar/desactivar toggle, entrar/salir del modo ajuste de volumen, o abrir submenú.
  - `Esc`: cerrar el menú.

### Volumen
- `Ctrl+M`: silenciar/restaurar volumen al instante (guarda el volumen anterior).
- `Ctrl+=`: subir volumen +5% (hasta 100).
- `Ctrl+-`: bajar volumen -5% (hasta 0).
- Ajuste fino desde **Settings → Sistema → Volumen** con `←`/`→`.
- El volumen se persiste en `config/ngneon.conf`.
- Mientras el mute está activo, los atajos `Ctrl+=`/`Ctrl+-` limpian el mute y ajustan el volumen.

### Gamepad (Ctrl+G)
- `Ctrl+G`: abrir/cerrar el overlay de configuración de gamepad.
  - **12 acciones remapeables**: 10 acciones NeoGeo + `Exit` + `ROM Browser`.
  - Flechas `↑`/`↓` o D-Pad/stick: navegar la lista de acciones.
  - `Enter` o botón `A`/`Start`: iniciar modo escucha de botón SDL2 para reasignar (pulso visual).
  - `R` o botón `C`: restaurar todos los bindings a valores por defecto.
  - `Esc` o botón `B`/`Back`: cancelar escucha o cerrar el overlay.
  - **Atajos globales por defecto**: `Back+Start` sale del emulador sin preguntar; `Guide` abre el ROM Browser.
  - **Multi-controller**: muestra todos los gamepads conectados, navegables con `←`/`→`.
  - **Hotplug**: detecta conexión/desconexión de gamepads sin reiniciar.
  - **Analog stick D-Pad**: el stick izquierdo emula las direcciones (deadzone ±8000).
  - **Persistencia**: bindings guardados automáticamente en `config/gamepad/<guid>.conf` (un archivo por controller).

### Keyboard Config (Settings → CONTROLS → Keyboard Config)
- Accesible desde **Ctrl+S → CONTROLS → Keyboard Config (`Enter`)**.
- **10 acciones remapeables**: Up, Down, Left, Right, A, B, C, D, Start, Coin.
- **Reasignar tecla**: `Enter` sobre una acción → modo escucha (pulso visual). Pulsa cualquier tecla para asignarla.
- **Restaurar default**: tecla `R` sobre la acción seleccionada.
- **Cancelar**: `Esc` sale del modo escucha o cierra el overlay.
- **Sin duplicados**: al asignar una tecla ya usada por otra acción, se libera automáticamente.
- **Persistencia**: los bindings se guardan en `config/keyboard.conf` (scancodes SDL2).
- **Soporte completo de teclas**: ~90 variantes SDL2 (A-Z, 0-9, F1-F24, flechas, numpad, puntuación `. , / \ - = [ ] ; `` ` `` , media keys, browser keys, sistema).

### Perfil de juego
- El perfil CRT por juego se mantiene como sistema interno de overrides por ROM.
  - Scanlines, CRT curvature y phosphor bloom con valores `Global` o `Override`.
  - Permite inspeccionar qué valores aplican a la ROM actual sin modificar nada.
  - `Esc`: cerrar.
  - Los valores se modifican con `F2`/`F3`/`F4` (toggle) o `Shift+F2`/`F3`/`F4` (reset a global).

### Carga automática (sesión persistente)
- Al salir: auto-guarda el estado de la ROM actual en slot 0.
- Al iniciar: auto-carga el estado guardado (slot 0) si existe.
- La última ROM cargada se restaura automáticamente desde `config/ngneon.conf`.
- El navegador de ROMs (`Ctrl+O`) también lee `rom_path` de la configuración para saber qué directorio escanear.

## Configuración persistente

Todas las preferencias se guardan en `config/ngneon.conf` (formato `key=value` simple):

```
bios=neogeo.zip:sp-s2.sp1
bios_dir=bios
lang=es
scanlines=on
curvature=off
bloom=on
fullscreen=off
rom_path=roms/
volume=100
diagnostic_dumps=on
window_scale=3
auto_save=on
muted=off
gamepad=off
ra_token=
ra_password=
ra_username=
ra_hardcore=off
```

> **Nota**: `bios` guarda el **label** de BIOS (ej. `neogeo.zip:sp-s2.sp1`), mientras que `bios_dir` define el **directorio** donde buscar los archivos de BIOS. Si `bios_dir` no existe o no se especifica, el emulador busca en `<exe_dir>/bios`, `./bios`, y finalmente `bios/`.
>
> Las claves `ra_token`, `ra_password`, `ra_username` y `ra_hardcore` configuran la integración con [RetroAchievements](https://retroachievements.org). Si hay `ra_token`, se prefiere sobre `ra_password`.
>
> **Gamepad SDL2**: `gamepad=off` es el modo seguro por defecto para evitar bloqueos de arranque en algunos backends de mando de Windows. Puedes activarlo desde **Ctrl+S → CONTROLS → Gamepad SDL2** y reiniciar el emulador para que SDL inicialice los mandos.

### Configuración de gamepad (`config/gamepad/<guid>.conf`)

El mapeo de botones de gamepad se persiste en `config/gamepad/` con un archivo por controller (identificado por su GUID). Formato `button_name=action_name`:

```
# NGNEON-EMU gamepad button mapping
# Controller GUID: 030000005e040000...

A=A
B=B
X=C
Y=D
Back=Coin
Guide=Coin
Start=Start
LStick=A
RStick=B
LB=Coin
RB=Start
D-Up=Up
D-Down=Down
D-Left=Left
D-Right=Right

# System/global actions. Use Button or Button+Button.
SystemExit=Back+Start
SystemRomBrowser=Guide
```

- **Carga automática**: al iniciar (`scan_initial()`) y en cada hotplug connect.
- **Guardado automático**: al cerrar overlay, restaurar defaults, reasignar botón, y al salir.
- **15 botones SDL2** mapeables a **10 acciones NeoGeo** y **2 acciones globales**.
- Las acciones globales aceptan un botón simple (`Guide`) o una combinación de dos botones (`Back+Start`).
- El directorio `config/gamepad/` se crea automáticamente si no existe.

### Configuración de teclado (`config/keyboard.conf`)

El mapeo de teclas se persiste en `config/keyboard.conf` con formato `ActionName=SDLK_SCANCODE`:

```
Up=1073741906
Down=1073741905
Left=1073741904
Right=1073741903
A=122
B=120
C=99
D=118
Start=13
Coin=32
```

- Se carga automáticamente al iniciar.
- Se guarda al cerrar el overlay de Keyboard Config.
- Soporta teclas de puntuación (`.` `,` `/` `\` `-` `=` `[` `]` `;` `` ` ``), F1-F24, numpad (`KP=` `Kp,`), media keys (VolUp, Mute, Play, etc.), browser keys, y teclas de sistema (Sleep, Eject, brillo).

### Perfiles por juego

Además de la configuración global, cada ROM puede tener ajustes CRT personalizados en `config/profiles/<sanitized_label>.conf`:

```
scanlines=on
curvature=on
bloom=off
```

- Los valores del perfil del juego toman precedencia sobre los globales.
- `F2`/`F3`/`F4` guardan en **ambos**: global + per-game.
- `Shift+F2`/`F3`/`F4` **resetean** el override del perfil, revirtiendo al valor global.
- Si el perfil se queda vacío al resetear, el archivo se elimina automáticamente.

## RetroAchievements

Integración con [RetroAchievements.org](https://retroachievements.org) mediante la librería nativa `rcheevos` (C) + FFI en Rust.

### Características

- **Login con API token o password**: autenticación mediante token web de RetroAchievements, con fallback por password si no hay token configurado.
- **Identificación de juegos**: hash MD5 de la P-ROM para identificar el juego en la base de datos de RA.
- **Logros (Achievements)**: evaluación en tiempo real de condiciones de memoria del juego.
- **Notificaciones visuales**: popup dorado con el nombre del logro y puntuación al desbloquear.
- **Modo Hardcore**: desactiva save states para juego legítimo (rankings).
- **Rich Presence**: estado del juego en tiempo real (progreso, nivel, etc.) reportado a RA.
- **Leaderboards**: puntuaciones enviadas a tablas de clasificación (procesado internamente).
- **Persistencia**: token/password, username y preferencia hardcore guardados en `config/ngneon.conf`.
- **Login automático**: al iniciar, si hay un token o password guardado, intenta hacer login automáticamente.

### Configuración

Añade estas claves a `config/ngneon.conf`:

```ini
ra_token=TU_API_TOKEN
ra_password=TU_PASSWORD
ra_username=TuUsuario
ra_hardcore=off
```

- **`ra_token`**: tu token de API de RetroAchievements (consíguelo en [Settings > API Keys](https://retroachievements.org/settings)).
- **`ra_password`**: fallback para login con password si no tienes `ra_token` configurado. `ra_token` tiene prioridad.
- **`ra_username`**: tu nombre de usuario en RetroAchievements (se guarda automáticamente tras el primer login exitoso).
- **`ra_hardcore`**: `on`/`off`. En modo hardcore no se permiten save states, rewind, ni trampas.

### Menú de configuración (Ctrl+S → SYSTEM)

En la pestaña **SYSTEM** del menú de configuración aparecen las opciones:
- **RA Login**: muestra el estado de la sesión (`No has iniciado sesión` / `Sesión: Usuario` / `Iniciar sesión`).
- **Modo Hardcore**: ON/OFF.

La UI muestra notificaciones cuando:
- Se desbloquea un logro (recuadro dorado en pantalla con el título y puntos).
- El login tiene éxito o falla.
- Un leaderboard es enviado.

### Arquitectura

La integración se compone de:

| Componente | Archivo | Descripción |
|---|---|---|
| FFI C | `core-emulator/src/rcheevos_ffi.rs` | Bindings Rust a la librería C `rcheevos` |
| Sesión RA | `core-emulator/src/retroachievements.rs` | `RASession`: cliente RA, callbacks, eventos, hashing |
| Build script | `core-emulator/build.rs` | Compila `rcheevos` desde C y enlaza con el core |
| API pública | `core-emulator/src/lib.rs` | Métodos `init_retroachievements()`, `ra_login()`, `ra_load_game()`, `ra_take_events()` |
| UI notificaciones | `frontend/src/ui.rs` | `draw_achievement_notification()`: popup de logro con bordes dorados |
| Menú settings | `frontend/src/ui.rs` | Pestaña SYSTEM del menú con opciones RA |
| Strings i18n | `frontend/src/lang.rs` | 13 campos `ra_*` en español e inglés |
| Estado runtime | `frontend/src/main.rs` | `RuntimeStatus`: token, username, hardcore, notificaciones, `frame_count` |

### Cómo probar

```powershell
# 1. Configura tu token en config/ngneon.conf
echo 'ra_token=TuApiToken' >> config/ngneon.conf

# 2. Ejecuta el emulador con una ROM compatible
cargo run --release --bin ngneon-emu -- roms/aof.neo

# 3. Abre el menú de configuración (Ctrl+S) → pestaña SYSTEM
#    Verás el estado de RA Login y Modo Hardcore
```

### Tests

Los tests de notificaciones de logros están en `frontend/src/ui.rs` (módulo `tests`):
- `achievement_notification_modifies_framebuffer` — verifica que se renderizan píxeles
- `achievement_notification_bilingual_titles` — valida títulos en ES/EN
- `achievement_notification_handles_long_name` — prueba truncado de nombres largos
- `achievement_notification_stacks_at_different_offsets` — verifica stacking de notificaciones
- `achievement_notification_zero_points` — caso borde: 0 puntos

---

## Translación (Español / English)

Todo el texto visible está centralizado en `frontend/src/lang.rs`:

- `Lang::spanish()` — todas las strings en español.
- `Lang::english()` — todas las strings en inglés.
- El idioma se selecciona con `Ctrl+L` y persiste en `config/ngneon.conf`.
- Los overlays (BIOS Selector, Save State Manager) se renderizan en el idioma activo.
- El texto de ayuda (`--help`) se genera dinámicamente en el idioma configurado.

## Save States

- 10 slots (0..9) por ROM, guardados en `saves/<sanitized_label>.state.<slot>`.
- Cada slot incluye una miniatura BMP (`<path>.thumb.bmp`) de 320×224.
- **Gestor de Save States** (`Ctrl+F12`): overlay full-screen con:
  - Lista de slots con indicador de datos, tamaño de archivo, timestamp.
  - Miniatura del slot seleccionado.
  - Acciones: guardar (F9), cargar (F10), eliminar (Supr).
  - Highlight del slot activo.
- **Indicador rápido** (esquina superior derecha):
  - Cuadro cian = slot actual.
  - Cuadro verde = slot con datos.
  - Cuadro gris = slot vacío.
- **Auto-save al salir** + **auto-load al iniciar**: sesión reanudable sin intervención.

## Verificación

```powershell
# Compilar todo en release (genera target/release/ngneon-emu.exe)
cargo build --release --workspace

# Ejecutar todos los tests (260 tests actualmente)
cargo test --workspace

# Formateo y linting
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings

# Tests específicos
cargo test -p frontend -- ui::tests           # UI (notificaciones, overlays, fuente)
cargo test -p core-emulator -- rom::tests::    # ROM loader
cargo test -p core-emulator -- video::tests::  # Video
cargo test -p core-emulator -- memory::tests::rtc_control  # RTC
```

### Binarios generados

Tras compilar en release, los ejecutables están en `target/release/`:

| Binario | Descripción |
|---|---|
| `ngneon-emu.exe` | Emulador principal con frontend SDL2+OpenGL (4.5 MB) |
| `probe_rom.exe` | Sonda headless para diagnóstico de ROMs |
| `diagnose_rom.exe` | Diagnóstico detallado de bancos ROM `.neo` y `.zip` |
| `make_test_neo.exe` | Genera ROM sintética `.neo` de prueba |
| `render_test_neo.exe` | Renderiza la ROM de prueba sin GUI |
| `profile_resampler.exe` | Benchmark del resampler de audio |

### Resumen de tests

```
core-emulator (lib):  178 passed
frontend (lib):        30 passed
ngneon-emu (bin):      52 passed
Total:                260 passed, 0 failed
```