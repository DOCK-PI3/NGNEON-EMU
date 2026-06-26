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

## Estado actual

- Avance de ROMs reales contrastado con FBNeo y Geolith/libretro:
  - `.neo`: el loader conserva el bloque C-ROM en el orden crudo que usa Geolith (`P,S,M,V1,V2,C` y C intercalado por byte). Ya no se reordena a un layout interno artificial.
  - `.zip`: los pares C1/C2, C3/C4... se cargan intercalados byte a byte, igual que espera la lectura de sprites del LSPC.
  - Sprites C-ROM: el decoder lee filas de 16x16 con mitad izquierda en `+64`, mitad derecha en `+0`, bits right-to-left y orden de planos `[0,2,1,3]`, coincidiendo con FBNeo `NeoDecodeSprites` y Geolith `geo_lspc_tpix`.
  - Fix S/SFIX: el decoder usa el layout nibble real `0x10,0x18,0x00,0x08` por fila, con pixel bajo/alto por byte, coincidiendo con Geolith y con el resultado preprocesado de FBNeo `NeoDecodeText`.
  - Metal Slug 3 `.zip`: la ruta SMA/CMC42 ya carga `neo-sma`/`green.neo-sma` en `0x0C0000`, P1/P2 desde `0x100000`, aplica word-swap y bitswap SMA, extrae S desde C-ROM a 512 KB y evita el decrypt CMC50 de M1 porque FBNeo no marca este set como `ENCRYPTED_M1`.
  - RTC uPD4990A inicial: `0x380051` acepta comando serial CLK/STB/DATA, `STATUS_A` expone DATA OUT/TP en bits 7/6 y la BIOS deja de quedarse en `CALENDAR ERROR`.
- Frontend SDL2 nativo con ventana escalada, pantalla completa, FPS en el título, carga de ROM en caliente y ejecución directa desde línea de comandos mediante `ngneon-emu`.
- El título muestra modo, FPS, `PC`, `SR`, ciclos CPU, banco P-ROM activo y puertos de input para diagnóstico rápido.
- Demo interna visible y controlable sin requerir BIOS ni ROM comercial.
- Carga de `.neo` NeoSD/TerraOnion con cabecera de 4 KB, partición de bancos `P,S,M,V1,V2,C`, metadata básica, validación y avisos de bancos ausentes.
- Carga básica de `.zip` con detección de chips por nombre y resumen de bancos.
- Puertos de entrada básicos memory-mapped en modo active-low para que el 68000 pueda leer controles.
- `probe_rom` puede mantener inputs durante un número configurable de frames: `probe_rom <rom> [captura.bmp] [frames] [inputs] [input_frames]`, por ejemplo `coin,start 5000`.
- DIP switch/watchdog en `0x300001`, test switch en `0x300081`, latch de sonido en `0x320000`, `STATUS_A` con salida RTC uPD4990A básica en `0x320001` y registros básicos de I/O/latches en `0x3800xx` y `0x3A00xx`.
- CPU de sonido Z80 integrada con `z80emu`: `REG_SOUND` (`0x320000`) guarda el último comando 68k->Z80, dispara NMI pendiente y el Z80 lee ese comando por el puerto `0x00`; la respuesta vuelve por el puerto `0x0C` hacia el 68000.
- Bus Z80 básico: M-ROM fija en `0x0000..0x7FFF`, ventanas banked en `0x8000..0xF7FF`, RAM de trabajo en `0xF800..0xFFFF`, puertos YM2610 en `0x04..0x07`, limpieza de sound latch por `0x00/0xC0` y bank switching NEO-ZMC en `0x08..0x0B` usando el byte alto del puerto, como en hardware.
- Palette RAM básica en `0x400000..0x401FFF`; internamente mantiene dos páginas de 8 KB seleccionadas por los latches `REG_PALBANK0/1`. El renderer toma colores desde la página activa y decodifica el formato NeoGeo `D R0 G0 B0 R4..R1 G4..G1 B4..B1` con dark bit. La P-ROM de prueba escribe colores NeoGeo reales vía CPU.
- VBlank IRQ1 autovectorizada al final de cada frame ROM.
- Carga opcional de BIOS real desde `bios/`, con soporte para ZIPs y normalización de byte order. También extrae `000-lo.lo` como tabla L0 de 128 KB para shrink/zoom de sprites y `sfix.sfix` como fix ROM de placa.
- BIOS diagnóstica mínima como fallback en `0xC00000..0xC7FFFF` cuando no hay BIOS real.
- Backup RAM MVS en `0xD00000..0xD0FFFF`.
- Rango de memory card en `0x800000..0xBFFFFF`: con tarjeta ausente devuelve `0xFF` como hardware conocido; con tarjeta insertada usa un buffer de 8 KB espejado y solo permite escritura si el latch de memory card está desbloqueado.
- Bus principal del 68000 normalizado a 24 bits antes del mapa de memoria, evitando aliases fantasma de 32 bits en ROMs reales.
- Registros LSPC/video mínimos en `0x3C0000..0x3C000E`: VRAM address, VRAM read/write, modulo, modo/scanline, timer e IRQ ack.
- `REG_LSPCMODE` expone un contador de auto-animación de 3 bits y el renderer aplica los bits de auto-animación de SCB1 para sustituir los bits bajos del índice de tile en ciclos de 4 u 8 frames.
- Latches de control en `0x3A0001..0x3A001F`: display on/off, cambio de vectores BIOS/cart, selección de audio M1/SM1 y fix S-ROM/SFIX por latches separados, bloqueo SRAM, banco de paleta y estado de memory card. El cambio de vectores sigue el NEO-E0 real: solo conmuta `0x000000..0x00007F` y refleja esa ventana con `0xC00000..0xC0007F`.
- P-ROM fija en `0x000000..0x0FFFFF` y ventana P-ROM banked inicial en `0x200000..0x2FFFFF` para ROMs mayores a 1 MB.
- Latch inicial de banco P-ROM en `0x2FFFF0..0x2FFFFF`, con bancos de 1 MB para pruebas tempranas.
- Visualización básica de C-ROM en matriz diagnóstica con tiles sprite NeoGeo 16x16, 4bpp y 128 bytes por tile. El decoder usa el layout intercalado real de cartucho: bytes `[plane0, plane2, plane1, plane3]`, bit bajo primero y mitad izquierda en el bloque alto del tile.
- Loader ZIP intercala bancos gráficos C-ROM por pares C1/C2, C3/C4... byte a byte para mantener el formato que consume el LSPC.
- Loader `.neo` conserva los bancos C-ROM en el layout intercalado por byte de NeoSD/TerraOnion/Geolith (`C1,C2,C1,C2...`), sin normalización posterior.
- Renderer inicial de sprites desde VRAM: lee SCB1/SCB2/SCB3/SCB4, dibuja por scanline con posición X/Y, altura, tile index, paleta, flip básico, shrink horizontal/vertical, límite de 96 sprites por línea y encadenado sticky simple. El parser de render empieza en sprite 1, ignora sprite 0 y deriva la selección visible directamente desde SCB por cada scanline, como Geolith.
- El shrink vertical ya respeta que SCB3 define una ventana de `16 * height_tiles` líneas; si `000-lo.lo` está disponible y el sprite tiene shrink real (`vshrink != $FF`), usa esa tabla L0 para elegir tile/fila fuente en sprites de hasta 16 tiles de alto, con fallback lineal para casos aún no cubiertos.
- Primera capa fix: lee S-ROM de cartucho o SFIX de placa segun el latch activo, como tiles 8x8 4bpp y mapa VRAM `$7000`, dibujando por encima de sprites con transparencia; incluye guards de paleta y densidad basados en tiles realmente dibujables, no solo en entradas no cero del mapa.
- El mapa de la capa fix usa el layout real por columnas de 32 palabras (`$7000 + columna * 32 + fila`), corrigiendo la posicion de textos/HUD frente al layout row-major inicial.
- La ROM sintética `examples/ngneon_test.neo` ya valida el camino CPU -> LSPC -> VRAM -> sprite/fix -> framebuffer.
- La selección de sprites y sus estadísticas se derivan del recorrido SCB/scanline real; las palabras VRAM `$8600/$8680` no se usan como filtros globales ni se sobrescriben con listas sintéticas.
- YM2610 inicial: registros SSG/FM/ADPCM, timers, lectura de V-ROM para ADPCM y generación de muestras estéreo a la tasa nativa aproximada del chip.
- Frontend SDL2 conecta audio real: el core genera muestras YM2610 siguiendo la cadencia de Geolith (1 muestra cada 72 tstates Z80, patrón MVS 938/939/939), las remuestrea de ~55.5 kHz a 44.1 kHz con un resampler Lanczos/lineal y las entrega a un ring buffer usado por el callback de audio.
- El Z80/YM2610 se sincroniza con el 68K por contadores de ciclo maestro estilo `geo_exec()` (`DIV_M68K=2`, `DIV_Z80=6`, `MCYC_PER_FRAME=405504`), conservando el desfase entre frames. El estado BUSY reinicia su periodo con cada escritura en vez de acumularlo; esto desbloquea los drivers M1 de Metal Slug 3 y Metal Slug X `.neo`, que antes quedaban mudos leyendo `YM_A0 = 0x80`.
- CPU 68000 integrada mediante `m68k`, con mitigación de fallos: se pausa la emulación y el frontend sigue vivo.
- `probe_rom` permite diagnosticar ROMs reales `.neo` y `.zip`, ejecutar frames headless y guardar BMP. `diagnose_rom` usa el mismo criterio por extensión para cargar `.neo` con `RomData::from_neo()` y `.zip` con `RomData::from_zip()`.
- `probe_rom` muestra una traza de bus para encontrar accesos a hardware todavía no mapeado, estado Z80 (`PC`, HALT, T-states, `IFF1/IFF2`, modo IRQ, línea YM, NMI y tamaño M-ROM), estado de timers YM2610 (`0x27`, contadores A/B, BUSY e IRQ), traza de puertos Z80/YM2610 y estadísticas de VRAM derivadas del recorrido LSPC real (`sprites_h`, `visible_sprites`, `gen_lines`, `gen_max`, `gen_overflow`, `h_shrink`, `v_shrink`, `fix_visible`, `fix_drawable`, `fix_unique`, `fix_pixels`, `palette_banks`).
- Gamepad: los botones SDL2 mantienen remapeo persistente por GUID y el stick izquierdo emite estados explícitos de press/release, incluyendo reversas directas izquierda→derecha o arriba→abajo sin dejar direcciones opuestas activas. El overlay de remapeo ignora ejes mientras escucha para evitar drift accidental.
- Icono: el frontend usa un icono NGNEON embebido para la ventana SDL y el build Windows incrusta `frontend/assets/ngneon_icon.ico` como recurso del ejecutable.
- Probado con `roms/diggerma.neo`: el bucle Z80 en `0x00BA` es la espera normal del Timer B, no un bloqueo. En reposo el prototipo permanece mudo; con pulsos tardíos `coin/start/a`, la sonda obtiene audio en 1337/2000 frames, pico `13001` y canales ADPCM-A activos.
- Barrida `.neo` de compatibilidad con entradas tardías: AOF, Metal Slug X, Metal Slug 3 y KOF2003 entran en código de cartucho, muestran escenas reconocibles y producen audio. Tras limpiar las métricas de sprites, AOF reporta `visible_sprites=51`, `gen_max=46`; KOF2003 reporta `visible_sprites=69`, `gen_max=47`; ambos sin overflow y con capturas idénticas byte a byte respecto al renderer anterior.
- Probado con `roms/aof.neo`: carga metadata de Art of Fighting, bancos completos y ejecuta con `neogeo.zip:sp-s2.sp1`; tras el RTC mínimo avanza más allá de `CALENDAR ERROR` hasta la rutina BIOS `PC=0xC11D94`, sin accesos unmapped recientes.
- Probado con `roms/kof2002.zip`: reconoce P/C/M/V y ejecuta con BIOS real, L0, SFIX, Z80 y YM2610 activos; el loader identifica este set como KOF2002 cifrado (`CMC50`/`ENCRYPTED_M1`/`PCM2`) y decripta P-ROM PCM2, M1 CMC50, V-ROM PCM2 V2 y C-ROM/fix CMC50. Con `probe_rom` a 3000 frames llega a título visible (`screenshots/probe_kof2002_zip_3000.bmp`), con `cart_vectors/audio/fix=true`, `sprites_h=23`, `visible_sprites=68`, audio activo y `pbank=0x300000`.
- Probado con `roms/mslugx.neo`: carga metadata de Metal Slug X (`NGH=0x250`) y activa el handler específico de protección/bankswitch en `0x2FFFE0..0x2FFFF0`, contrastado con FBNeo. Con BIOS MVS y RTC mínimo avanza más allá de `CALENDAR ERROR` hasta `PC=0xC11D94`; queda pendiente inicializar correctamente backup RAM/calendario y limpiar el estado fix denso de BIOS para entrar a cartucho con esta BIOS.
- Probado con `roms/mslug3.zip`: el loader reconoce `neo-sma`, `256-pg1.p1`, `256-pg2.p2`, C/M/V y filtra BIOS incluidas; la sonda reporta `P=9437184 C=67108864 S=524288 M=524288 V=16777216`, header `NEO-GEO`, `NGH=0x0256`, Z80 activo y ruta SMA/CMC42. Con `probe_rom` a 900 frames llega a pantalla de título (`screenshots/probe_mslug3_zip_play.bmp`) con sprites, fix y audio.
- Probado con `roms/kof2000.zip`: ruta SMA/CMC50 + M1 CMC50, metadata estilo Geolith (`board=Sma`, `fix=Tile`) y `probe_rom` a 900 frames llega a escena visible con sprites/audio (`screenshots/probe_kof2000_zip_play.bmp`).
- `roms/kog.zip` queda marcado como caso especial/incompleto local: contiene `P/S/C` pero no `M/V`, la P-ROM no presenta cabecera NeoGeo válida tras carga (`header='<no-ascii>'`) y queda en crosshatch. No usarlo como prueba de regresión del loader ZIP general hasta implementar/identificar su protección y set completo.
- Probado también con `NGNEON_BIOS_HINT=uni-bios_4_0`: con el swap de vectores corregido a `$80` bytes y sin mantener `Start`, AOF ya entra en código de cartucho (`PC=0x006794/0x006798`) y KOF2002 entra en código de cartucho (`PC=0x0064EE`). Tras mapear memory card, KOF2002 ya no muestra `0xA9CC06` como unmapped; el siguiente bloqueo visible son cambios repetidos de banco `0x2FFFF0/1`.
- Tras normalizar el C-ROM de `.neo`, corregir Palette RAM, leer el mapa fix por columnas, conectar la tabla L0, aplicar auto-animación SCB1, cargar SFIX y corregir el bit order de sprites, `roms/aof.neo` ya muestra una escena limpia y reconocible. La captura `screenshots/aof_probe_sprite_bitorder_unibios4.bmp` presenta el retrato enmarcado de Art of Fighting sin las diagonales fuertes que quedaban sobre los personajes.

## Referencias contrastadas

- `.neo`: el loader se contrastó con Geolith/libretro (`geo_neo.c`), que valida firma `NEO`, usa cabecera de 4096 bytes y asigna bancos `P,S,M,V1,V2,C` en ese orden.
- `.zip` C-ROM y sprites: FBNeo `NeoDecodeSprites` y Geolith `geo_lspc_tpix` confirman el orden intercalado por byte y los planos `[0,2,1,3]`; el core ya no transforma C-ROM a mitades separadas.
- Fix layer: Geolith `geo_lspc_fixline_default/line` y FBNeo `NeoDecodeText` confirman el layout S/SFIX por nibbles `0x10,0x18,0x00,0x08`; el renderer local ya usa ese formato.
- RTC: Geolith `geo_rtc.c` y FBNeo `neo_upd4990a.cpp` confirman que `0x380051` controla DATA/CLK/STB y que `STATUS_A` devuelve DATA OUT/TP en bits altos.
- BIOS/L0/SFIX: Geolith y FBNeo tratan `000-lo.lo` como ROM auxiliar de zoom/shrink; FBNeo la declara como tabla de 128 KB y su renderer consulta páginas de 256 bytes por valor de shrink vertical. Geolith tambien conmuta la capa fix entre `romdata->sfix` y `romdata->s`, igual que ahora hace el latch local.
- SCB1/auto-animación: Geolith aplica los bits 2/3 del atributo de tile para reemplazar los bits bajos del número de tile usando el contador interno de `REG_LSPCMODE`; FBNeo hace el mismo ajuste antes de leer C-ROM.
- Sprite parser: Geolith recorre sprites desde el índice 1 y corta por límite de sprites por línea; NeoGeoDev documenta el límite de 96 sprites por scanline. El renderer local sigue esa ruta para decidir visibilidad y prioridad básica.
- Sprite graphics: NeoGeoDev documenta tiles 16x16 4bpp en 4 bloques de 8x8, con filas almacenadas right-to-left; Geolith extrae los pixels de C-ROM con `byte >> x`, y FBNeo predecodifica los mismos bytes en dos palabras de 8 pixels por fila. El decoder local se ajustó a ese orden.
- Z80/audio: Geolith documenta el mapa Z80 como M1 fijo `0x0000..0x7FFF`, bancos `0x8000..0xF7FF`, RAM `0xF800..0xFFFF`, puertos de comando/respuesta `0x00/0x0C`, limpieza por `0x00/0xC0`, YM2610 en `0x04..0x07` y bank switching por lecturas de puertos `0x08..0x0B`; `geo_exec()` además sincroniza 68K/Z80/YM con contadores `mcycs/zcycs/ymcycs`, esquema que ahora replica el core local. NeoGeoDev mantiene el mapa de registros 68k, incluyendo `REG_SOUND` en `0x320000`.
- KOF2002 ZIP: FBNeo declara el set `265-*` como `HARDWARE_SNK_CMC50 | HARDWARE_SNK_ENCRYPTED_M1`, sin S1 dedicado porque el texto viene de C-ROM, y aplica `PCM2DecryptP`/`PCM2DecryptV2` además de la ruta CMC50. La ruta local ya reproduce esas protecciones para P/M1/V/C/fix: la traza de KOF2002 pasa de Z80 en `PC=0x003F` sin I/O a Z80 en `PC=0x012E` con 65 accesos recientes a YM2610/bancos, y la S-ROM derivada de C-ROM queda en 512 KB.
- Metal Slug X: FBNeo instala un handler en `0x2FFC00..0x2FFFFF`; `0x2FFFE8` devuelve bits de protección y `0x2FFFF0` selecciona bancos P-ROM con `((valor & 7) + 1) * 0x100000`. El core local activa ese handler para `.neo` con `NGH=0x250`.
- Metal Slug 3: FBNeo usa `green.neo-sma`/`neo-sma` junto a `256-pg1.p1`/`256-pg2.p2` y declara `HARDWARE_SNK_CMC42 | HARDWARE_SNK_SMA_PROTECTION`; por eso el ZIP usa decriptado SMA de P-ROM y CMC42 de C/fix, no la ruta CMC50 de KOF2002.
- `.zip`: el camino actual sigue la convención de sets MAME/FBNeo por chips `p/c/s/m/v`; los sets modernos/protegidos como KOF2002 aún requieren lógica de board/banking/decrypt adicional.

## Punto de continuidad

Última verificación de esta tanda (11 de junio de 2026): `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace -- --test-threads=1` y `cargo build --release --workspace` pasan correctamente. También pasan `diagnose_rom` release con `aof.neo`/ZIPs locales y `probe_rom` release con `kof2000.zip`, `kof2002.zip`, `mslug3.zip`.

El siguiente paso natural es completar la ruta de arranque real hasta una escena visible:

- Corregir estado de arranque MVS tras RTC: ahora AOF/MSlugX ya no muestran `CALENDAR ERROR`, pero entran en una pantalla BIOS/fix muy densa con paleta verde. El siguiente foco es backup RAM/calendario inicial, limpieza de fix durante BIOS y transición a cartucho.
- Refinar MSlugX desde la ruta con RTC: validar si el handler de protección necesita más casos cuando la BIOS ya entregue control al cartucho.
- Ampliar compatibilidad ZIP a más sets MAME/FBNeo: primero distinguir sets completos de hacks incompletos, después añadir metadata/protecciones solo cuando el nombre de chips sea inequívoco.
- Investigar `kog.zip` por separado: confirmar set esperado, protección real y si faltan M/V ROMs antes de tratarlo como objetivo de compatibilidad general.
- Seguir el handshake Z80/YM2610 en AOF: el enable/disable de NMI, el bank switching por byte alto del puerto, la limpieza `0x00/0xC0` y la traza de puertos ya siguen Geolith/FBNeo; falta validar respuestas de M1 reales y estado YM con más detalle.
- Usar la nueva telemetría de `probe_rom` para validar IRQ/timers YM2610 en más drivers M1 y separar estados idle legítimos de bloqueos reales.
- Ampliar el uso de L0 para sprites altos y combinarlo con prioridad/sorting real.
- Refinar la capa fix con bancos `$7500+` para juegos NEO-CMC cuando haya S-ROM válida.
- Avanzar la ejecución de BIOS/juegos reales más allá de las rutinas actuales para que escriban escena completa en VRAM.
- Después de tener imagen real estable, profundizar en temporización fina de VBlank/HBlank, mezcla YM2610 más precisa y save RAM persistente.
