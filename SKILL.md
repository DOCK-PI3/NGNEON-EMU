# SKILL.md


## Skill: Extensión de UI nativa y efectos avanzados

### Propósito
Automatiza y documenta la extensión de la interfaz gráfica nativa (egui/SDL2) para NGNEON-EMU, incluyendo overlays, CRT, configuradores, dashboards, y efectos visuales avanzados.

### Instrucciones para agentes
- Proponer y generar código Rust para nuevas pantallas, paneles de configuración, y overlays visuales.
- Integrar efectos CRT, scanlines, bloom y curvatura usando shaders compatibles con OpenGL ES 2.0.
- Documentar cómo añadir nuevos widgets o paneles en egui/SDL2.
- Sugerir patrones para mantener la UI desacoplada del núcleo de emulación.
- Enlazar a documentación relevante y ejemplos de código.

### Overlays actuales (dibujados en framebuffer 320×224)
- **Debug overlay (F6)**: FPS, PC, SR, ciclos CPU, label ROM.
- **Notifications**: mensajes toast localizados con timer de fade-out.
- **BIOS Selector (Ctrl+B)**: lista navegable, Enter aplica + reinicia CPU. Usa `resolve_bios_directory()` para encontrar el directorio BIOS (configurable con clave `bios_dir` en `config/ngneon.conf`).
- **Save State Manager (Ctrl+F12)**: 10 slots, miniaturas BMP, guardar/cargar/eliminar.
- **Slot Indicator**: esquina superior derecha, 10 cuadros (cian/verde/gris).
- **Settings Menu (Ctrl+S)**: 5 pestañas (VIDEO/AUDIO/SYSTEM/CONTROLS/PATHS). ←/→ siempre cambian de pestaña. Volumen usa modo de ajuste dedicado (Enter para entrar, ←/→ ajustan, Enter/Esc salen).
- **Profile Config (Ctrl+P)**: overrides CRT per-game (Global/Override).
- **Gamepad Config (Ctrl+G)**: overlay de configuración de gamepad con modo escucha de botones.
- **Keyboard Config**: overlay de reasignación de teclas, accesible desde Settings → CONTROLS.
- **ROM Browser (Ctrl+O)**: grid de 3 columnas con box art (80×60 px).

### Fuente bitmap (60 glifos)
La fuente de los overlays soporta 60 glifos: A-Z, flechas ↑↓←→, símbolos de UI (`>`, `:`, `.`, `/`, `-`, `(`, `)`, espacio), dígitos 0-9, puntuación extra (`!`, `'`, `"`, `…`), caracteres acentuados españoles (`áéíóúñ`), y bloques `█`/`░` para la barra de volumen. Las minúsculas (a-z) mapean automáticamente a mayúsculas. Ver `draw_char()` en `ui.rs` para los índices.

### Cómo añadir un nuevo overlay
1. Añadir `show_nuevo_overlay: bool` a `RuntimeStatus` en `main.rs`.
2. Añadir strings localizadas a `lang.rs` (ambos constructores).
3. Crear función `draw_nuevo_overlay()` en `ui.rs` siguiendo el patrón de `draw_keyboard_config()`:
   - Fondo semitransparente oscuro (`DIM_BG`).
   - Cabecera centrada con título.
   - Lista de items con highlight bar.
   - Barra de acciones/ayuda en la parte inferior.
4. Añadir guard en el event loop de `main.rs`: `if status.show_nuevo_overlay { handle... } else { ... }`.
5. Añadir guard en el bloque de input gating para evitar procesar teclas de juego durante el overlay.

---

## Skill: Soporte avanzado de gamepads

### Propósito
Automatiza la integración y remapeo de gamepads multiplataforma.

### Instrucciones para agentes
- Generar código Rust para detección y mapeo de gamepads usando SDL2.
- El sistema de hotplug y persistencia ya está implementado; extender sobre la base existente.
- Sugerir pruebas automáticas para verificar compatibilidad en Windows, macOS y Linux.

### Estado actual — TODO IMPLEMENTADO ✅

#### GamepadManager (`frontend/src/gamepad.rs`)
- **`GamepadManager`**: struct con `controllers: Vec<GameController>`, `mappings: Vec<ControllerMapping>`, `sticks: Vec<StickState>`, `guids: Vec<String>`.
- **`ControllerMapping`**: array de 15 `EmuAction` indexado por `Button` discriminant.
- **`button_name(button) -> &str`** / **`find_button_by_name(name) -> Option<Button>`**: roundtrip para 15 botones SDL2.
- **`action_name(action) -> &str`** / **`find_action_by_name(name) -> Option<EmuAction>`**: roundtrip para 10 acciones NeoGeo.
- **Analog stick D-pad emulation**: deadzone de ±8000, edge-crossing detection en `process_axis()`.

#### Hotplug detection ✅
- `handle_hotplug()` procesa eventos `ControllerDeviceAdded` / `ControllerDeviceRemoved`.
- Al conectar: abre el controller, carga mapping desde `config/gamepad/<guid>.conf`, lo añade al manager.
- Al desconectar: elimina controller, mapping y stick state del manager.

#### Persistencia ✅ (`config/gamepad/<guid>.conf`)
- Formato: `button_name=action_name` (ej. `A=A`, `D-Up=Up`, `Start=Start`).
- **`save_all()`**: guarda mappings de todos los controllers conectados a `config/gamepad/<sanitized_guid>.conf`.
- **`load_mapping_for_guid(guid)`**: carga mapping desde disco; si no existe, usa `default_mapping()`.
- **`save_mapping_for_guid(guid, mapping)`**: escribe archivo individual con header `# NGNEON-EMU gamepad button mapping`.
- El directorio `config/gamepad/` se crea automáticamente si no existe.

#### Puntos de integración
| Evento | Ubicación | Acción |
|---|---|---|
| Startup | `main.rs` → `scan_initial()` | Carga mappings para todos los controllers ya conectados |
| Hotplug connect | `gamepad.rs` → `handle_hotplug()` | Carga mapping del nuevo controller |
| Cerrar overlay (Esc) | `main.rs` | `gamepad_mgr.save_all()` |
| Restaurar defaults (R) | `main.rs` | `gamepad_mgr.save_all()` tras reset |
| Reasignar botón | `main.rs` | `gamepad_mgr.save_all()` tras cada binding |
| Salir del programa | `main.rs` | `gamepad_mgr.save_all()` al cerrar |

#### Overlay de Gamepad Config (Ctrl+G)
- Lista de 10 acciones NeoGeo (Up/Down/Left/Right/A/B/C/D/Start/Coin).
- **Enter** inicia modo escucha de botón SDL2.
- **R** restaura binding default de la acción seleccionada.
- **Esc** cancela escucha o cierra overlay.
- Soporte multi-controller: navegación entre controllers conectados.

#### Tests (5 tests en `gamepad.rs`)
- `default_mapping_is_sane`: verifica mapping por defecto.
- `button_name_roundtrip`: todos los botones → nombre → botón.
- `action_name_roundtrip`: todas las acciones → nombre → acción.
- `mapping_set_and_get`: asignación y lectura.
- `stick_deadzone_is_positive`: validación de deadzone.

#### Pendiente (mejoras futuras)
- **Remapping de sticks analógicos**: permitir reasignar ejes LeftX/LeftY a acciones no-direccionales.
- **Perfiles por juego**: diferentes mappings de gamepad para diferentes ROMs.
- **Soporte para triggers analógicos**: mapear L2/R2 como acciones (actualmente ignorados).

---

## Skill: Configuración de teclado (Keyboard Config)

### Propósito
Gestiona el mapeo y persistencia de teclas del teclado físico para emular los controles NeoGeo (joystick + botones A/B/C/D + Start + Coin).

### Archivos involucrados
- `frontend/src/input.rs` — `KeyboardMapping` struct, global static, persistencia.
- `frontend/src/ui.rs` — `draw_keyboard_config()` overlay.
- `frontend/src/lang.rs` — ~10 strings `kb_*` localizadas.
- `config/keyboard.conf` — archivo de persistencia (`ActionName=SDLK_SCANCODE`).
- `frontend/src/main.rs` — `RuntimeStatus` fields (`show_kb_config`, `kb_*`), event handling.

### API de KeyboardMapping
```rust
// Estructura principal
```

---

## Skill: Comparación binaria de dumps ROM y validación de vectores de reset

### Propósito
Automatiza la comparación de archivos binarios de bancos ROM (P-ROM, C-ROM, S-ROM, M-ROM, V-ROM) generados por NGNEON y Geolith, y valida los vectores de reset y stack pointer en la P-ROM.

### Instrucciones para agentes
- Leer y comparar archivos binarios (`screenshots/{label}_prom_dump.bin`, etc.) entre NGNEON y Geolith.
- Reportar diferencias byte a byte y mostrar los primeros offsets donde difieren.
- Extraer y mostrar el vector de reset (offset 0x000004) y stack pointer (offset 0x000000) de la P-ROM.
- Validar que el vector de reset apunte a una dirección válida (no 0x00000000 ni 0xFFFFFFFF, y dentro del rango de la P-ROM).
- Sugerir causas probables si los vectores son inválidos o hay diferencias críticas.
- Generar advertencias si la P-ROM es demasiado pequeña, está truncada o tiene cabecera corrupta.

### Ejemplo de uso
1. El agente detecta que una ROM no arranca y genera el dump `screenshots/aof_prom_dump.bin`.
2. El usuario proporciona el dump equivalente de Geolith.
3. El agente compara ambos archivos, muestra diferencias y valida los vectores.
4. Si el vector de reset es inválido o apunta fuera de rango, sugiere revisar el parser `.neo` o la extracción de la P-ROM.

---
pub struct KeyboardMapping {
    map: HashMap<Keycode, EmuAction>,
}

// Métodos clave
impl KeyboardMapping {
    pub fn default() -> Self;                          // flechas + ZXCV + Enter + Space
    pub fn set(&mut self, keycode: Keycode, action: EmuAction);  // asigna y elimina duplicados
    pub fn process(&self, kc: Keycode) -> Option<EmuAction>;     // lookup directo
    pub fn key_for_action(&self, action: EmuAction) -> Option<Keycode>;  // reverse lookup
    pub fn save_to_config<P: AsRef<Path>>(&self, path: P);       // guarda a disco
    pub fn load_from_config<P: AsRef<Path>>(&mut self, path: P); // carga de disco
}

// Funciones auxiliares
pub fn keycode_name(kc: Keycode) -> &'static str;       // ~90 variantes SDL2 → nombre
pub fn find_keycode_by_name(name: &str) -> Option<Keycode>;  // nombre → variante SDL2

// Global static
pub fn set_global_mapping(mapping: KeyboardMapping);
pub fn get_global_mapping() -> KeyboardMapping;
pub fn process_event(keycode: Keycode) -> Option<EmuAction>;
```

### Cómo extender el soporte de teclas
1. Añadir nueva variante `Keycode::*` a `keycode_name()` en `input.rs` (match arm → `"Nombre"`).
2. Añadir entrada inversa en `find_keycode_by_name()` (`"Nombre"` → `Some(Keycode::*)`).
3. Asegurar simetría: todo nombre en `keycode_name` debe tener entrada en `find_keycode_by_name`.
4. Los tests existentes (`keycode_name_roundtrip_common`) verifican la simetría para teclas comunes.
5. **No usar `Keycode::Apostrophe`** — no existe en `sdl2` crate v0.38. Usar `Keycode::Quote` si es necesario.

### Teclas soportadas actualmente (~90 variantes)
| Categoría | Teclas |
|---|---|
| Alfanuméricas | A-Z, 0-9 |
| Funciones | F1-F24 |
| Flechas | Up, Down, Left, Right |
| Modificadores | LShift, RShift, LCtrl, RCtrl, LAlt, RAlt, LGui, RGui, Mode |
| Navegación | Space, Enter, Esc, BkSp, Tab, Ins, Home, End, PgUp, PgDn, Del |
| Numpad | KP0-KP9, KpEnter, KpDivide, KpMultiply, KpMinus, KpPlus, KpPeriod, KpEquals, KpComma |
| Puntuación | Period, Comma, Slash, Backslash, Minus, Equals, LeftBracket, RightBracket, Semicolon, Backquote |
| Lock | Caps, NumLk, ScrollLock |
| Sistema | PrtSc, Pause, Menu |
| Media | Mute, VolUp, VolDn, AudioNext, AudioPrev, AudioStop, AudioPlay, AudioMute, MediaSelect |
| Browser | Www, Mail, Calc, MyPC, ASrch, AHome, ABack, AFwd, AcStop, ARefr, ABook |
| Power/Brillo | Power, Sleep, Eject, BrtDn, BrtUp |

### Persistencia
- Formato: `ActionName=SDLK_SCANCODE` (ej. `A=122`, `Start=13`, `Up=1073741906`).
- Archivo: `config/keyboard.conf`.
- Carga automática al iniciar (`load_keyboard_config()` en `run()`).
- Guardado al cerrar el overlay de Keyboard Config.
- Si el archivo no existe, se usa el mapping por defecto.

---

## Skill: Automatización de builds multiplataforma

### Propósito
Facilita la generación de binarios y paquetes para Windows, macOS y Linux.

### Instrucciones para agentes
- Proveer scripts y comandos para compilar y empaquetar el emulador en cada plataforma.
- Documentar dependencias y pasos de build.
- Sugerir integración con CI/CD (GitHub Actions, etc.) para builds automáticos.

---

## Skill: Save states, snapshots y grabación

### Propósito
Automatiza la implementación de save states, snapshots y grabación de video/audio.

### Instrucciones para agentes
- Generar código Rust para serializar/deserializar el estado del emulador.
- Documentar cómo capturar y guardar imágenes y audio.
- Sugerir UI para gestionar slots de save/load y grabaciones.

### Estado actual
- Save states serializados con `savestate.rs` en `core-emulator`.
- 10 slots por ROM en `saves/<label>.state.<n>` con miniaturas BMP.
- Auto-save slot 0 al salir (gated por `status.auto_save`).
- Auto-load slot 0 al iniciar.
- Gestor visual (Ctrl+F12) con guardar/cargar/eliminar.

---

## Skill: Panel de diagnóstico y debugging

### Propósito
Facilita la creación de paneles de diagnóstico en tiempo real.

### Instrucciones para agentes
- Proveer ejemplos de paneles para registros de CPU, FPS, uso de memoria, y debugging visual.
- Documentar cómo conectar estos paneles al núcleo de emulación.

---

## Skill: Pruebas automáticas y verificación

### Propósito
Automatiza la generación y ejecución de pruebas unitarias y de integración.

### Instrucciones para agentes
- Sugerir y generar tests para el parser de ROMs, CPU, video y audio.
- Documentar cómo ejecutar y validar los tests en todas las plataformas.

### Comandos de verificación
```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

### Tests actuales (253 passing)
- `core-emulator`: ROM parser, video decoder, memory map, RTC, CPU, Z80, YM2610, savestate.
- `frontend`: keyboard mapping (5 tests), gamepad (5 tests), audio resampler, language.

---

## Documentación relacionada
- [AGENTS.md](AGENTS.md) — guía para agentes, atajos, config, estructura.
- [README.md](README.md) — documentación de usuario, controles, features.
- [implementation_plan.md](implementation_plan.md) — plan técnico, roadmap, features completados.
