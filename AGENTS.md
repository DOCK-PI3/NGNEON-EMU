# AGENTS.md

## Propósito

Este archivo guía a los agentes de IA para ser productivos en el proyecto **NGNEON-EMU**. Aquí se documentan convenciones, comandos clave y decisiones arquitectónicas relevantes.

---

## Resumen del Proyecto

- **NGNEON-EMU** es un emulador NeoGeo multiplataforma con un núcleo de emulación en Rust y frontend SDL2+OpenGL.
- **Idiomas:** Español (por defecto) e Inglés, toggle con Ctrl+L, persistente en config.
- **Config:** Sistema unificado `config/ngneon.conf` + perfiles por juego en `config/profiles/` + `config/keyboard.conf` + `config/gamepad/<guid>.conf`.

---

## Comandos de Build y Test

```powershell
# Compilar todo (release)
cargo build --release --workspace

# Compilar solo núcleo
cargo build --release --manifest-path core-emulator/Cargo.toml

# Ejecutar emulador nativo
cargo run --release --bin ngneon-emu -- --demo
cargo run --release --bin ngneon-emu -- roms/aof.neo

# Tests
cargo test --workspace
cargo test -p core-emulator rom::tests::
cargo test -p core-emulator video::tests::

# Tests específicos de frontend
cargo test -p frontend -- input::tests
cargo test -p frontend -- gamepad::tests

# Verificación completa
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

---

## Estructura del Frontend

| Archivo | Propósito |
|---|---|
| `frontend/src/main.rs` | Entry point, event loop, config persistence, save/load states, BIOS selector, kb config overlay, gamepad config overlay, auto-save/mute gating |
| `frontend/src/ui.rs` | Renderizado de overlays (debug, notifications, BIOS selector, save manager, slot indicator, settings menu 6 pestañas, profile config, gamepad config, **keyboard config**) |
| `frontend/src/lang.rs` | Strings traducibles Español/Inglés + método `usage_text()`. ~80 campos incluyendo labels para AUDIO, PATHS, CONTROLS, Keyboard Config, paths, mute, auto-save, gamepad |
| `frontend/src/gl_render.rs` | Pipeline CRT: curvature, scanlines, bloom (OpenGL 3.3) |
| `frontend/src/audio.rs` | Resampler Lanczos-3 + RingBuffer SDL2 |
| `frontend/src/input.rs` | Mapeo teclado → EmuAction, **KeyboardMapping** con persistencia en `config/keyboard.conf`, global static con Mutex, keycode_name/find_keycode_by_name con ~90 variantes SDL2 |
| `frontend/src/gamepad.rs` | **GamepadManager** con hotplug, analog stick emulation, **ControllerMapping** con persistencia en `config/gamepad/<guid>.conf`, 15 botones SDL2 + acciones globales, tests |
| `frontend/src/screenshot.rs` | Carga de BMP (para thumbnails de save states) |

---

## Atajos de Teclado

| Tecla | Acción |
|---|---|
| Esc | Salir / cerrar overlay |
| F1 | Cargar ROM desde diálogo |
| F2 / Shift+F2 | Toggle / Reset scanlines |
| F3 / Shift+F3 | Toggle / Reset CRT curvature |
| F4 / Shift+F4 | Toggle / Reset phosphor bloom |
| F5 | Demo interna |
| F6 | Debug overlay |
| F7 / Shift+F7 | Slot anterior / siguiente |
| F8 | Reset CPU |
| F9 | Save state (slot actual) |
| F10 | Load state (slot actual) |
| F11 | Fullscreen |
| F12 / Ctrl+F12 | Screenshot / Save State Manager |
| P | Pause / Resume |
| Ctrl+B | BIOS Selector |
| Ctrl+G | Configurar gamepad |
| Ctrl+L | Toggle language |
| Ctrl+M | Toggle mute / unmute |
| Ctrl+= | Volume up (+5%) |
| Ctrl+- | Volume down (-5%) |
| Ctrl+O | ROM Browser (grid con box art) |
| Ctrl+S | Settings menu (6 pestañas: VIDEO/AUDIO/SYSTEM/CONTROLS/PATHS/RA) |
| — | **Dentro de Settings → CONTROLS:** |
| Enter | Toggle Gamepad SDL2 o abrir sub-overlay: Gamepad Config / **Keyboard Config** |
| — | **Dentro de Keyboard Config:** |
| ↑/↓ | Navegar acciones |
| Enter | Iniciar escucha de tecla para reasignar |
| Esc | Cancelar escucha / cerrar overlay |
| R | Restaurar binding por defecto de la acción seleccionada |
| — | **Dentro de Gamepad Config (Ctrl+G):** |
| ↑/↓ | Navegar acciones |
| Enter | Iniciar escucha de botón para reasignar |
| Esc | Cancelar escucha / cerrar overlay |
| R | Restaurar defaults de todos los botones |

---

## Sistema de Configuración

### Config global (`config/ngneon.conf`)

```
bios=neogeo.zip:sp-s2.sp1
bios_dir=bios
media_dir=media
lang=es
scanlines=on
curvature=off
bloom=on
fullscreen=off
rom_path=roms/aof.neo
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

**Clave `bios_dir`**: Especifica el directorio donde se buscan los archivos de BIOS (ZIPs como `neogeo.zip`/`aes.zip`). El emulador resuelve la ruta con prioridad: `bios_dir` en config → `<exe>/bios` → `./bios` → fallback `bios/`. La clave `bios` guarda el **label** de la BIOS activa (ej. `neogeo.zip:sp-s2.sp1`), no la ruta.

**Clave `media_dir`**: Especifica el directorio para carátulas/cartuchos del ROM Browser. Por defecto usa `media/`. Las imágenes son `.png` con el mismo nombre base que la ROM (`roms/aof.neo` → `media/aof.png`) y se ajustan proporcionalmente al recuadro 150×84. El ROM Browser usa una cuadrícula 2×2 para priorizar legibilidad del cartucho y se renderiza sin CRT/scanlines/bloom para conservar nitidez en carátulas y texto.

**RetroAchievements**: `ra_token` tiene prioridad para login automático. Si no hay token, el frontend usa `ra_username` + `ra_password` como fallback mediante `rc_client_begin_login_with_password()`. `ra_hardcore` persiste el modo hardcore.

**Clave `gamepad`**: Activa/desactiva la inicialización SDL2 GameController (`on`/`off`). Por seguridad el paquete release la deja en `off`, evitando bloqueos de arranque en algunos backends de mando de Windows. Se puede cambiar desde **Ctrl+S → CONTROLS → Gamepad SDL2** y requiere reiniciar para aplicar.

### Config de teclado (`config/keyboard.conf`)

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

- Formato: `ActionName=SDLK_SCANCODE`.
- `KeyboardMapping::save_to_config()` / `load_from_config()` en `input.rs`.
- Global static protegido por `Mutex`, accesible desde cualquier overlay.
- Soporta ~90 variantes de teclas SDL2 (alfanuméricas, puntuación, F1-F24, numpad, media, browser, sistema).

### Config de gamepad (`config/gamepad/<sanitized_guid>.conf`)

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

- Formato: `button_name=action_name` (15 botones SDL2 → 10 acciones NeoGeo).
- Acciones globales por controller: `SystemExit` sale sin preguntar; `SystemRomBrowser` abre el ROM Browser. Aceptan `Button` o `Button+Button`.
- **Un archivo por controller** (identificado por GUID sanitizado).
- `GamepadManager::save_all()` / `load_mapping_for_guid()` en `gamepad.rs`.
- Carga automática al iniciar (`scan_initial()`) y en cada hotplug connect.
- Guardado al cerrar overlay, restaurar defaults, reasignar botón, y al salir del programa.
- Directorio `config/gamepad/` creado automáticamente si no existe.

### Config per-game (`config/profiles/<label>.conf`)

```
scanlines=on
curvature=on
bloom=off
```

**Helpers disponibles en `main.rs`:**
- `save_config_key(key, value)` / `load_config_key(key)` — genéricos
- `save_config_bool(key, bool)` / `load_config_bool(key, default)` — booleanos
- `save_config_bios(bios_label)` / `load_config_bios()` — persistencia del label de BIOS
- `resolve_bios_directory()` — resuelve el directorio BIOS (config → exe_dir → cwd → fallback)
- `save_game_key(label, key, value)` / `load_game_key(label, key)` — per-game
- `save_game_bool(label, key, bool)` / `load_game_bool(label, key, default)`
- `delete_game_key(label, key)` — elimina clave del perfil
- `apply_game_config(label, crt_gl)` — aplica overrides CRT del perfil
- `apply_startup_config(crt_gl, status, window)` — aplica globales al iniciar

Los helpers preservan otras claves: leen el archivo, filtran la línea a reemplazar, y reescriben.

---

## Save States

- 10 slots por ROM en `saves/<sanitized_label>.state.<slot>`.
- Miniaturas BMP en `<path>.thumb.bmp` (320×224).
- Slots 0–9 con timestamp y tamaño visible.
- Auto-save slot 0 al salir, auto-load slot 0 al iniciar.

---

## Localización (idiomas)

Todas las strings visibles están en `frontend/src/lang.rs`:

- `Lang::spanish()` — Español
- `Lang::english()` — English
- `Lang::usage_text()` — texto de `--help` bilingüe
- Añadir nuevo campo: agregar a `Lang` struct + ambos constructores + referencias en `ui.rs`

---

## Convenciones y Notas

- Mantener la lógica de emulación desacoplada de la UI.
- Si se usa el `notification` system, pasar strings desde `Lang` con `.replacen("{}", &value, 1)`.
- Los overlays se dibujan en el framebuffer antes del upload a GPU.
- Los efectos CRT corren en shader GLSL 330 core (no CPU).
- Documentar cualquier workaround multiplataforma (ej. diferencias de SDL2 en Windows/Linux).

---

## Documentación Relacionada

- [README.md](README.md) — documentación de usuario
- [implementation_plan.md](implementation_plan.md) — plan de implementación con detalles técnicos
- [SKILL.md](SKILL.md) — skills para agentes (Keyboard Config, Gamepad, UI, etc.)

---

## Menú de Configuración (Ctrl+S) — 6 Pestañas

| Tab | Índice | Opciones |
|---|---|---|
| VIDEO | 0 | Scanlines, Curvature, Bloom, Fullscreen, Window Scale (2x/3x/4x) |
| AUDIO | 1 | Volume (**modo ajuste dedicado**), Mute (ON/OFF) |
| SYSTEM | 2 | Language (ES/EN), Diagnostic Dumps, Auto-save, BIOS Selector (`>`) |
| CONTROLS | 3 | Gamepad SDL2 (ON/OFF, requiere reinicio), Gamepad Config (`>`), **Keyboard Config** (`>`) |
| PATHS | 4 | ROMs, BIOS, Screenshots, Saves (read-only, rutas resueltas) |

Navegación: `←`/`→` **siempre** cambia de pestaña. `↑`/`↓` navega opciones, `Enter` toggle/abrir submenú.

### Modo de ajuste de volumen (AUDIO tab)
- Presiona **Enter** sobre «Volume» para entrar al modo de ajuste dedicado.
- Dentro del modo: `←`/`→` ajustan volumen ±5%. **Enter** o **Esc** salen del modo.
- Navegar con `↑`/`↓` fuera de la opción de volumen **sale automáticamente** del modo de ajuste.
- La barra de volumen muestra bloques `█` llenos y `░` vacíos (glifos 58/59 de la fuente).

### Keyboard Config Overlay (accesible desde Settings → CONTROLS)

- Lista de 10 acciones remapeables: Up, Down, Left, Right, A, B, C, D, Start, Coin.
- **Enter** sobre una acción → modo escucha: pulsa una tecla para asignarla.
- **Esc** cancela la escucha.
- **R** restaura el binding por defecto de la acción seleccionada.
- Las teclas se guardan en `config/keyboard.conf` (scancodes SDL2).
- Indicador de pulso visual durante la escucha.
- Se eliminan duplicados automáticamente: al asignar una tecla ya usada, se libera de su acción anterior.
- Los bindings se aplican globalmente a través de `input::process_event()`.

### Gamepad Config Overlay (Ctrl+G)

- Lista de 10 acciones NeoGeo (Up/Down/Left/Right/A/B/C/D/Start/Coin).
- Soporte multi-controller: muestra todos los gamepads conectados.
- **Enter** inicia modo escucha de botón SDL2 (pulso visual).
- **R** restaura todos los bindings a defaults.
- **Esc** sale del modo escucha o cierra el overlay.
- Persistencia automática en `config/gamepad/<guid>.conf` (por GUID de controller).
- Hotplug: detecta conexión/desconexión sin reiniciar.

## Keyboard Mapping (`input.rs`)

- **`KeyboardMapping`**: struct con `HashMap<Keycode, EmuAction>`.
- **`default()`**: flechas + ZXCV + Enter + Space.
- **`set(keycode, action)`**: asigna y elimina binding previo de esa acción.
- **`process(&self, keycode) -> Option<EmuAction>`**: lookup.
- **`keycode_name(kc) -> &str`**: ~90 variantes SDL2 (A-Z, 0-9, F1-F24, flechas, numpad, puntuación, media, browser, sistema).
- **`find_keycode_by_name(name) -> Option<Keycode>`**: parseo inverso para roundtrip save/load.
- **Persistencia**: `save_to_config("config/keyboard.conf")` + `load_from_config()`.
- **Global static**: `GLOBAL_KEYBOARD_MAPPING: Mutex<KeyboardMapping>` + `set_global_mapping()` / `get_global_mapping()`.
- **Tests**: 5 tests (default map, roundtrip keycode_name↔find, save/load, set con duplicados, roundtrip de teclas comunes).

## Gamepad Manager (`gamepad.rs`)

- **`GamepadManager`**: struct con controllers, mappings, stick state, GUIDs.
- **`ControllerMapping`**: array de 15 `EmuAction` indexado por SDL2 `Button` discriminant + 2 chords globales (`SystemExit`, `SystemRomBrowser`).
- **`scan_initial(sub)`**: detecta y abre todos los controllers ya conectados, cargando sus mappings.
- **`handle_hotplug(sub, event)`**: procesa eventos `ControllerDeviceAdded`/`Removed`.
- **`process_event(event)`**: traduce eventos SDL2 (botones + ejes analógicos) a `EmuAction`.
- **`process_event_actions(event)`**: ruta recomendada para input real; devuelve eventos explícitos `{ action, pressed }`, permitiendo que un cambio directo de stick izquierda→derecha suelte la dirección previa antes de presionar la nueva.
- **`save_all()`**: persiste todos los mappings a `config/gamepad/<guid>.conf`.
- **`load_mapping_for_guid(guid)`**: carga mapping desde disco o devuelve `default_mapping()`.
- **`button_name()`** / **`find_button_by_name()`**: roundtrip para 15 botones.
- **`action_name()`** / **`find_action_by_name()`**: roundtrip para 10 acciones NeoGeo.
- **Tests**: cubren default mapping, button/action roundtrip, set/get, deadzone, reversa directa del stick, release al centrar stick y chords globales.

## Fuente Bitmap del Overlay

- **60 glifos** (expandido desde 34): A-Z (0-25), `↑` (26), `↓` (27), `←` (28), `→` (29), `>` (30), `:` (31), `.` (32), `/` (33), `-` (34), `(` (35), `)` (36), espacio (37), `…` (38), `0`-`9` (39-48), `!` (49), `'` (50), `"` (51), `á` (52), `é` (53), `í` (54), `ó` (55), `ú` (56), `ñ` (57), `█` (58), `░` (59).
- Minúsculas (a-z, ASCII 97-122) mapean automáticamente a mayúsculas (índices 0-25).
- Caracteres acentuados españoles (áéíóúñ) tienen glifos propios.
- Bloques `█`/`░` se usan para la barra de volumen en el menú de configuración.

---

## Correcciones Recientes (Mayo 2026)

### Fix 1: "Codec RAM error" — ADPCM-A/B RAM buffers

**Archivo:** `core-emulator/src/memory.rs`

**Problema:** La BIOS MVS escribe patrones de test en la RAM interna del YM2610 a través del bus 68k en `0x300000-0x303FFF` (ADPCM-A) y `0x310000-0x313FFF` (ADPCM-B). El código anterior tenía `INPUT_P1_PORT = 0x300000` que interceptaba lecturas, y las escrituras iban a la nada. El test de BIOS fallaba porque no podía leer de vuelta los patrones escritos.

**Solución:**
- Añadidos buffers `adpcm_a_ram: Vec<u8>` (16KB) y `adpcm_b_ram: Vec<u8>` (16KB) al struct `Memory`.
- Inicializados a cero en `Memory::new()` y reseteados en `load_rom()` con `fill(0)`.
- `read8()`: mapea `0x300002..=ADPCM_A_END` → ADPCM-A RAM, `ADPCM_B_START..=ADPCM_B_END` → ADPCM-B RAM.
  - `INPUT_P1_PORT` (0x300000), `DIPSW_PORT` (0x300001) y `TEST_SWITCH_PORT` (0x300081) se manejan ANTES del rango ADPCM-A.
- `write8()`: mapea `ADPCM_A_START..=ADPCM_A_END` (0x300000-0x303FFF) → ADPCM-A RAM, `ADPCM_B_START..=ADPCM_B_END` → ADPCM-B RAM.
  - El orden en write8 pone ADPCM_A **antes** que DIPSW_PORT (que ahora es código muerto en write8, ya que 0x300001 cae dentro del rango ADPCM-A). Esto es correcto porque DIP switches son read-only en hardware real.
- **Savestate**: serialización de ambos buffers añadida a `mem_sections` y `mem_dests` en `savestate.rs` (arrays de 5→7 elementos).

### ~~Fix 2: Fix layer 8x16 tile mode~~ (REVERTIDO — Junio 2026)

**Archivo:** `core-emulator/src/video.rs`

**Problema del fix anterior:** Interpretaba el bit 5 de `LSPC_MODE` (`0x3C0006`, valor `0x0020`) como selector de tiles FIX de 8x16. En Geolith y en el hardware ese bit forma parte del control del temporizador IRQ; la capa FIX sigue usando tiles de 8x8. Cuando juegos como Strikers 1945 Plus mantienen `0x0020`, NGNEON agrupaba dos tiles como uno, omitía filas del mapa y mostraba fondos/logotipos como bandas repetidas.

**Corrección:**
- La capa FIX vuelve a usar siempre tiles de 8x8 y 28 filas visibles.
- `render_fix_scanline()`, `render_fix_layer()` y `fix_debug_stats()` ya no cambian la geometría según `LSPC_MODE`.
- Añadido el test `lspc_timer_bit_does_not_change_fix_tile_height()`.
- Verificado contra Geolith con VRAM y paleta idénticas en `s1945p.neo`.

### Fix 3: Verificación del registro LSPC RomSize (0x3C000C) contra crom_mask

**Archivos:** `core-emulator/src/video.rs`, `core-emulator/src/memory.rs`

**Problema:** El registro LSPC RomSize (`0x3C000C`) define la ventana de direcciones C-ROM que el juego espera, pero NGNEON ignoraba su valor y solo usaba `calc_crom_mask()` basada en el tamaño real de datos. Geolith en `geo_lspc_postload()` verifica el registro contra la máscara calculada y usa el valor del registro para el wrap-around de tiles.

**Solución (3 partes):**

**Parte A — Captura del registro en memory.rs:**
- Añadido campo `lspc_rom_size: u16` al struct `Memory`.
- Capturado en `write_lspc_register()` cuando se escribe a `LSPC_IRQACK` (`0x3C000C`): el mismo registro tiene doble propósito (IRQ acknowledge + RomSize).
- Inicializado a `0` en `Memory::new()`, reseteado en `load_rom()`.

**Parte B — Funciones de verificación en video.rs:**
- `calc_crom_mask(crom_size)`: calcula máscara `next_power_of_two(tiles) - 1` desde el tamaño real de datos. Equivale a `geo_calc_mask(32, csz>>7)` de Geolith.
- `register_to_crom_mask(rom_size)`: convierte valor del registro LSPC RomSize a máscara: `(1 << ((rom_size & 0x1F) + 12)) - 1`. Con clamp seguro para evitar panic en 32-bit.
- `verify_crom_mask_register(memory)`: compara máscara del registro contra datos. Devuelve false con warning en stderr si el juego espera más C-ROM del disponible. Llamada desde `render_frame()`.

**Parte C — decode_sprite_tile() modificado:**
- Ahora acepta tercer parámetro `lspc_rom_size: u16`.
- Usa `register_to_crom_mask(lspc_rom_size)` cuando el registro está inicializado, fallback a `calc_crom_mask(crom.len())` cuando no.
- Esto replica exactamente el comportamiento de Geolith: la ventana de direcciones la define el juego, no el tamaño real de datos.
- Call sites actualizados: `draw_sprite_scanline()`, `render_crom_diagnostic_matrix()`, y 6 tests.

**Tests:** 7 nuevos tests cubriendo: calc_crom_mask (6 valores), register_to_crom_mask (7 valores), verify_crom_mask_register (4 casos), decode_sprite_tile wrap-around (5 valores).

### Fix 4: Runtime SMA detection — validación por tamaño de P-ROM

**Archivo:** `core-emulator/src/rom.rs`

**Problema:** NGNEON asignaba `NeoBoardType::Sma` basándose únicamente en el NGH, pero algunos NGH colisionan entre variantes SMA y no-SMA (ej. NGH 0x251 puede ser KOF 99 cifrado SMA o KOF 98 descifrado). Esto causaba que ROMs descifradas pequeñas fueran tratadas incorrectamente como SMA.

**Solución:**
- Añadida función `validate_sma_board_type(board_type, prom) -> NeoBoardType`.
- Si `board_type == Sma` y `prom.len() <= 0x500000` (5MB), se degrada a `NeoBoardType::Default`.
- Esto replica la heurística de Geolith: los cartuchos SMA reales siempre tienen P-ROM > 5MB.
- Llamada desde `from_neo()` después de `detect_neo_board_type()`.

### Fix 5: Parser .neo acepta versiones alternativas (0x00, 0x02, 0x03, 0x05)

**Archivo:** `core-emulator/src/rom.rs`

**Problema:** El parser de `.neo` solo aceptaba la versión 0x01, rechazando ROMs creadas con otras versiones del formato (legacy 0x00, o versiones alternativas 0x02, 0x03, 0x05).

**Solución:**
- Ampliado el check de versión en `parse_neo_file()`: de `version != 0x01` a `!matches!(version, 0x00 | 0x01 | 0x02 | 0x03 | 0x05)`.
- Mensaje de error actualizado: ahora indica `"esperada 0x00-0x03 o 0x05"`.
- El campo `metadata.version` preserva el valor original de la cabecera sin importar qué versión aceptada se use.
- Ningún código downstream depende del valor de versión, así que no hay riesgo de breaks.

### Fix 6: Detección de bootleg KOF2003 (NGH 0x271) por inspección de P-ROM

**Archivo:** `core-emulator/src/rom.rs`

**Problema:** NGNEON asignaba `NeoBoardType::Pvc` a todos los ROMs con NGH 0x271 (KOF 2003), sin distinguir entre el lanzamiento oficial NEO-PVC y las variantes bootleg (`kf2k3bla`, `kf2k3bl`, `kf2k3pl`, `kf2k3upl`).

**Solución:**
- Añadida función `validate_kof2003_board_type(ngh, board_type, prom) -> NeoBoardType`.
- Inspecciona bytes específicos del P-ROM (offsets relativos al inicio, equivalentes a `neodata[0x1000 + offset]` en Geolith):
  - `prom[0x689] == 0x10` → `NeoBoardType::Kf2k3Bla` (bootleg sets `kf2k3bla`, `kf2k3pl`)
  - `prom[0xc1] == 0x02` → `NeoBoardType::Kf2k3Bl` (bootleg sets `kf2k3bl`, `kf2k3upl`)
  - Ninguno → mantiene `NeoBoardType::Pvc` (lanzamiento oficial)
- Llamada desde `from_neo()` después de `validate_sma_board_type()`.
- Emite mensaje `[INFO]` en stderr cuando se detecta un bootleg.

### Fix 7: NEO-PVC bankswitching — soporte para KOF 2003, MS5, SVC Chaos

**Archivos:** `core-emulator/src/memory.rs`, `core-emulator/src/savestate.rs`

**Problema:** KOF 2003 (NGH 0x271) no arrancaba correctamente — se quedaba en la pantalla verde de test de BIOS con texto ilegible. El problema raíz era que el emulador detectaba `board_type = Pvc` pero no tenía implementado el bankswitching NEO-PVC que usan los juegos de la placa NEO-PVC (KOF 2003, Metal Slug 5, SVC Chaos).

**Solución (3 áreas):**

**Parte A — CartProtection::Pvc y cartram en memory.rs:**
- Añadida variante `CartProtection::Pvc` al enum `CartProtection`.
- Constantes `PVC_CARTRAM_START` (0x2FE000) y `PVC_CARTRAM_SIZE` (0x2000 = 8KB).
- Campos `pvc_cart_ram: Vec<u8>` (8KB cart RAM) y `pvc_bank_addr: usize` (dirección de banco calculada) al struct `Memory`.
- Inicializados en `Memory::new()` y reseteado en `load_rom()` (`pvc_cart_ram.fill(0)`, `pvc_bank_addr = 0x100000`).

**Parte B — Operaciones PVC (unpack/pack/bankswap):**
Replican exactamente la lógica de Geolith (`geo_m68k_pvc_unpack/pack/bankswap`):
- `pvc_unpack()`: extrae R,G,B,D de `cartram[0x1FE0-0x1FE1]` y los expande en `cartram[0x1FE2-0x1FE5]`. Se dispara con escrituras a `0x2FFFE0-0x2FFFE3`.
- `pvc_pack()`: comprime R,G,B,D de `cartram[0x1FE8-0x1FEB]` en `cartram[0x1FEC-0x1FED]`. Se dispara con escrituras a `0x2FFFE8-0x2FFFEB`.
- `pvc_bankswap()`: calcula `bankaddress` de 24 bits desde `cartram[0x1FF1-0x1FF3]`, lo almacena en `pvc_bank_addr` (con `+0x100000` y máscara `0xFFFFFF`), y escribe valores fijos (`0xA0`, `&= 0xFE`, `&= 0x7F`). Se dispara con escrituras a `0x2FFFF0-0x2FFFF3`.
- `write_pvc_cartram_byte(addr, value)`: escribe en cartram con XOR `^1` (byte-swapped access), y dispara unpack/pack/bankswap según la dirección base.

**Parte C — Integración en read/write handlers:**
- `read8()`: lecturas a PVC cartram (`>= 0x2FE000`) van a `pvc_cart_ram` con XOR `^1`. Banked PROM usa `pvc_bank_addr` en vez de `prom_bank_offset` para PVC.
- `read16()`/`read32()`: mismo patrón con fast path para banked PROM usando `pvc_bank_addr`.
- `write8()`: escrituras a PVC cartram capturadas ANTES del handler genérico `PROM_BANK_REGISTER`. El handler de `PROM_BANK_REGISTER` se excluye para PVC (`cart_protection != Pvc`).
- `write16()`/`write32()`: zona PVC delegada byte-by-byte a `write8()`.
- `detect_cart_protection()`: devuelve `CartProtection::Pvc` para board types `Pvc`, `Kf2k3Bla`, y `Kf2k3Bl`.

**Savestate (version 2 → 3):**
- `pvc_cart_ram` añadido a `mem_sections` y `mem_dests` (arrays de 7→8 elementos).
- `pvc_bank_addr` serializado como nueva sección 6 (4 bytes), condicionado a `version >= 3` para backward compatibility.

### Fix 8: Tile-based fix bankswitching — KOF2000, Matrimelee, SVC, KOF2003

**Archivos:** `core-emulator/src/video.rs`, `core-emulator/src/memory.rs`

**Problema:** Los juegos que usan bankswitching por tile en la capa fix (KOF2000, Matrimelee, SVC, KOF2003) no tenían soporte en el emulador. Solo existía `FixBankSwitch::Line` (per-scanline) y `FixBankSwitch::None`. Esto causaba que la capa fix (HUD, texto, menús) se renderizara con tiles incorrectos.

**Solución:**
- Añadida variante `FixBankSwitch::Tile` al enum `FixBankSwitch`.
- `detect_fix_bankswitch()`: mapea NGH 0x0257 (KOF2000), 0x0266 (Matrimelee), 0x0269 (SVC), 0x0271 (KOF2003) → `FixBankSwitch::Tile`.
- Nueva función `fix_tile_bank(memory, visible_row, col) -> usize`:
  - Calcula el banco desde `VRAM[0x7500 + ((row-1) & 0x1F) + 32*(col/6)]`.
  - Extrae 2 bits según la posición del tile en su grupo de 6 columnas.
  - Aplica complemento bitwise (`!bank_bits & 0x03`), replicando `geo_lspc_fixline_tile()` de Geolith.
- `fix_tile_index()` actualizada: acepta parámetro `col` y usa `fix_tile_bank()` para `FixBankSwitch::Tile`.
- 3 call sites actualizados: `render_fix_scanline()`, `render_fix_layer()`, y `fix_debug_stats()`.

**Resultado:** KOF 2003 arranca correctamente (ya sin pantalla verde de BIOS). 102/102 tests del núcleo pasan.

### Fix 9: PVC atomic write — write16/write32 atómicos para NEO-PVC

**Archivo:** `core-emulator/src/memory.rs`

**Problema:** Cuando el 68k escribía 16-bit a los registros PVC (`0x2FFFE0` para unpack, `0x2FFFE8` para pack, `0x2FFFF0` para bankswap), `write16()` dividía la escritura en dos `write8()`. Cada `write8()` a la zona PVC disparaba la operación PVC (unpack/pack/bankswap) **inmediatamente** con datos incompletos, causando una corrupción en cadena:

1. `write8(0x2FFFF0, hi_byte)` → escribe 1 byte → dispara `pvc_bankswap()` leyendo **bytes stale** de `cartram[0x1FF2]` y `[0x1FF3]` → calcula dirección de banco **incorrecta**.
2. `pvc_bankswap()` **modifica** `cartram[0x1FF0]=0xA0`, `[0x1FF1]&=0xFE`, `[0x1FF3]&=0x7F`.
3. `write8(0x2FFFF1, lo_byte)` → sobreescribe `cartram[0x1FF0]` (ya corrompido por el paso 2) → dispara otro bankswap con datos corruptos.

El resultado era una excepción de UniBIOS (pantalla de error con "EXCEPTION ERROR" e "ILLEGAL INSTRUCTION") y texto NEOGEO del boot ilegible porque la capa fix leía del banco SROM incorrecto.

**Solución:**
- Añadido método `pvc_trigger_operation(addr)` que extrae la dirección base (`addr & !3`) y dispara unpack/pack/bankswap según corresponda.
- Añadido método `write_pvc_cartram_16(addr, value)`: escribe **ambos bytes** a `pvc_cart_ram` (con XOR `^1` para byte-swapped access), registra los accesos al bus, y **luego** dispara la operación PVC **una sola vez**. Réplica exacta de `geo_m68k_write_banksw_16_pvc` de Geolith.
- Añadido método `write_pvc_cartram_32(addr, value)`: igual para escrituras 32-bit — escribe 4 bytes primero, luego dispara operaciones para cada región de 4 bytes.
- `write_pvc_cartram_byte()` refactorizado para delegar el trigger a `pvc_trigger_operation()`.
- `write16()`: para la zona PVC (`0x2FE000-0x2FFFFF`), usa `write_pvc_cartram_16()` en vez de dividir en dos `write8()`.
- `write32()`: para la zona PVC, usa `write_pvc_cartram_32()` en vez de dividir en cuatro `write8()`.

**Resultado:** KOF 2003 arranca sin excepción de UniBIOS. El texto NEOGEO del boot se renderiza correctamente. 102/102 tests pasan.

### ~~Fix 10~~ (REVERTIDO — Mayo 2026): `pvc_bank_addr = 0` — ERA INCORRECTO

**Archivo:** `core-emulator/src/memory.rs`

**⚠️ Este fix fue revertido.** Cambió `pvc_bank_addr` de `0x100000` a `0`, asumiendo que la ventana banked debía espejar la ventana fija antes del primer bankswap. Esto era incorrecto: Geolith (`geo_m68k_reset()`) inicializa `banksw_addr = 0x100000` cuando `psz > 0x100000`, haciendo que la ventana banked (`0x200000`) apunte al **segundo megabyte** del P-ROM, no al primero. Con `pvc_bank_addr = 0`, los juegos PVC solo veían el primer megabyte en ambas ventanas (fixed + banked) y no encontraban el código del segundo megabyte → pantalla verde. **Reemplazado por Fix 11.**

### Fix 11: `pvc_bank_addr` initialization — valor correcto `0x100000` (Mayo 2026)

**Archivo:** `core-emulator/src/memory.rs`

**Problema:** El Fix 10 (revertido) puso `pvc_bank_addr = 0`, causando que la ventana banked (`0x200000-0x2FFFFF`) espejara el primer megabyte del P-ROM (ya visible en `0x000000-0x0FFFFF`). Geolith inicializa `banksw_addr = 0x100000` cuando el P-ROM supera 1MB, haciendo que la ventana banked apunte al **segundo megabyte**. KOF2003 (9MB), SVC Chaos (8MB) y Metal Slug 5 (8MB) necesitan acceder al código del segundo megabyte vía la ventana banked.

**Solución:**
- `Memory::new()`: `pvc_bank_addr` se mantiene en `0` (valor por defecto seguro; `prom` está vacío en construcción).
- `load_rom()`: inicialización condicional que replica `geo_m68k_reset()` de Geolith:
  ```rust
  self.pvc_bank_addr = if self.prom.len() > FIXED_PROM_WINDOW_SIZE {
      FIXED_PROM_WINDOW_SIZE  // 0x100000
  } else {
      0
  };
  ```
- `FIXED_PROM_WINDOW_SIZE` = `0x100000` (1MB), misma constante usada en el resto del módulo.
- `Memory::new()` mantiene `pvc_bank_addr: 0` porque `prom` está vacío — `load_rom()` siempre sobrescribe este valor.

**Verificación:** KOF2003, SVC Chaos y Metal Slug 5 arrancan sin errores, sin pantalla verde. Capturas automáticas (frame 120) confirman renderizado visible. 191/191 tests pasan.

### Fix 12: Captura automática retrasada + `frame_count` increment (Mayo 2026)

**Archivo:** `frontend/src/main.rs`

**Problema (Parte A):** La captura automática diagnóstica (`--dump-rom-banks`) se realizaba en el frame 1, antes de que la BIOS/juego renderizaran contenido visible. Las capturas resultantes eran completamente negras (0% píxeles no-negros).

**Solución (Parte A):**
- La condición de captura cambió de `!status.auto_captured` (frame 1) a `!status.auto_captured && status.frame_count >= 120` (~2 segundos a 60fps).
- Esto da tiempo suficiente a la BIOS (UniBIOS) y al juego para inicializar y dibujar la primera pantalla visible.

**Problema (Parte B):** El campo `status.frame_count` existía en `RuntimeStatus` (usado para expiry de notificaciones RetroAchievements) pero **nunca se incrementaba**. Las notificaciones RA nunca expiraban porque `frame_count - start_frame` siempre era `0 - 0 = 0 < 180`.

**Solución (Parte B):**
- Añadido `status.frame_count += 1` al inicio del bloque de captura automática (se ejecuta cada frame, incondicionalmente).
- Esto corrige el expiry de notificaciones RA: ahora expiran correctamente tras 180 frames (~3 segundos).

**Verificación:** Capturas de KOF2003 (4.0% píxeles no-negros), SVC (10.0%), MS5 (9.8%) confirman renderizado visible. Sin errores de compilación ni warnings.

### Fix 13: YM2610 status read clears timer flags — FM music now plays (Mayo 2026)

**Archivos:** `core-emulator/src/ym2610.rs`, `core-emulator/src/z80.rs`

**Problema:** La música FM no sonaba en ningún juego (solo efectos ADPCM). El Z80 ejecutaba el driver de sonido del M-ROM, los timers YM2610 disparaban, las IRQs llegaban al Z80 y se atendían, pero el secuenciador de música nunca producía eventos `KEY_ON` — los canales FM permanecían en silencio.

**Causa raíz:** El método `read_status()` del YM2610 no limpiaba las flags `timer_a_flag` y `timer_b_flag` después de leerlas. En hardware real, leer el registro de status **limpia** ambas flags. Sin este comportamiento:

1. Timer A dispara → flag A = true
2. Timer B dispara → flag B = true  
3. Z80 ISR lee status → ambas flags siguen set (bug: debían limpiarse)
4. ISR verifica bit 1 (timer B) → siempre true → salta al handler de timer B (`$2159`)
5. ISR **nunca llega** al handler de timer A que llama al secuenciador de música (`$1318`) → nunca `KEY_ON`

**Solución:**
- Convertidos `timer_a_flag` y `timer_b_flag` de `bool` a `Cell<bool>` para permitir mutación interna desde `read_status(&self)`.
- `read_status()` ahora limpia ambas flags con `.set(false)` después de leerlas, replicando el comportamiento del hardware real.
- Actualizados todos los accesos a los campos (`.get()`/`.set()`) en: `new()`, `reset()`, `write_data()` (reg 0x27), `timer_irq_pending()`, `save_state()`, `load_state()`, `clock_timers()`, y tests.
- Eliminado código de diagnóstico temporal de `z80.rs` (M-ROM source check, is_irq/irq_data logging, step() PC tracking, run_tstates() messages) y `ym2610.rs` (diag counters, write_data logging, KEY_ON/KEY_OFF/KEY_OFF prints, FM register logging, per-sample/per-frame summaries, timer fire messages).

**Fix adicional en z80.rs:** Eliminado el `if self.cpu.is_halt() { return 0; }` en `step()`. El crate `z80emu` verifica `is_irq()` dentro de `execute_next()` para despertar la CPU de HALT. Sin llamar a `execute_next()`, las IRQs de timer nunca podían despertar al Z80 del estado HALT (aunque NMI sí funcionaba porque se maneja antes).

**Verificación:** 204/204 tests pasan. Diagnóstico confirma abundantes eventos `KEY_ON` y `fm_nonzero_samples > 0` en todos los frames — la música FM funciona.

### Fix 14: Audio timing Geolith-like — Z80 intercalado y cadencia YM por tstates (Junio 2026)

**Archivos:** `core-emulator/src/lib.rs`, `core-emulator/src/z80.rs`, `core-emulator/src/audio.rs`, `frontend/src/main.rs`

**Problema:** Al pulsar Start/Coin en algunos juegos, especialmente AOF, se escuchaba un ruido repetitivo parecido a un efecto de crédito/start en bucle. El core ejecutaba todo el Z80 al principio del frame, antes de que el 68K escribiera comandos de sonido, retrasando el ACK del M1 hasta el siguiente frame. Además el mixer generaba 925 muestras YM por frame, una cadencia de 60 Hz, no la cadencia MVS/Geolith.

**Solución:**
- El Z80 ahora se ejecuta durante el frame después de cada bloque de 68K, sincronizado con contadores de ciclo maestro estilo Geolith (`DIV_M68K=2`, `DIV_Z80=6`, `MCYC_PER_FRAME=405504`), con resto conservado entre frames.
- Los puertos Z80 no manejados devuelven `0x00`, como Geolith, en vez de `0xFF`; esto cubre el par auxiliar `0x0D/0x0E` usado por varios drivers M1.
- `AudioMixer::generate_for_tstates()` genera YM2610 a razón de 1 muestra cada 72 tstates Z80, manteniendo resto entre frames. Con MVS produce el patrón 938/939/939 pares estéreo.
- El frontend usa pacing MVS aproximado (`59.185606 Hz`) y extrae unas 745 muestras de salida por frame a 44.1 kHz.

**Verificación:** `cargo test -p core-emulator -- --test-threads=1`, `cargo test -p frontend -- --test-threads=1`, `cargo build --release --workspace` pasan. `probe_rom` con `roms/aof.neo` y pulsos `coin/start` termina con `SOUND command=0x00`, `nmi_pending=false`, `Z80 tstates=67584` y audio activo.

### Fix 15: YM2610 intercalado por scanline + BUSY no acumulativo (Junio 2026)

**Archivos:** `core-emulator/geolith_ymfm/ng_ymfm_bridge.c`, `core-emulator/src/audio.rs`, `core-emulator/src/lib.rs`, `core-emulator/src/ym2610.rs`

**Problema:** Varias ROMs `.neo` mostraban imagen correctamente pero quedaban totalmente mudas. En Metal Slug 3 y Metal Slug X, la traza Z80 terminaba con lecturas repetidas `YM_A0 = 0x80`: el driver M1 esperaba que terminara el estado BUSY del YM2610, pero el chip solo avanzaba al final del frame. Además, el bridge sumaba cada nuevo periodo BUSY al anterior, creando esperas enormes durante ráfagas de inicialización.

**Solución:**
- `ymfm_set_busy_end()` ahora reemplaza el periodo BUSY actual con el de la escritura más reciente, como deadline relativo, en vez de acumular todos los periodos.
- `AudioMixer` añade `begin_frame()` y `append_for_tstates()` para acumular muestras generadas en varios bloques sin cambiar el buffer final consumido por el frontend.
- `NeoGeo::step()` avanza YM2610 inmediatamente después de cada tramo Z80 por scanline. Así BUSY y los timers progresan mientras el Z80 ejecuta, no solo al final del frame.
- La cadencia total se conserva: los bloques intercalados siguen produciendo el patrón MVS 938/939/939 pares estéreo.
- Añadidos tests para BUSY no acumulativo y para equivalencia de cadencia al dividir un frame en varios bloques.

**Verificación:**
- Metal Slug 3 `.neo`: de `AUDIO_RUN frames_nonzero=0 peak=0` a `frames_nonzero=721 peak=19312`; la traza deja el bucle BUSY y muestra I/O YM normal.
- Metal Slug X `.neo`: de `AUDIO_RUN frames_nonzero=0 peak=0` a `frames_nonzero=803 peak=26992`.
- AOF mantiene audio e imagen correctos.
- Digger Man deja de leer BUSY `0x80`; el diagnóstico posterior confirma que su bucle `0x00BA` espera Timer B correctamente y que el juego produce audio tras inputs tardíos.
- `cargo test -p core-emulator -- --test-threads=1`: 256/256.
- `cargo test -p frontend -- --test-threads=1`: 30/30 lib + 54/54 bin.

### Fix 16: Cursor automático para ROMs + telemetría de timers YM2610 (Junio 2026)

**Archivos:** `frontend/src/main.rs`, `core-emulator/geolith_ymfm/ng_ymfm_bridge.c`, `core-emulator/src/ym2610.rs`, `core-emulator/src/z80.rs`, `core-emulator/src/bin/probe_rom.rs`

**Cambios:**
- `apply_loaded_rom()` sincroniza el cursor SDL2 para todas las rutas de carga: oculto al ejecutar una ROM real y visible al volver a la demo interna.
- `probe_rom` informa `IFF1/IFF2`, modo de interrupción Z80, status/línea IRQ YM2610, registro de modo `0x27` y contadores internos A/B/BUSY del bridge.
- Añadida una regresión que reproduce la secuencia Timer B de Digger Man (`0x26=0xE7`, `0x27=0x0A`) y verifica que el backend expone el bit B (`0x02`).

**Diagnóstico Digger Man:** El bucle Z80 en `0x00BA` espera y procesa Timer B correctamente; el status `0x01` observado entre ticks pertenece a Timer A. El prototipo permanece mudo en idle, pero con pulsos tardíos `coin/start/a` produce audio en 1337/2000 frames, pico `13001` y ADPCM-A activo.

**Verificación:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace -- --test-threads=1` (257 core + 30 frontend lib + 55 frontend bin) y `cargo build --release --workspace` pasan.

### Fix 17: Diagnóstico de sprites derivado del recorrido LSPC real (Junio 2026)

**Archivos:** `core-emulator/src/video.rs`, `core-emulator/src/lib.rs`, `core-emulator/src/bin/probe_rom.rs`, `core-emulator/src/bin/diagnose_rom.rs`

**Problema:** El renderer Geolith-like ya recorre los sprites SCB `1..381` por scanline y aplica el límite de 96 sprites por línea, pero el diagnóstico todavía interpretaba las palabras VRAM `$8600/$8680` como dos listas activas globales. En ROMs reales esto producía métricas engañosas como `active_unique=0` y `active_invalid=96`, aunque la escena se renderizara correctamente. Esa máscara falsa también podía filtrar incorrectamente las estadísticas de shrink.

**Solución:**
- Eliminadas las funciones sin consumidores que leían, renderizaban o sintetizaban listas globales `$8600/$8680`.
- `VramDebugStats` ahora expone `generated_visible_sprites`, scanlines con sprites, máximo por scanline y scanlines que exceden el límite LSPC de 96.
- Las estadísticas de shrink recorren SCB directamente y ya no dependen de contenido VRAM ajeno.
- `probe_rom`, `diagnose_rom` y el test diagnóstico del core muestran las métricas derivadas del recorrido real.
- Añadida regresión con 97 sprites coincidentes que verifica 16 scanlines de overflow.

**Verificación:** AOF reporta `visible_sprites=51`, `gen_max=46`; KOF2003 `69/47`; Metal Slug X `88/69`; Metal Slug 3 `80/68`; todos con `gen_overflow=0`. Las cuatro capturas de 1200 frames conservan exactamente el mismo SHA-256 antes y después del cambio. Pasan `cargo fmt --all -- --check`, Clippy estricto, 257 tests del core, 30 del frontend lib, 55 del frontend bin y build release del workspace.

### Fix 18: `.neo` P-ROM byte-swap por cabecera `NEO-GEO` (Junio 2026)

**Archivo:** `core-emulator/src/rom.rs`

**Problema:** Algunas ROMs `.neo` reales guardan la P-ROM con bytes intercambiados por palabra de 16 bits, como espera Geolith (`swapb16_range(romdata->p, romdata->psz)`), pero sus vectores iniciales no satisfacen la heurística de stack pointer usada por NGNEON. `fatfury3.neo` quedaba con cabecera cruda `EN-OEG`, NGH leído como `0x6900`, `cart_vectors=false` y sin audio/sprites.

**Solución:**
- `program_rom_looks_byte_swapped()` ahora prefiere la firma del header de cartucho en `P-ROM[0x100..]`: si raw no es `NEO-GEO` pero el swap por palabras produce `NEO-GEO`, normaliza toda la P-ROM.
- La validación diagnóstica de vectores usa la misma detección para evitar warnings falsos en `.neo` válidas con stack pointer no estándar.
- Se conserva el fallback por vector para ROMs sintéticas de tests.
- Añadida regresión con cabecera raw `EN-OEG` y vectores raros para asegurar que NGH `0x0069` queda en orden big-endian tras normalizar.

**Verificación:** `fatfury3.neo` pasa de `PHEAD ngh=0x6900`, `cart_vectors=false`, `audio_frames=0`, `sprites_h=0` a `PHEAD ngh=0x0069`, `cart_vectors=true`, `audio_frames=1942`, `sprites_h=78`, `nonblack=11109`. Pasan `cargo test -p core-emulator rom::tests:: -- --test-threads=1`.

### Fix 19: `REG_LSPCMODE` lee contador raster con offset `0xF8` (Junio 2026)

**Archivo:** `core-emulator/src/memory.rs`

**Problema:** Algunos juegos esperan VBlank sondeando `0x3C0006` hasta que el valor leído sea negativo (`move.w $3c0006,d0; bpl ...`). NGNEON exponía el scanline crudo en bits 7-15, sin el offset `0xF8` de Geolith, por lo que el bit 15 aparecía en el tramo equivocado del frame. `gowcaizr.neo` y `dragonsh.neo` entraban al cartucho pero se quedaban negros esperando esa condición.

**Solución:**
- `lspc_mode_with_scanline()` ahora devuelve `((scanline + 0xF8) & 0x1FF) << 7 | auto_animation_counter`, igual que `geo_lspc_mode_rd()`.
- El valor escrito en `lspc_mode` se mantiene separado para los bits de configuración usados por render/fix, evitando mezclar status leído con modo escrito.
- Añadida regresión que verifica `read16(0x3C0006) == 0x7C00` en scanline 0 y que en scanline 8 ya se lee con bit 15 activo.

**Verificación:** `gowcaizr.neo` a 2400 frames pasa de pantalla negra (`audio_frames=462`, `sprites_h=0`, `nonblack=0`) a escena visible (`audio_frames=1669`, `visible_sprites=15`, `nonblack=64043`, captura con personaje renderizado). `dragonsh.neo` llega al menú dev `TOOL/GAME` (`nonblack=1870`, `fix_drawable=176`). Controles `fatfury3`, `aof`, `mslug3` y `kof2003` siguen activos. Pasan `cargo test -p core-emulator memory::tests:: -- --test-threads=1`.

### Fix 20: BIOS MVS estándar — VRAMRW sin autoincremento, RTC TP por ciclos y palette byte lane Geolith (Junio 2026)

**Archivos:** `core-emulator/src/memory.rs`, `core-emulator/src/lib.rs`

**Problema:** Varias ROMs `.neo` arrancaban con UniBIOS, pero con BIOS MVS estándar (`sp-s2.sp1`) algunas quedaban en test de BIOS o no llegaban a activar vectores/audio/FIX de cartucho. `doubledr.neo` mostraba primero `VIDEO RAM ERROR ADDRESS 00004000 WRITE 5555 READ 0000`; después de corregir VRAM quedaba en un bucle de BIOS leyendo `0x320001` y esperando el flanco del pin TP del RTC4990A.

**Causa raíz:**
- `REG_VRAMRW` (`0x3C0002`) autoincrementaba `REG_VRAMADDR` en lecturas. Geolith no incrementa al leer; el test de VRAM de la BIOS espera releer la misma dirección seleccionada.
- El RTC exponía un bit de pulso derivado de lecturas, pero no avanzaba el pin TP por ciclos 68k. La BIOS MVS configura el RTC y espera que TP cambie con el tiempo real del sistema.
- Las escrituras byte a byte de palette RAM usaban offset lineal; Geolith primero convierte la dirección de bus a espacio de palabras y luego aplica el byte lane.

**Solución:**
- `read_lspc_register(LSPC_VRAMRW)` ahora devuelve `read_vram_word(vram_addr)` sin modificar `vram_addr`; las escrituras mantienen el autoincremento por `vram_mod`.
- `Rtc4990a` ahora mantiene contador de ciclos, `timer_1hz`, `tp_interval`, `tp_counter` y `tp_running`. Los comandos `0x04..=0x0B` configuran las frecuencias TP como Geolith (64/256/2048/4096 Hz o 1/10/30/60 s).
- `Memory::advance_rtc(cycles)` se llama desde las rutas de ejecución 68k en `NeoGeo::run_cpu_timing_slice()` y `NeoGeo::step()`, junto a watchdog e IRQ2.
- `read_status_a()` conserva el mapeo de hardware: DATA OUT en bit 7 y TP en bit 6.
- `write8()` de palette RAM usa helper `palette_ram_byte_write_offset()`, alineado con el handler de Geolith para writes de 8 bits.

**Tests añadidos/actualizados:**
- `lspc_vram_registers_read_write_and_apply_modulo`: comprueba que leer VRAMRW no mueve `vram_addr`, mientras escribir sí aplica `vram_mod`.
- `rtc_tp_pin_advances_by_cycles_for_mvs_bios_waits`: configura TP a 64 Hz y verifica que bit 6 de `STATUS_A` cambia tras avanzar ciclos 68k.
- `palette_byte_writes_use_geolith_word_address_lane`: cubre writes a `0x400000..0x400003` con el byte lane Geolith.

**Verificación real:** con `NGNEON_BIOS_HINT=sp-s2.sp1`, `aof.neo`, `joyjoy.neo`, `fatfury3.neo` y `doubledr.neo` llegan a `cart_vectors=true`, `cart_audio=true`, `cart_fix=true`, audio no-cero y framebuffer visible. `doubledr.neo` a 12000 frames renderiza escena jugable (`nonblack=34665`, `visible_sprites=97`). Control PVC/UniBIOS: `kof2003.neo` sigue arrancando (`pvc_bank=0x200000`, `nonblack=52733`). Pasan `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test -p core-emulator -- --test-threads=1` (261/261).

### Fix 21: RetroAchievements login con password como fallback (Junio 2026)

**Archivos:** `core-emulator/src/retroachievements.rs`, `core-emulator/src/lib.rs`, `frontend/src/main.rs`, `README.md`

**Problema:** La UI y la documentación aceptaban `ra_token`, pero no había una ruta funcional para configurar usuario + password aunque `rcheevos` ya expusiera `rc_client_begin_login_with_password()` en el FFI.

**Solución:**
- Añadido `RASession::login_with_password()` y wrapper público `NeoGeo::ra_login_with_password()`.
- `RuntimeStatus` carga `ra_password` desde `config/ngneon.conf`.
- Auto-login: si existe `ra_token`, se usa token; si no, se usa `ra_username` + `ra_password`.
- El menú `RA Login` considera configurada la sesión si hay token o usuario+password.
- Logout ya no borra `ra_username` de la configuración persistente.
- README y AGENTS documentan `ra_password` como fallback, manteniendo `ra_token` como opción preferida.

**Verificación:** `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`.

### Fix 22: `.neo` S-ROM vacía usa extracción FIX CMC de 512KB (Junio 2026)

**Archivo:** `core-emulator/src/rom.rs`

**Contexto Geolith:** `geo_neo_load()` asigna las secciones `.neo` en orden (`P`, `S`, `M`, `V1`, `V2`, `C`) y luego ejecuta `geo_lspc_postload()`, `geo_lspc_set_fix(LSPC_FIX_CART)`, `geo_lspc_set_fix_banksw(FIX_BANKSW_NONE)` y `geo_m68k_postload()` antes de aplicar heurísticas por NGH. Para variantes como Matrimelee, Geolith usa el tamaño real `romdata->ssz` y datos del tail de C-ROM para corregir la capa FIX.

**Problema:** NGNEON tenía dos políticas incompatibles para `.neo` con S-ROM vacía:
- Un fallback temprano rellenaba `parsed.srom` con 128KB crudos del final de C-ROM.
- El postproceso posterior esperaba detectar `rom.srom.is_empty()` para extraer datos FIX CMC con `extract_cmc_s_data(..., 0x80000)`.

Al ejecutar el fallback temprano, la ruta CMC de 512KB quedaba bloqueada. Esto no afectaba las `.neo` actuales del directorio `roms/` porque todas tienen `SSize > 0`, pero sí rompía conversiones o sets futuros con `SSize == 0`.

**Solución:**
- Eliminado el fallback temprano de 128KB en `RomData::from_neo()`.
- La extracción de S-ROM ausente queda centralizada en `post_process_neo_rom()`, usando la ruta CMC de 512KB desde C-ROM.
- Añadida regresión `neo_empty_srom_extracts_cmc_fix_data_from_crom_tail`, que construye un `.neo` sintético con S-ROM vacía y verifica tamaño/contenido contra `extract_cmc_s_data(&crom, 0x80000)`.

**Verificación real:** `roms/*.neo` tiene `SSize > 0` en todos los casos locales, así que el cambio no altera las ROMs existentes. Controles con `probe_rom` release: `aof.neo`, `fatfury3.neo`, `kof2003.neo` y `gowcaizr.neo` mantienen `cart_vectors=true`, `cart_audio=true`, `cart_fix=true`, audio no-cero y framebuffer visible. Pasan `cargo test -p core-emulator rom::tests:: -- --test-threads=1` (47/47) y `cargo test -p core-emulator -- --test-threads=1` (262/262).

### Fix 23: Sincronización Z80/YM2610 por ciclo maestro Geolith (Junio 2026)

**Archivo:** `core-emulator/src/lib.rs`

**Contexto Geolith:** `geo_exec()` mantiene contadores persistentes `mcycs` y `zcycs`. Cada instrucción 68K suma `icycs * DIV_M68K` al reloj maestro y luego el Z80 ejecuta instrucciones hasta que `zcycs >= mcycs`, sumando `scycs * DIV_Z80`. El YM2610 avanza cada 72 tstates Z80. Al final del frame, `mcycs` y `zcycs` se reducen módulo `MCYC_PER_FRAME`.

**Problema:** NGNEON ya intercalaba Z80/YM por frame, pero seguía usando un objetivo fraccional por scanline (`z80_tstates_per_frame * scanline / 264`). Esto aproximaba la cadencia total, pero no replicaba el catch-up real de Geolith: los pequeños overshoots de instrucción y el desfase 68K/Z80 no se conservaban con la misma semántica.

**Solución:**
- Añadidos `master_cycles` y `z80_master_cycles` a `NeoGeo`.
- `run_cpu_timing_slice()` ahora suma ciclos maestros tras cada tramo 68K y llama a `catch_up_z80_audio_to_master_clock()`.
- El Z80 avanza instrucción a instrucción con `Z80::step()` mientras `z80_master_cycles < master_cycles`; cada instrucción genera YM con `AudioMixer::append_for_tstates()`.
- Al terminar el frame, ambos contadores aplican `% 405504`, igual que Geolith.
- `load_rom_and_connect()`, `reset()` y watchdog reset limpian contadores y mixer para evitar restos de ROM/ejecución anterior.
- Añadida regresión `geolith_master_clock_catches_z80_up_to_68k`, que verifica el objetivo MVS de 67,584 tstates con overshoot máximo de una instrucción y 938/939 muestras YM.

**Verificación:** `cargo test -p core-emulator` pasa (263/263), `cargo clippy --workspace --all-targets -- -D warnings` pasa, `cargo build --release --workspace` pasa. `probe_rom` release con `roms/aof.neo` carga con BIOS real y reporta `Z80 tstates=67586`, consistente con Geolith (67,584 + overshoot final de instrucción).

### Fix 24: Icono profesional + gamepad con estados explícitos (Junio 2026)

**Archivos:** `frontend/src/gamepad.rs`, `frontend/src/main.rs`, `frontend/build.rs`, `assets/ngneon_icon.*`, `frontend/assets/ngneon_icon.*`

**Problema:** El stick analógico podía quedarse con direcciones opuestas presionadas si se movía directamente de izquierda a derecha o de arriba a abajo sin volver al centro. Además el proyecto no tenía icono propio integrado para la ventana SDL ni para el ejecutable Windows.

**Solución:**
- Añadido `GamepadAction { action, pressed }` y `GamepadManager::process_event_actions()`.
- Las transiciones del stick ahora emiten release previo + press nuevo en reversas directas.
- El overlay de remapeo ignora ejes mientras escucha para evitar bindings accidentales por drift, y selecciona automáticamente el mando que envía el botón.
- Añadidos tests para reversa directa y release al centrar stick.
- Creado icono NGNEON profesional (`PNG` transparente + `.ico` multi-resolución).
- `set_window_icon()` aplica el PNG embebido como icono de ventana SDL.
- `frontend/build.rs` compila `ngneon_icon.rc` con `rc.exe` y enlaza el `.res` para incrustar el icono en `ngneon-emu.exe` en Windows.

**Verificación:** `cargo clippy --workspace --all-targets -- -D warnings` pasa. `cargo test --workspace -- --test-threads=1` pasa (263 core, 32 frontend lib, 57 frontend bin). `cargo build --release --workspace` pasa y genera `target/release/ngneon-emu.exe` con recurso `ngneon_icon.res`.

### Fix 25: Acciones globales de gamepad + navegación por menús (Junio 2026)

**Archivos:** `frontend/src/gamepad.rs`, `frontend/src/main.rs`, `frontend/src/ui.rs`, `frontend/src/lang.rs`, `README.md`, `AGENTS.md`

**Problema:** El gamepad solo mapeaba acciones NeoGeo del juego. No había acciones configurables para salir del emulador o abrir el ROM Browser, y los overlays de configuración seguían dependiendo principalmente del teclado.

**Solución:**
- Añadidas acciones globales por controller: `SystemExit` y `SystemRomBrowser`.
- Soporte de chord simple o doble botón (`Button` / `Button+Button`) en `config/gamepad/<guid>.conf`.
- Defaults: `SystemExit=Back+Start` sale sin preguntar; `SystemRomBrowser=Guide` abre el ROM Browser.
- El overlay de Gamepad Config muestra 12 filas: 10 acciones NeoGeo + `Exit` + `ROM Browser`.
- Gamepad usable en overlays: D-Pad/stick navega, `A`/`Start` acepta, `B`/`Coin` vuelve/cierra, `C` restaura defaults donde aplica.
- El ROM Browser bloquea input de juego mientras está abierto y se puede navegar/cargar con gamepad.

**Verificación:** `cargo clippy --workspace --all-targets -- -D warnings` pasa. `cargo test --workspace -- --test-threads=1` pasa (263 core, 34 frontend lib, 59 frontend bin). `cargo build --release --workspace` pasa.

### Fix 26: ROM Browser nítido + metadata Geolith para ZIPs conocidos (Junio 2026)

**Archivos:** `frontend/src/screenshot.rs`, `frontend/src/main.rs`, `frontend/src/ui.rs`, `README.md`, `AGENTS.md`, `core-emulator/src/rom.rs`

**Problema A:** Las carátulas/cartuchos del ROM Browser podían verse suaves o borrosos al reducir PNGs de alta resolución y al pasar el overlay por CRT/scanlines/bloom.

**Solución A:**
- `load_png_thumbnail()` usa filtro Catmull-Rom, más nítido para arte con texto/logos pequeños.
- El ROM Browser usa una cuadrícula 2×2 con recuadros 150×84 para que cartuchos completos no queden reducidos a una pegatina ilegible.
- Cuando `show_rom_browser` está activo, el render final desactiva temporalmente scanlines, curvature y bloom para mantener la UI limpia.

**Problema B:** Los ZIPs conocidos (`detect_known_zip_set`) aplicaban decryption CMC/SMA/PCM2, pero terminaban con `metadata: None`. Eso hacía que el runtime no recibiera board/fix metadata equivalente a `.neo` para sets inequívocos como KOF2003, SVC, MSlug3, Garou y KOF2000 parent.

**Solución B:**
- Añadido `metadata_from_known_zip_set()`.
- ZIP KOF2003 asigna NGH 0x271 → `Pvc` + `Tile`.
- ZIP SVC asigna NGH 0x269 → `Pvc` + `Tile` cuando las firmas P-ROM lo indican.
- ZIP MSlug3/Garou/KOF2000 parent heredan board/fix metadata estilo Geolith.
- KOG/bootlegs ambiguos sin archivos 257 se dejan sin metadata automática.

**Verificación:** `cargo clippy --workspace --all-targets -- -D warnings` pasa. `cargo test --workspace -- --test-threads=1` pasa (264 core, 34 frontend lib, 59 frontend bin).

### Fix 27: Diagnóstico ZIP y validación de arranque real (Junio 2026)

**Archivos:** `core-emulator/src/bin/diagnose_rom.rs`, `README.md`, `AGENTS.md`

**Problema:** `probe_rom` ya aceptaba `.neo` y `.zip`, pero `diagnose_rom` seguía forzando `RomData::from_neo()`. Eso impedía usar el diagnóstico detallado de bancos/VRAM con ZIPs reales porque fallaba en la firma ZIP (`PK`) antes de entrar al loader correcto.

**Solución:**
- Añadido selector explícito de extensión en `diagnose_rom`: `.neo` → `RomData::from_neo()`, `.zip` → `RomData::from_zip()`.
- Añadido `RomFileKind` con tests de extensión case-insensitive y rechazo de formatos no soportados.
- Validación local de ZIPs reales:
  - `kof2000.zip`: entra a escena visible con sprites/audio, `board=Sma`, `fix=Tile`, `cart_vectors/audio/fix=true`.
  - `kof2002.zip`: entra al título tras ejecución prolongada, CMC50/PCM2 activo, sprites y audio funcionales.
  - `mslug3.zip`: entra a pantalla de título jugable con sprites/audio, ruta SMA/CMC42 activa.
  - `kog.zip`: se identifica como caso especial/incompleto local (`M=0`, `V=0`, cabecera P-ROM inválida, crosshatch); no debe usarse como regresión del loader ZIP general.
- Control `.neo`: `diagnose_rom` con `aof.neo` sigue cargando y renderizando, manteniendo compatibilidad del camino clásico.

**Verificación:** `cargo test -p core-emulator --bin diagnose_rom` pasa (2/2). `cargo test -p core-emulator rom::tests:: -- --test-threads=1` pasa (48/48). `diagnose_rom` release valida `kof2000.zip`, `kof2002.zip`, `mslug3.zip`, `kog.zip` y `aof.neo`. `probe_rom` release genera capturas jugables para `kof2000.zip`, `kof2002.zip` y `mslug3.zip`.

---

## Siguientes pasos sugeridos

- **Grabación de video/audio**: capturar frames + audio ring buffer → archivo `.mp4`/`.webm` desde la UI.
- **Shaders GLSL adicionales**: NTSC composite artifacts, phosphor decay configurable, máscara de subpíxeles RGB.
- **Integrar CI/CD**: GitHub Actions para builds Windows/Linux con artifacts automáticos.
- **Audio visualizer en tiempo real**: espectro de frecuencias sobre el overlay de debug.
- **Netplay básico**: rollback + sincronización de inputs para 2 jugadores en red.
- **ROM Browser con filtros**: búsqueda por nombre, género, año, NGH.
- **Perfiles de gamepad por juego**: diferentes mappings de gamepad para diferentes ROMs.
- **Remapping de sticks analógicos**: permitir reasignar ejes a acciones no-direccionales.

---

## Diagnóstico y compatibilidad de carga de ROM `.neo`

### Checklist de diagnóstico para agentes

- Verificar que el vector de reset en la P-ROM es válido (no 0x00000000 ni 0xFFFFFFFF, apunta a código ejecutable).
- Confirmar que la P-ROM tiene el tamaño esperado y no está truncada.
- Validar que los campos NGH, board type y fix bankswitching se asignan correctamente (ver funciones `detect_neo_board_type`, `detect_neo_fix_banksw`).
- Revisar que la extracción de S-ROM desde C-ROM tail ocurre si falta la sección S.
- Comparar los dumps de bancos ROM (`screenshots/{label}_prom_dump.bin`, etc.) con los generados por Geolith para la misma ROM.
- Inspeccionar logs de arranque: buscar advertencias sobre tamaño de P-ROM, detección de bootleg, o downgrade de board type.
- Si hay excepción 68k (pantalla "EXCEPTION ERROR HANDLING"), revisar:
  - Vector de reset y stack pointer en el dump de P-ROM.
  - Si la ROM es bootleg, que el board type se haya ajustado correctamente.
  - Si la ROM requiere bankswitching especial (SMA, PVC), que el loader lo detecte y configure.

### Comparación con Geolith

- Usar los mismos archivos `.neo` y comparar los dumps binarios de P-ROM, C-ROM, S-ROM, M-ROM, V-ROM.
- Verificar que los vectores de reset y stack pointer coinciden en ambos emuladores.
- Si hay diferencias, revisar el parser de `.neo` y la lógica de post-procesado (`post_process_neo_rom`).
- Para bootlegs y variantes, comparar los bytes de P-ROM en los offsets críticos (ej. 0x689, 0xc1 para KOF2003).

### Skills recomendadas para agentes

- Skill de comparación binaria de dumps ROM entre NGNEON y Geolith.
- Skill de validación automática de vectores de reset y stack pointer.
- Skill de generación de logs detallados de arranque y warnings.

---

> **Nota para agentes:**
> Si tras aplicar el checklist la ROM sigue sin arrancar, sugerir al usuario comparar el dump de P-ROM y los vectores de reset con Geolith, y revisar los logs de advertencia generados por el loader.

### Auditoría integral `.neo` y PCM de referencia (Junio 2026)

- `audit_neo_library` validó las 155 ROM `.neo` durante 1800 frames:
  - 155/155 cargan y ejecutan sin fallo CPU.
  - Todas generan exactamente 1.689.600 pares PCM en 1800 frames.
  - Cadencia nativa exacta: patrón 938/939 (ocasional 940 por límites de frame), error acumulado 0.
  - `ganryu` y `tpgolf` permanecen en silencio durante los primeros 1800 frames, pero producen audio real al extender la prueba a 7200 frames.
- Nuevos capturadores deterministas:
  - `capture_geolith_pcm`: host libretro mínimo que carga `geolith_libretro.dll` y vuelca directamente PCM estéreo nativo + métricas de vídeo CSV, sin AAC, SDL ni resampler.
  - `capture_ngneon_pcm`: vuelca el PCM nativo y las mismas métricas desde NGNEON.
  - Ambos aceptan el argumento final `stimulus` para aplicar pulsos fijos de Coin/Start/A y comparar respuestas audibles.
- Referencia temporal confirmada contra Geolith:
  - 55.555 Hz nativos.
  - 938/939 pares por frame.
  - 563.200 pares en 600 frames y 1.689.600 en 1800 frames.
  - AOF, Garou, KOF99 y MSlug4 mantienen fase estable; los offsets observados son de 0 a 3,02 ms y no acumulan deriva.
  - En `s1945p`, el primer audio con estímulo difiere solo 6 pares PCM (0,108 ms) entre Geolith y NGNEON.

### Fix 28: Puertos de sistema, RTC y NMI Z80 equivalentes a Geolith (Junio 2026)

**Archivos:** `core-emulator/src/memory.rs`, `core-emulator/src/lib.rs`, `core-emulator/src/z80.rs`, `core-emulator/src/bin/audit_neo_library.rs`

**Discrepancias encontradas durante la matriz PCM:**
- La habilitación de NMI del Z80 aceptaba erróneamente los puertos `0x09-0x0B`; Geolith solo la activa escribiendo en `0x08`.
- El pin TP del RTC arrancaba detenido, mientras Geolith lo inicializa en modo RUN.
- `STATUS_A` inactivo devolvía `0xFF` en lugar de `0x07`.
- `STATUS_B` no distinguía correctamente sistema MVS/AES y presencia de memory card.

**Corrección:**
- El Z80 habilita NMI exclusivamente mediante el puerto `0x08`.
- `Rtc4990a` inicia `tp_running=true`.
- Los puertos inactivos parten de `P1=0xFF`, `STATUS_A=0x07` y `STATUS_B=0x3F`.
- `STATUS_B` aplica el bit de sistema igual que Geolith: MVS sin tarjeta `0xBF`, MVS con tarjeta `0x8F`, AES sin tarjeta `0x3F`.
- Añadido `system_is_mvs` y su setter para mantener explícita la configuración del hardware.
- Añadidas regresiones para NMI, TP del RTC y los tres estados de `STATUS_B`.

**Resultado de auditoría:**
- La auditoría posterior de las 155 ROM `.neo` no presenta fallos de carga, CPU ni cadencia.
- `diggerma.neo`, único caso inicialmente marcado para revisión por audio nulo, también produce silencio en la captura directa de Geolith durante los mismos 1800 frames; se clasifica como attract esperado y no como fallo de audio.
- Resultado consolidado: 155/155 `.neo` compatibles en la prueba de 1800 frames, con 1.689.600 pares PCM por ROM y error acumulado de cadencia igual a cero.
- No se inició ninguna prueba ni modificación adicional del camino `.zip`.

### Fix 29: Resampling MVS exacto y continuidad Lanczos entre frames (Junio 2026)

**Archivos:** `frontend/src/audio.rs`, `frontend/src/main.rs`, `frontend/src/bin/audit_frontend_audio.rs`

**Problema A — deriva subaudible acumulativa:**
- El núcleo genera una muestra YM2610 cada 72 tstates del Z80 a 4 MHz: `4.000.000 / 72 = 55.555,555... Hz`.
- El frontend construía el resampler con la constante redondeada `55.555 Hz`.
- La diferencia de 10 ppm acumulaba aproximadamente 2.000 pares fuente por hora y podía acabar sobrescribiendo el FIFO interno en sesiones prolongadas.

**Problema B — borde entre bloques:**
- El filtro Lanczos-3 recibía un bloque nativo por frame y consumía inmediatamente la salida correspondiente.
- En determinados finales de frame podía faltar una muestra futura para interpolar; la ruta anterior emitía pares aislados de silencio.

**Solución:**
- Añadido `Resampler::new_mvs()`, que usa la razón exacta `(4.000.000 / 72) / output_rate`.
- El stream MVS precarga cuatro pares de historia repitiendo el primer par: 72 µs de margen fijo, sin deriva ni silencios periódicos.
- `main.rs` usa ahora el constructor MVS exacto.
- Añadidas métricas `starved_output_pairs`, `overrun_samples` y `underrun_samples`, con reset al cambiar ROM.
- Añadida la herramienta `audit_frontend_audio`, limitada deliberadamente a `.neo`, que ejecuta núcleo → resampler → ring buffer → callbacks de 1024 pares y comprueba matemáticamente la salida esperada.

**Verificación:**
- Regresión sintética de 10.000 frames: deriva fuente 0, starvation 0.
- AOF, Garou, KOF99, Metal Slug 4, Strikers 1945 Plus, KOF2003, Jockey Grand Prix y Digger Man pasan 3.600 frames cada uno con conteo exacto, starvation 0, overruns 0 y underruns 0.
- Auditoría completa del frontend: 155/155 `.neo` pasan 600 frames en cuatro shards (39 + 39 + 39 + 38), sin pérdidas ni desfase acumulado.
- La salida por frame se mantiene ligada al reloj maestro MVS: `44.100 * 405.504 / 24.000.000`.
- No se ejecutaron ni modificaron ROMs `.zip`.

### Fix 30: RetroAchievements — mapa RAM Arcade equivalente a RetroArch (Junio 2026)

**Archivos:** `core-emulator/src/retroachievements.rs`, `core-emulator/src/rcheevos_ffi.rs`, `core-emulator/src/lib.rs`, `frontend/src/main.rs`

**Problema raíz:**
- El callback de memoria interpretaba las direcciones de rcheevos como direcciones físicas del bus 68k.
- Para consola `RC_CONSOLE_ARCADE`, rcheevos no define regiones específicas. La integración oficial `rc_libretro` concatena `RETRO_MEMORY_SYSTEM_RAM` desde la dirección RA `0x000000`.
- Geolith expone como `RETRO_MEMORY_SYSTEM_RAM` sus 64 KiB de RAM principal 68k.
- Por tanto, las direcciones de logros `RA $0000-$FFFF` deben leer Neo Geo físico `$100000-$10FFFF`.
- NGNEON leía desde `$000000`, es decir, P-ROM. Los logros se cargaban pero evaluaban datos de cartucho constantes y nunca se disparaban.

**Corrección:**
- `ra_memory_read_callback()` usa un espacio RA contiguo de 64 KiB respaldado por `Memory::ram`. **El orden byte directo descrito originalmente aquí fue corregido posteriormente por Fix 33 para igualar FBNeo (`address ^ 1`).**
- Las lecturas fuera de `$0000-$FFFF` devuelven cero bytes, igual que una región libretro no disponible.
- Añadida regresión que demuestra que RA `$0000` devuelve RAM y no P-ROM, incluyendo lectura parcial en `$FFFF`.
- El reloj de rcheevos usa ahora `Instant` monotónico real en milisegundos, en lugar de aproximar cada frame como 16 ms.
- `rc_client_idle()` se ejecuta mientras la emulación está pausada.
- Se procesa `RC_CLIENT_EVENT_RESET`: al cambiar Hardcore se reinician máquina, runtime RA y pipeline de audio, como exige `rc_client.h`.
- Se procesan eventos de desconexión/reconexión y se muestra el error real del servidor.
- Activado logging `RC_CLIENT_LOG_LEVEL_INFO`.
- El token devuelto tras login por contraseña se guarda automáticamente, evitando repetir el fallback en siguientes arranques.
- La identificación del juego se aplaza hasta que el login termina; se elimina el falso `LOGIN_REQUIRED` inicial.

**Validación real:**
- Arranque release con `aof.neo`: login por token correcto.
- rcheevos identifica hash Arcade `df4a8b32238c36921a260ed6ab784850`.
- Juego RA reconocido: Art of Fighting, ID 11758.
- Set 4896 cargado con 27/28 logros activos y 2 leaderboards.
- Log de runtime confirma: `RA $000000-$00FFFF -> Neo Geo RAM $100000-$10FFFF`.
- El flujo final no presenta `LOGIN_REQUIRED`, fallo de token ni error de carga.

### Fix 31: RetroAchievements — transporte HTTP asíncrono sin congelar frames (Junio 2026)

**Archivo:** `core-emulator/src/retroachievements.rs`

**Problema raíz:**
- `rc_client_do_frame()` puede solicitar al frontend que conceda un logro antes de emitir `RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED`.
- El callback HTTP de NGNEON ejecutaba `reqwest::blocking` en el hilo de emulación.
- Cuando un logro cumplía sus condiciones, el juego quedaba congelado durante la petición de red; un timeout o error podía retrasar o impedir que el frontend procesara la notificación.
- RetroArch no bloquea el frame: copia la petición, la ejecuta en su cola de tareas HTTP y entrega el callback de rcheevos posteriormente.

**Corrección:**
- `ra_server_call_callback()` copia URL, POST, content type, user-agent y callback, y lanza la petición en un worker.
- El worker envía una respuesta propia mediante un canal global seguro.
- `RASession::do_frame()` y `RASession::idle()` llaman a `pump_server_responses()` para entregar las respuestas pendientes a rcheevos desde el hilo de emulación.
- La concesión ya no espera la red: rcheevos puede emitir inmediatamente el evento visual del logro y el juego continúa sin pausa.
- Añadido `target/retroachievements.log` persistente con tipo de operación, estado HTTP y duración. Solo registra el parámetro seguro `r` (`awardachievement`, `login`, etc.); nunca guarda usuario, token, contraseña ni el cuerpo POST.
- Añadida regresión que verifica que el callback no se ejecuta al encolar y sí al bombear la respuesta desde el hilo de emulación.

**Validación:**
- Los cuatro tests específicos de RetroAchievements pasan.
- `preisle2.neo` inicia sesión, se identifica como juego RA 12343 y carga 27/28 logros y 2 leaderboards con respuestas HTTP 200.
- Las peticiones iniciales tardan aproximadamente 176-226 ms sin bloquear el bucle de emulación.

### Fix 32: RetroAchievements — save states, Hardcore y pausa según rc_client (Junio 2026)

**Archivos:** `core-emulator/src/rcheevos_ffi.rs`, `core-emulator/src/retroachievements.rs`, `core-emulator/src/lib.rs`, `frontend/src/main.rs`

**Discrepancias detectadas contra la guía oficial de integración de rcheevos:**
- Los save states solo contenían la máquina Neo Geo; no incluían el progreso interno de logros, hit counters, leaderboards y Rich Presence.
- Un auto-load podía ejecutarse antes de terminar el login/carga asíncrona del juego RA.
- Hardcore todavía permitía guardar/cargar estados.
- La tecla de pausa no consultaba `rc_client_can_pause()`, necesaria para impedir pause spam en condiciones sensibles.

**Corrección:**
- Añadidos bindings FFI para `rc_client_can_pause()`, `rc_client_progress_size()`, `rc_client_serialize_progress_sized()` y `rc_client_deserialize_progress_sized()`.
- Los save states nuevos adjuntan un trailer `NGRASTAT` con el bloque de progreso producido por rcheevos.
- El parser mantiene compatibilidad con estados anteriores sin trailer. Al cargar uno antiguo, reinicia el runtime RA para no conservar condiciones de un estado futuro.
- Si el estado se carga antes de `GameLoaded`, el progreso RA queda pendiente y se restaura inmediatamente después de completar la carga asíncrona.
- Hardcore bloquea save, load, auto-save y auto-load; el frontend muestra un mensaje bilingüe.
- Al intentar pausar se llama a `rc_client_can_pause()`. Si todavía no está permitido, se muestra el tiempo restante aproximado.
- Las respuestas HTTP asíncronas quedan vinculadas al `rc_client_t` que las originó; una respuesta tardía de una sesión destruida se descarta en vez de invocar un callback obsoleto.
- Los fallos de transporte devuelven `RC_API_SERVER_RESPONSE_RETRYABLE_CLIENT_ERROR` (`-2`) con su mensaje real. rcheevos conserva y reintenta concesiones/leaderboards pendientes; antes recibía `-1`, las clasificaba como definitivas y podía perder un logro durante un corte temporal.

**Compatibilidad:**
- El formato base del estado de emulación no cambia; el bloque RA es un trailer opcional.
- Los estados antiguos siguen cargando en modo Softcore.
- El progreso RA no se serializa si todavía no hay juego RA cargado.

### Fix 33: RetroAchievements — orden de bytes FBNeo para RAM Neo Geo (Junio 2026)

**Archivos:** `core-emulator/src/retroachievements.rs`, `core-emulator/src/bin/audit_ra_preisle2.rs`

**Problema raíz:**
- La dirección lógica RA ya se traducía a los 64 KiB de work RAM, pero los bytes se entregaban en el orden físico big-endian usado internamente por NGNEON/Geolith.
- Los sets Arcade de RetroAchievements para Neo Geo están creados contra la RAM expuesta por FBNeo.
- FBNeo almacena las palabras 68000 en orden host-endian y sus accesos byte aplican `address ^= 1` (`src/cpu/m68000_intf.cpp`).
- Por tanto, una dirección RA byte debe leer `Memory::ram[address ^ 1]` en NGNEON.
- En Prehistoric Isle 2, el set espera el estado de juego `0xD3` en RA `$7503`; la captura de RAM Geolith/NGNEON mostró ese byte en el offset físico vecino `$7502`. La ruta directa nunca podía cumplir la condición.

**Corrección:**
- `read_ra_memory()` transforma individualmente cada dirección con XOR 1.
- Se conserva la ventana lógica RA `$0000-$FFFF`; solo cambia el orden de bytes dentro de cada palabra.
- La regresión de memoria verifica explícitamente el formato FBNeo y el límite `$FFFF`.
- La auditoría online en modo Spectator carga el set oficial 5120 y reproduce la transición física de Stage 1 (`$8435: 0→4`, `$7502: D3`), comprobando que rcheevos dispara el logro 431379 sin enviarlo al servidor.
- El cliente HTTP se reutiliza globalmente para conservar conexiones y evitar crear un pool/TLS nuevo en cada login, ping o concesión.

**Validación real de la cadena RA:**
- Juego identificado: Prehistoric Isle 2, ID 12343, hash `b693a2560b6f671f495b11c357853a16`.
- Set oficial cargado con 28/28 logros en Spectator.
- Evento confirmado: `Spectated achievement 431379: Pachycephalosaurus`.
- No se concedieron puntos durante la auditoría.
