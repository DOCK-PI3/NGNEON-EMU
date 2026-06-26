# Implementation Plan: **NGNEON-EMU**

`NGNEON-EMU` is a state-of-the-art, futuristic, and highly polished multiplatform NeoGeo emulator. It combines a high-performance **Rust emulation core** (with a native UI, not web-based) and a premium, cyberpunk-themed graphical frontend using egui or SDL2 for true multiplatform support.


This architecture provides native-speed emulation, total cross-platform compatibility (Windows, macOS, Linux), and allows us to implement rich visual effects, CRT shaders, and professional diagnostic interfaces using native graphics libraries, avoiding web technologies.

---

## User Review Required


> [!IMPORTANT]
> **Native Multiplatform Architecture:** The core is built in Rust, with a native graphical frontend (egui/SDL2). This enables a double-clickable, highly performant multiplatform experience without web dependencies.
>
> **Instant Playability (Built-in Homebrew Demo):** Since original NeoGeo BIOS (`neogeo.zip`) and commercial ROMs are copyrighted and difficult to acquire, we will embed a custom **Cyberpunk Homebrew Demo Game** directly inside the Rust emulator. This allows you (and any user) to immediately boot, play, test the CRT scanline shaders, and see the diagnostic registry panels in action without needing to supply external files.
>
> **Supported ROMs:** Users can select `.neo` (TerraOnion/MiSTer), `.zip`, or `.7z` NeoGeo files, which our Rust core will parse, unpack, and load into memory.

---

## Proposed Project Structure


We will organize the repository to separate the Rust Emulation Core and the Modern Cyberpunk Native Frontend:

```
NGNEON-EMU/
├── Cargo.toml               # Cargo workspace / project config
├── core-emulator/           # Rust NeoGeo Core
│   ├── Cargo.toml           # Dependencies: r68k (M68k CPU), zip, etc.
│   └── src/
│       ├── lib.rs           # Main emulator interface
│       ├── cpu.rs           # 68000 CPU emulator mapping (via `r68k` / interpreter)
│       ├── memory.rs        # NeoGeo Bus memory mapping (RAM, VRAM, ROM banks)
│       ├── rom.rs           # ROM Parser (.neo headers, zip file reader)
│       ├── video.rs         # Sprite and Fix-layer decoding and buffer output
│       ├── audio.rs         # Sound mixer and YM2610 FM/ADPCM simulation
│       └── demo.rs          # Embedded cyberpunk homebrew demo ROM
└── frontend/                # Futuristic Cyberpunk Native Interface (egui/SDL2)
    ├── Cargo.toml           # Native frontend project config
    └── src/
        ├── main.rs          # Entry point for native UI
        ├── ui.rs            # UI panels, overlays, CRT effects
        ├── input.rs         # Gamepad/keyboard input
        └── shaders/         # GLSL shaders for CRT, scanlines, bloom, etc.
```

---

## Proposed Changes & Components

### 1. **Rust Emulation Core (`core-emulator`)**

The emulation core will be written in highly optimized, safe Rust, designed to handle high-fidelity simulation of the NeoGeo hardware.

*   **`rom.rs` (ROM & Header Parser):**
    *   **`.neo` Parser:** Reads the 4096-byte `NeoFile` header, parses fields such as `PSize`, `SSize`, `MSize`, `V1Size`, `V2Size`, and `CSize`, along with metadata (`Name`, `Year`, `Genre`, `NGH`). It then partitions the subsequent raw binary data into respective system buses.
    *   **`.zip` / `.7z` Parser:** Scans archive contents for standard NeoGeo chips (`p1.bin`, `m1.bin`, `v1.bin`, `s1.bin`, `c1.bin`) using Rust crates like `zip` and compiles them into active memory blocks.
    *   **Diagnostic Dumps:** Automatically dumps all five ROM banks (P/C/S/M/V) to `screenshots/{label}_{bank}_dump.bin` on load, with chunked progress for large banks and an opt-out toggle (`--no-dump-rom-banks` or `diagnostic_dumps=off`).
*   **`cpu.rs` & `memory.rs` (Execution and Bus):**
    *   Integrates a clean memory bus mapping original NeoGeo addresses:
        *   `0x000000 - 0x0FFF9F`: BIOS / Program ROM (P-ROM)
        *   `0x100000 - 0x10FFFF`: User RAM (Work RAM)
        *   `0x300000 - 0x30FFFF`: Video controller registers (VRAM access)
        *   `0x320000`: Sound chip registers (YM2610)
        *   `0x380000`: Controller input ports
    *   Executes instructions step-by-step, feeding register states, opcode cycles, and PC addresses back to the frontend.
*   **`video.rs` (Graphics Pipeline):**
    *   Processes `C-ROM` graphics blocks (sprites, containing $16 \times 512$ tile arrays) and `S-ROM` (fix character layer).
    *   Assembles layers: Background sprites, active game sprites, and foreground "Fix" text layer.
    *   Outputs a flat raw pixel array (RGBA) at native NeoGeo resolution ($320 \times 224$ pixels) at a locked 60 FPS.
*   **`demo.rs` (Embedded Cyberpunk Demo ROM):**
    *   Features a compiled-in NeoGeo-style 68000 program and graphics set representing **"Neon Runner"**.
    *   Implements keyboard/gamepad-controlled neon ship running through an obstacles course with moving sprites, parallax scrolling grids, and a digital score counter.

---

### 2. **Futuristic Cyberpunk Frontend (`frontend`)**

To achieve the requested "futuristic emulators with professional effects", the UI will be designed with a state-of-the-art sci-fi user interface.

*   **Aesthetics & Visual Identity:**
    *   **Native Neon Console:** SDL2/egui panels with translucent-style colors and an animated scrolling cyber-grid rendered natively.
    *   **Harmonious Color Palette:** Sleek deep-slate bases, fluorescent cyan (`#00f0ff`) and hot magenta (`#ff007f`) neon highlights, amber alerts.
    *   **Interactive Micro-Animations:** Responsive holographic hover effects, glowing interfaces, and click sound transitions.
*   **Professional Options Panel:**
    *   **CRT Screen Simulator:** 
        *   *CRT Scanlines:* Real-time overlay of scanlines with customizable transparency.
        *   *CRT Curvature:* OpenGL/SDL2 shader-based barrel distortion simulating a physical convex retro monitor.
        *   *Phosphor Bloom:* Soft glow effect representing CRT cathode ray illumination.
        *   *Screen Flicker & Static:* Cyberpunk style switchable glitch overlays.
    *   **ROM Manager:** Native file dialog and CLI path loading. Displays parsed `.neo` game cards containing manufacturer details, game art placeholder, NGH serial, and category.
    *   **Input Controller Configurator:** Custom key-remapping panel for Players 1 and 2, supporting gamepads through SDL2 controllers.
    *   **Save/Load States Console:** A dashboard with 5 snapshot slots showing save timestamp, screenshot previews, and quick save/restore buttons.
*   **Real-time Diagnostics Dashboard:**
    *   **CPU Registers Panel:** Displays dynamic 68000 registers: `D0-D7` (Data registers), `A0-A7` (Address registers), `PC` (Program Counter), and `SR` (Status Register).
    *   **Assembly Stream:** Real-time disassembly of current running instructions (e.g., `MOVE.W`, `ADD.Q`, `JSR`).
    *   **Audio Visualizer:** Oscilloscope and frequency spectrum bars rendering audio streams in real-time.
    *   **Performance Metrics:** Displays active frames-per-second (FPS), cycle execution speed, and audio latency.

---

## ✅ Completado: Audio Resampler de Alta Calidad

[... el contenido se mantiene igual — ver README.md ...]

---

## ✅ Completado: Turismo de características

### ✅ BIOS Selector (Ctrl+B)
- Overlay full-screen con lista navegable de todas las BIOS disponibles en `bios/`.
- Enter aplica la BIOS seleccionada + reinicia CPU.
- Indicador de BIOS activa, highlight del slot seleccionado.
- Persistencia en `config/ngneon.conf`.

### ✅ Config Persistence
- Archivo `config/ngneon.conf` con formato `key=value`.
- `save_config_key()` / `load_config_key()` — helpers genéricos.
- `save_config_bool()` / `load_config_bool()` — conveniencia booleana.
- Keys preservadas al escribir: lectura → filtro → reescritura.

### ✅ Language System (Ctrl+L)
- `frontend/src/lang.rs` — `Language` enum + `Lang` struct con todas las strings traducibles.
- Español e Inglés completos: notificaciones, overlays, USAGE, slots, BIOS, títulos.
- `usage_text()` genera ayuda bilingüe.
- Persiste en `config/ngneon.conf`.

### ✅ Save State Manager (Ctrl+F12)
- Overlay full-screen: lista de 10 slots con timestamp, tamaño, miniatura.
- Thumbnails BMP del framebuffer (320×224) por slot.
- Acciones: F9 guardar, F10 cargar, Supr eliminar.
- Highlight, barra de acciones localizada.

### ✅ Slot Indicator (top-right)
- 10 cuadros compactos indicando: slot actual (cian), con datos (verde), vacío (gris).
- Números de slot visibles.
- Semi-transparente sobre el juego.

### ✅ Per-game Config Profiles
- `config/profiles/<sanitized_label>.conf` — overrides CRT por ROM.
- F2/F3/F4 guardan en global + per-game.
- Shift+F2/F3/F4 resetean el override del perfil → revierte a global.
- Perfil vacío se elimina automáticamente.

### ✅ Auto-save / Auto-load
- Slot 0 guardado al salir, cargado al iniciar.
- Última ROM restaurada desde config (`rom_path`).

### ✅ Diagnostic ROM Bank Dumps
- Al cargar una ROM, el core genera automáticamente volcados de diagnóstico de los 5 bancos.
- Soporta `.neo` y `.zip`; captura datos post-desencriptación (CMC, PCM2, SMA).
- Toggle global: `--dump-rom-banks` / `--no-dump-rom-banks` y clave `diagnostic_dumps`.

### ✅ CRT Effects (OpenGL 3.3)
- Scanlines (F2), CRT curvature (F3), phosphor bloom (F4).
- Vignette cuando curvature activa.

### ✅ ROM Browser (Ctrl+O)
- Grid de 3 columnas con box art (80×60 px) desde `media/`.
- Navegación con flechas, Enter para cargar, indicador de página.
- Escanea directorio de ROMs configurado.

---

## ✅ Completado: Settings Menu 5 Pestañas + Keyboard Config

### ✅ 5-Tab Settings Menu (Ctrl+S)
- **Tabs**: VIDEO (5 items), AUDIO (2 items), SYSTEM (4 items), CONTROLS (2 items), PATHS (4 items).
- Navegación: `←`/`→` cambia pestaña, `↑`/`↓` navega, `Enter` toggle/abrir sub-overlay.
- **VIDEO**: Scanlines, Curvature, Bloom, Fullscreen, **Window Scale** (2x/3x/4x).
- **AUDIO**: Volume (ajuste fino `←`/`→` ±5%), **Mute** (ON/OFF).
- **SYSTEM**: Language (ES/EN), Diagnostic Dumps, **Auto-save**, BIOS Selector (`>`).
- **CONTROLS**: Gamepad Config (`>`), **Keyboard Config** (`>`).
- **PATHS**: ROMs, BIOS, Screenshots, Saves (read-only, rutas resueltas).
- Archivos modificados: `lang.rs` (~25 nuevos campos localizados), `ui.rs` (5 tabs + helper `draw_settings_row`), `main.rs` (`handle_settings_enter` para tabs 3/4, `RuntimeStatus` extendido).

### ✅ Keyboard Config Overlay
- Accesible desde Ctrl+S → CONTROLS → Keyboard Config (`Enter`).
- **9 acciones remapeables**: Up, Down, Left, Right, A, B, C, D, Start, Coin.
- **Enter** inicia modo escucha (indicador de pulso visual).
- **R** restaura binding default de la acción seleccionada.
- **Esc** cancela escucha o cierra overlay.
- Sin duplicados: asignar tecla usada → se libera de acción previa.
- Persistencia: `config/keyboard.conf` (scancodes SDL2).

### ✅ KeyboardMapping (`input.rs`)
- `HashMap<Keycode, EmuAction>` con defaults (flechas + ZXCV + Enter + Space).
- `set(keycode, action)` con eliminación de binding previo.
- `process(keycode) -> Option<EmuAction>`.
- `save_to_config()` / `load_from_config()`.
- Global static `GLOBAL_KEYBOARD_MAPPING: Mutex<KeyboardMapping>`.
- `set_global_mapping()` / `get_global_mapping()`.
- `process_event()` usando el mapping global.
- **5 tests**: default map, roundtrip keycode_name, save/load, set duplicados, roundtrip common keys.

### ✅ keycode_name / find_keycode_by_name completos (~90 variantes SDL2)
- **Alfanuméricas**: A-Z, 0-9.
- **F1-F24**: todas las funciones extendidas.
- **Flechas + modificadores**: Up/Down/Left/Right, Shift/Ctrl/Alt, LGui, RGui, Mode.
- **Numpad**: KP0-KP9, KpEnter, KpDivide (`KP/`), KpMultiply (`KP*`), KpMinus (`KP-`), KpPlus (`KP+`), KpPeriod (`KP.`), KpEquals (`KP=`), KpComma (`Kp,`).
- **Puntuación**: Period (`.`), Comma (`,`), Slash (`/`), Backslash (`\`), Minus (`-`), Equals (`=`), LeftBracket (`[`), RightBracket (`]`), Semicolon (`;`), Backquote (`` ` ``).
- **Navegación/edición**: Space, Enter, Esc, BkSp, Tab, Ins, Home, End, PgUp, PgDn, Del, PrtSc, Pause, Menu, Caps, ScrollLock, NumLk.
- **Media**: Mute, VolUp, VolDn, AudioNext, AudioPrev, AudioStop, AudioPlay, AudioMute, MediaSelect.
- **Browser**: Www, Mail, Calculator, Computer, AcSearch, AcHome, AcBack, AcForward, AcStop, AcRefresh, AcBookmarks.
- **Sistema**: Power, Sleep, Eject, BrightnessDown, BrightnessUp.

### ✅ Bug Fixes aplicados
- **Infinite recursion**: `impl Default for KeyboardMapping` llamaba `Self::default()` → corregido a `KeyboardMapping::default()` (inherent method).
- **Duplicate doc comment**: `draw_settings_row()` tenía docs duplicados y contradictorios (`String` vs `&str`) → limpiado.
- **Hardcoded paths**: `draw_settings_menu()` usaba strings fijos (`"roms"`, `"bios"`) → ahora usa rutas resueltas con `rom_dir.to_string_lossy()`.
- **Muted field sync**: Ctrl+M handler no actualizaba `status.muted` → corregido. Ctrl+=/Ctrl+- ahora limpian `status.muted = false`.
- **Mute check inconsistency**: `handle_settings_enter` usaba `muted_volume == 0` en lugar de `!status.muted` → unificado.
- **F2-F12 roundtrip**: `keycode_name` no tenía F2-F12 → test fallaba → añadidos.
- **Window scale startup**: escala no se aplicaba al iniciar → ahora `window.set_size()` si `window_scale != WINDOW_SCALE`.
- **Auto-save gating**: auto-save al salir ahora respeta `status.auto_save`.
- **Localized listening text**: `draw_keyboard_config` tenía texto de escucha hardcodeado en inglés → usa `lang.kb_listening`.
- **Input gating**: el guard de input ahora incluye `show_kb_config` para evitar procesar teclas de juego durante la configuración.

---

## ✅ Completado: Gamepad Config Persistence + Hotplug

### ✅ GamepadManager (`frontend/src/gamepad.rs`)
- **`GamepadManager`**: struct con `controllers: Vec<GameController>`, `mappings: Vec<ControllerMapping>`, `sticks: Vec<StickState>`, `guids: Vec<String>`.
- **`ControllerMapping`**: array de 15 `EmuAction` indexado por SDL2 `Button` discriminant.
- **Default mapping**: A/B/X/Y → A/B/C/D, DPad → direcciones, Start → Start, Back → Coin, LB → Coin, RB → Start.

### ✅ Hotplug Detection
- `handle_hotplug()` procesa eventos `ControllerDeviceAdded` / `ControllerDeviceRemoved`.
- Al conectar: abre el controller, carga mapping desde `config/gamepad/<guid>.conf`.
- Al desconectar: elimina controller, mapping y stick state del manager.

### ✅ Gamepad Persistence (`config/gamepad/<sanitized_guid>.conf`)
- Formato: `button_name=action_name` (ej. `A=A`, `D-Up=Up`, `Start=Start`).
- **Un archivo por controller**, identificado por GUID sanitizado.
- `save_all()` persiste todos los mappings conectados.
- `load_mapping_for_guid(guid)` carga desde disco o devuelve `default_mapping()`.
- Directorio `config/gamepad/` creado automáticamente.

### ✅ Integración completa
| Evento | Acción |
|---|---|
| Startup | `scan_initial()` → carga mappings de controllers ya conectados |
| Hotplug connect | `handle_hotplug()` → carga mapping del nuevo controller |
| Cerrar overlay (Esc) | `gamepad_mgr.save_all()` |
| Restaurar defaults (R) | `gamepad_mgr.save_all()` tras reset |
| Reasignar botón | `gamepad_mgr.save_all()` tras cada binding |
| Salir del programa | `gamepad_mgr.save_all()` al cerrar |

### ✅ Analog Stick D-Pad Emulation
- Deadzone de ±8000 en LeftX/LeftY.
- Edge-crossing detection: solo emite `EmuAction` al cruzar el umbral.
- Retorno a centro detectado correctamente (emite acción contraria al soltar).

### ✅ Tests (5 tests)
- `default_mapping_is_sane`: verifica mapping por defecto.
- `button_name_roundtrip`: 15 botones → nombre → botón.
- `action_name_roundtrip`: 10 acciones → nombre → acción.
- `mapping_set_and_get`: asignación y lectura.
- `stick_deadzone_is_positive`: validación de deadzone.

### ✅ button_name / find_button_by_name (15 botones SDL2)
- A, B, X, Y, Back, Guide, Start, LStick, RStick, LB, RB, D-Up, D-Down, D-Left, D-Right.

---

## ✅ Completado: Expansión de Fuente Bitmap (34→60 glifos)

### ✅ Glifos nuevos
- **A-Z completo** (índices 0-25): mayúsculas renderizadas correctamente.
- **Minúsculas mapean a mayúsculas**: cualquier carácter a-z (ASCII 97-122) se convierte automáticamente a su glifo mayúscula.
- **Caracteres acentuados españoles**: `á`(52), `é`(53), `í`(54), `ó`(55), `ú`(56), `ñ`(57) con glifos propios.
- **Flechas** (26-29): ↑ ↓ ← → para navegación.
- **Símbolos de UI** (30-37): `>`, `:`, `.`, `/`, `-`, `(`, `)`, espacio.
- **Dígitos 0-9** (38-47): para contadores y timestamps.
- **Puntuación extra** (48-51): `!`, `'`, `"`, `…` (elipsis).
- **Bloques para barra de volumen** (58-59): `█` (lleno) y `░` (vacío).

### ✅ Impacto
- Labels del menú de configuración ahora muestran texto correcto en vez de espacios.
- Todos los overlays (BIOS, Save States, Settings, Gamepad, Keyboard) se benefician de mayúsculas/minúsculas correctas.
- La barra de volumen en Settings → AUDIO usa bloques visuales en vez de texto.

---

## ✅ Completado: Corrección de Navegación del Menú de Configuración

### ✅ Problema resuelto
- **Antes**: `←`/`→` tenían doble función (cambiar pestaña Y ajustar volumen), causando que no se pudiera llegar a las pestañas CONTROLS y PATHS cuando el volumen estaba seleccionado.
- **Ahora**: `←`/`→` **siempre** cambian de pestaña. El ajuste de volumen usa un **modo dedicado**.

### ✅ Modo de ajuste de volumen
- `Enter` sobre «Volume» en la pestaña AUDIO → entra al modo de ajuste (`settings_vol_adjusting = true`).
- En el modo: `←`/`→` ajustan volumen ±5%.
- `Enter` o `Esc` salen del modo.
- Navegar con `↑`/`↓` fuera de Volume **sale automáticamente** del modo.
- Reabrir el menú con `Ctrl+S` limpia el flag.

### ✅ Corrección de bugs de estado obsoleto
- **Bug**: Navegar con `↑`/`↓` fuera de Volume mientras estabas en modo ajuste dejaba el flag `true` → `←`/`→` seguían ajustando volumen en cualquier pestaña.
- **Fix**: Los handlers de `↑`/`↓` ahora limpian `settings_vol_adjusting = false`.
- **Bug**: Reabrir settings con `Ctrl+S` después de haber estado en modo ajuste mantenía el flag `true`.
- **Fix**: El handler de `Ctrl+S` ahora limpia `settings_vol_adjusting = false`.

---

## ✅ Completado: Directorio BIOS Configurable

### ✅ `resolve_bios_directory()`
- Nueva función en `main.rs` que resuelve el directorio de BIOS con prioridad:
  1. Clave `bios_dir` en `config/ngneon.conf`
  2. `<exe_dir>/bios`
  3. `<cwd>/bios`
  4. Fallback: `bios/`
- Sigue el mismo patrón que `resolve_rom_directory()`.

### ✅ Cambios en APIs
- **`load_default_bios(neogeo, bios_dir)`**: ahora acepta `bios_dir: &str`.
- **`RuntimeStatus::new(label, language, bios_dir)`**: ahora acepta `bios_dir: &str`.
- **7 hardcodeos de `"bios"` reemplazados** por la variable resuelta:
  - `load_default_bios()` — carga inicial de BIOS/SFIX/L0.
  - `RuntimeStatus::new()` — escaneo de BIOS disponibles para el selector.
  - Handler de `Enter` en BIOS Selector — aplicar BIOS seleccionada.
  - Carga de BIOS guardada desde config.
  - `draw_settings_menu()` — pestaña PATHS muestra la ruta real.

### ✅ Config
- Nueva clave `bios_dir=bios` en `config/ngneon.conf`.
- La clave `bios` guarda el **label** de BIOS, `bios_dir` guarda la **ruta** del directorio.
- Eliminado valor incorrecto `bios=C:\Users\...\roms` que era una ruta, no un label.
- La pestaña PATHS ahora muestra la ruta resuelta (ej. `C:\...\bios`) en vez de solo `"bios"`.

---

## ✅ Verificación

- **Build**: `cargo build --release --workspace` — compila limpio (3 warnings menores).
- **Tests**: `cargo test --workspace` — **253/253 passing**.
- **Code review**: revisado por code-reviewer-deepseek en múltiples iteraciones.

---

## 🔮 Roadmap: Próximos pasos sugeridos

### Prioridad alta — Emulación
- **Transición BIOS → Cartucho**: backup RAM/calendario inicial, limpieza de fix durante BIOS, entrega de control al cartucho para obtener escenas visibles en ROMs reales.
- **Handshake Z80/YM2610**: validar respuestas de M1 reales, estado YM con detalle, enable/disable de NMI, bank switching por byte alto del puerto.
- **Protecciones**: refinar handlers para MSlugX (`0x2FFFF0`), KOF2002 (CMC50/PCM2), MSlug3 (SMA/CMC42).
- **Temporización fina**: VBlank/HBlank preciso, mezcla YM2610 más exacta, save RAM persistente en disco.
- **L0 table**: ampliar uso para sprites altos (>16 tiles).

### Prioridad alta — UI/Frontend
- **ROM Browser con filtros**: búsqueda por nombre, género, año, NGH. Mejorar presentación del grid.
- **Netplay básico**: rollback + sincronización de inputs para 2 jugadores en red (GGPO-style).
- **Perfiles de gamepad por juego**: diferentes mappings de gamepad para diferentes ROMs.

### Prioridad media — Efectos/Extras
- **Grabación de video/audio**: capturar frames + audio ring buffer → `.mp4`/`.webm`.
- **Shaders GLSL adicionales**: NTSC composite artifacts, máscara de subpíxeles RGB, phosphor decay configurable.
- **Remapping de sticks analógicos**: permitir reasignar ejes LeftX/LeftY a acciones no-direccionales.
- **Audio visualizer**: espectro de frecuencias en tiempo real sobre overlay de debug.
- **CI/CD**: GitHub Actions para builds Windows/Linux con artifacts automáticos.

### Prioridad baja — Pulido
- **Más temas visuales**: themes de color alternativos (ámbar monocromo, verde fósforo, blanco y negro).
- **Achievements/trophies**: integración con RetroAchievements.
- **Cheats**: motor de búsqueda y aplicación de cheats (GameShark/Action Replay).

---

## Verification & Testing Plan

### Automated & Unit Tests
1.  **ROM Parser Tests:** Verification of `.neo` headers with test vectors, ensuring all chunk sizes (`PSize`, `CSize`, etc.) are read and aligned correctly.
2.  **Zip Extraction Tests:** Mock zip file reading to verify standard bin name recognition.
3.  **CPU Assembly Decoding:** Unit tests verifying standard 68000 opcodes decoded by the CPU emulator match reference values.
4.  **Keyboard Mapping Tests:** Roundtrip keycode_name↔find_keycode_by_name, save/load config, set with duplicates, default map validation.
5.  **Gamepad Tests:** Button name roundtrip, action name roundtrip, default mapping sanity, set/get, deadzone validation.

### Manual Verification
1.  **Cross-Platform Boot Test:** Launch the native executable on Windows, macOS, and Linux to ensure the SDL2 window, input, renderer, and sound mixer initialize correctly.
2.  **Instant Playability Test:** Boot the emulator without loading a file to confirm the custom "Neon Runner" demo loads instantly and runs smoothly at 60 FPS.
3.  **Visual Effects Test:** Toggle scanlines, CRT curvature, bloom, and speed filters, validating they scale responsively without dragging performance.
4.  **ROM Loading Test:** Load test ROMs in `.zip` and `.neo` formats through the native file dialog and CLI argument path to verify parsing and diagnostics initialization.
5.  **Keyboard Config Test:** Open Settings → CONTROLS → Keyboard Config, remap keys, verify persistence across restarts, test roundtrip of all punctuation and extended keys.
6.  **Gamepad Config Test:** Connect controller, open Ctrl+G, remap buttons, verify persistence in `config/gamepad/<guid>.conf`, test hotplug disconnect/reconnect.
