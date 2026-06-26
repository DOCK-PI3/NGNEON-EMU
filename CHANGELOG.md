# Changelog de NGNEON-EMU

Historial consolidado de cambios implementados durante el desarrollo del emulador.  
El proyecto combina un núcleo Neo Geo en Rust, un frontend nativo SDL2/OpenGL y un configurador web local.

> Nota legal: NGNEON-EMU no distribuye BIOS ni ROMs comerciales. El usuario debe aportar sus propios archivos.

---

## 2026-06-25 — Compatibilidad completa de ROMs ZIP auditadas

### Loader ZIP

- Añadido soporte real de auditoría runtime para ROMs `.zip` además de `.neo`.
- El auditor `audit_neo_library` ahora:
  - detecta `.neo` y `.zip`;
  - permite filtrar por nombre con o sin extensión;
  - carga mediante `RomData::from_neo()` o `RomData::from_zip()`;
  - mide arranque, cambio a vectores de cartucho, vídeo, audio, cadencia y errores CPU.
- Corregido el layout de P-ROM de 2MB estilo FBNeo/MAME `LOAD + CONTINUE`.
  - Muchos sets clásicos tenían el header de cartucho en `0x100100`, no en `0x100`.
  - Se detecta automáticamente y se intercambian mitades del primer P-ROM cuando corresponde.
  - Esto desbloqueó sets que antes quedaban en BIOS pese a cargar sin error.
- Añadido soporte ZIP para KOF98 cifrado:
  - detección por chips `242-p1.p1`, `242-p2.sp2`, `242-c*.c*`, `242-m1.m1`, `242-v1.v1`;
  - aplicación del descifrado equivalente a `kof98Decrypt()` de FBNeo;
  - activación correcta del board runtime `Kof98`.
- Corregido `irrmaze.zip`:
  - los archivos internos `236-bios.sp1` y `236-bios_japan_hack.sp1` se filtran como BIOS internas;
  - ya no se clasifican erróneamente como P-ROM por su extensión `.sp1`;
  - el set arranca y pasa auditoría runtime.
- Añadidas excepciones documentadas en auditoría para boots silenciosos esperados:
  - `diggerma`;
  - `dragonsh`, cuyo sample ROM aparece como `NODUMP` en FBNeo.

### Validación

- Auditoría runtime completa de `roms_zip/`:
  - `total=160`;
  - `loaded=160`;
  - `load_failed=0`;
  - `runtime_passed=160`;
  - `runtime_review=0`;
  - `runtime_failed=0`.
- Auditoría rápida ZIP:
  - `total=160`;
  - `loaded=160`;
  - `failed=0`.
- Tests del loader ROM:
  - `88 passed`.
- Build final:
  - `cargo build --release --workspace` correcto.

---

## 2026-06 — Compatibilidad avanzada `.neo`, Geolith/FBNeo y timing

### Parser `.neo`

- El parser `.neo` acepta versiones alternativas del formato:
  - `0x00`;
  - `0x01`;
  - `0x02`;
  - `0x03`;
  - `0x05`.
- Se preserva la versión original en metadata.
- Se mantiene compatibilidad con `.neo` oficiales y variantes legacy.
- Se añadieron heurísticas para validar board types en función del P-ROM real.

### Detección de boards y protecciones

- Implementada validación runtime de placas SMA:
  - evita clasificar como SMA ROMs descifradas o pequeñas;
  - replica la heurística de Geolith: los cartuchos SMA reales usan P-ROM grande.
- Implementada detección de bootlegs KOF2003:
  - `kf2k3bla`;
  - `kf2k3pl`;
  - `kf2k3bl`;
  - `kf2k3upl`.
- Añadido soporte NEO-PVC:
  - KOF2003;
  - Metal Slug 5;
  - SVC Chaos.
- Implementado bankswitching NEO-PVC:
  - `pvc_unpack`;
  - `pvc_pack`;
  - `pvc_bankswap`;
  - cart RAM de 8KB;
  - serialización en save states.
- Corregidas escrituras atómicas PVC:
  - `write16`;
  - `write32`;
  - evita disparar operaciones PVC con datos incompletos.
- Inicialización correcta de `pvc_bank_addr`:
  - `0x100000` cuando el P-ROM supera 1MB;
  - replica el comportamiento de Geolith.

### CMC, PCM2, SMA y sets cifrados

- Integradas rutas de descifrado para CMC42/CMC50.
- Extracción de S-ROM desde C-ROM para juegos CMC cuando la S-ROM viene vacía.
- Soporte M1 cifrado para sets CMC50.
- Soporte PCM2 para reorganización de muestras.
- Soporte SMA P-ROM para sets como:
  - Garou;
  - KOF99;
  - KOF2000;
  - Metal Slug 3.
- Verificación de sets problemáticos:
  - `garou`;
  - `kof99`;
  - `mslug4`;
  - `s1945p`.

### FIX layer y vídeo

- Restaurada la capa FIX a tiles 8x8.
  - Se revirtió una interpretación incorrecta del bit 5 de `LSPC_MODE`.
  - Ese bit pertenece al temporizador IRQ, no a altura de tiles FIX.
- Añadido bankswitching FIX por tile:
  - KOF2000;
  - Matrimelee;
  - SVC;
  - KOF2003.
- Implementado cálculo de banco FIX por tile replicando Geolith.
- Añadida verificación del registro LSPC RomSize (`0x3C000C`) contra máscara C-ROM.
- `decode_sprite_tile()` usa el valor del registro LSPC RomSize cuando está inicializado.
- Captura diagnóstica automática retrasada hasta frame 120 para evitar capturas negras.
- Incremento correcto de `frame_count`, usado por notificaciones y diagnóstico.

### Audio y sincronización

- Corregida la cadencia de audio por frame.
  - Se reemplazó el uso de `round(44100 / MVS_FRAME_RATE)`.
  - Ahora se conserva el resto fraccional entre frames.
  - Esto evita drift acumulado y mantiene audio/vídeo sincronizados.
- El acumulador se limpia al reset/carga de ROM.
- Corregida lectura de status del YM2610:
  - leer status limpia las flags de Timer A/B;
  - permite que el driver Z80 alcance el secuenciador FM;
  - se recupera música FM que antes no sonaba.
- Mejoras en ADPCM-A/B RAM:
  - buffers internos para pruebas de BIOS;
  - mapeo correcto de lectura/escritura;
  - integración en save states.
- Auditorías PCM comparativas con Geolith para validar comportamiento.

---

## 2026-06 — Frontend nativo SDL2/OpenGL

### Frontend

- Frontend nativo en Rust con SDL2 + OpenGL.
- Separación clara entre:
  - `core-emulator`;
  - `frontend`.
- Ejecución de ROM desde línea de comandos:
  - `.neo`;
  - `.zip`;
  - demo interna.
- Selector nativo de archivos si se arranca sin ROM.
- Ocultación automática del cursor durante juego.
- Restauración de cursor al volver a demo interna.

### Render OpenGL

- Pipeline CRT en GPU con OpenGL 3.3.
- Efectos:
  - scanlines;
  - curvatura CRT;
  - phosphor bloom.
- Render sin CRT en overlays donde importa la nitidez:
  - ROM Browser;
  - carátulas;
  - texto.
- Soporte para escala de ventana:
  - 2x;
  - 3x;
  - 4x.
- Soporte de pantalla completa.
- Trabajo de base para presentación 4:3/16:9 y preservación de aspecto.

### Overlays y UI

- Overlay de debug con datos CPU/runtime.
- Sistema de notificaciones.
- Selector de BIOS.
- Save State Manager con miniaturas.
- Slot indicator.
- Menú de configuración con pestañas:
  - VIDEO;
  - AUDIO;
  - SYSTEM;
  - CONTROLS;
  - PATHS;
  - RA.
- Overlay de perfil por juego.
- Overlay de configuración de teclado.
- Overlay de configuración de gamepad.
- Fuente bitmap extendida:
  - letras;
  - números;
  - flechas;
  - acentos;
  - símbolos de barra de volumen.

### Navegador de ROMs

- ROM Browser con `Ctrl+O`.
- Grid con box art/cartucho.
- Soporte de `media_dir`.
- Carga proporcional de imágenes PNG.
- Navegación con teclado.
- Carga directa de ROM seleccionada.
- Indicador de páginas.
- Fallback visual cuando falta media.

---

## 2026-06 — Configuración persistente y perfiles

### Config global

- Sistema unificado en `config/ngneon.conf`.
- Persistencia de:
  - BIOS activa;
  - directorio BIOS;
  - directorio media;
  - idioma;
  - scanlines;
  - curvatura;
  - bloom;
  - fullscreen;
  - última ROM;
  - volumen;
  - mute;
  - auto-save;
  - volcado diagnóstico;
  - escala de ventana;
  - gamepad SDL2;
  - RetroAchievements.

### Perfiles por juego

- Perfiles en `config/profiles/<label>.conf`.
- Overrides por ROM para:
  - scanlines;
  - curvature;
  - bloom.
- Helpers que preservan claves existentes al guardar.

### Rutas

- Resolución robusta de BIOS:
  - `bios_dir` en config;
  - `<exe>/bios`;
  - `./bios`;
  - `bios/`.
- PATHS tab muestra rutas resueltas.
- Carpetas de saves, screenshots y media centralizadas.

---

## 2026-06 — Controles, teclado y gamepad

### Teclado

- Mapping global configurable.
- Persistencia en `config/keyboard.conf`.
- Acciones:
  - Up;
  - Down;
  - Left;
  - Right;
  - A;
  - B;
  - C;
  - D;
  - Start;
  - Coin.
- Reasignación desde overlay.
- Eliminación automática de duplicados.
- Restauración individual de defaults.
- Soporte de ~90 variantes SDL2.

### Gamepad

- Backend SDL2 GameController.
- Activación/desactivación desde configuración.
- Modo seguro por defecto (`gamepad=off`) para evitar bloqueos en Windows.
- Hotplug.
- Multi-controller.
- Stick analógico como D-Pad.
- Deadzone.
- Persistencia por GUID en `config/gamepad/<guid>.conf`.
- Acciones globales:
  - salir con `Back+Start`;
  - abrir ROM Browser con `Guide`.
- Reasignación desde overlay con modo escucha.

---

## 2026-06 — Save states, capturas y diagnóstico

- 10 slots por ROM.
- Auto-save en slot 0 al salir.
- Auto-load en slot 0 al iniciar.
- Miniaturas BMP para Save State Manager.
- Screenshots BMP con `F12`.
- Auto-captura al cargar ROM.
- Auto-volcado WAV.
- Dumps diagnósticos:
  - P-ROM;
  - C-ROM;
  - S-ROM;
  - M-ROM;
  - V-ROM.
- Flag para desactivar dumps:
  - `--no-dump-rom-banks`;
  - `diagnostic_dumps=off`.
- Herramientas auxiliares:
  - `probe_rom`;
  - `diagnose_rom`;
  - `audit_zip_library`;
  - `audit_neo_library`;
  - comparadores PCM;
  - capturas headless.

---

## 2026-06 — BIOS

- Carga de BIOS desde:
  - archivos sueltos;
  - `neogeo.zip`;
  - `aes.zip`.
- Selección por label persistente.
- Selector de BIOS con `Ctrl+B`.
- Carga de:
  - BIOS 68K;
  - L0/zoom ROM;
  - SFIX;
  - SM1.
- Prioridad configurable mediante `NGNEON_BIOS_HINT`.
- Fallback a BIOS diagnóstica interna para pruebas sintéticas.

---

## 2026-06 — RetroAchievements

- Integración con `rcheevos`.
- Configuración persistente:
  - `ra_token`;
  - `ra_username`;
  - `ra_password`;
  - `ra_hardcore`.
- Preferencia por token frente a usuario/contraseña.
- Login automático cuando hay credenciales.
- Auditoría específica para casos de logros.
- Notificaciones RA con expiración por frame.
- Corrección de tirones relacionados con evaluación/estado mediante revisión de integración y frame counter.
- Panel RA en configurador web local:
  - perfil;
  - puntos;
  - rango;
  - true points;
  - últimos juegos;
  - últimos logros;
  - progreso.

---

## 2026-06 — Configurador web local

- Proyecto `configurator/`.
- Launcher/configurador web local en `127.0.0.1:4177`.
- Lectura de `roms/gamelist.xml`.
- Biblioteca visual estilo consola mini.
- Media por juego:
  - vídeo preview;
  - fanart;
  - captura;
  - mix;
  - título;
  - caja 3D;
  - cartucho;
  - manual.
- Descripciones desde `gamelist.xml`.
- Lanzamiento de ROMs desde la interfaz.
- Favoritos.
- Juegos recientes.
- Buscador.
- Integración con RetroAchievements Web API.
- Protección de secretos:
  - `ra_token`;
  - `ra_password`;
  - `ra_api_key`;
  - no se exponen al navegador.
- Ajustes visuales para:
  - cajas 3D no recortadas;
  - vídeo y media sin estirar;
  - desplazamiento de media para evitar solapamiento con descripción.

---

## 2026-05 / 2026-06 — Núcleo de emulación

### CPU y memoria

- Núcleo de emulación en Rust.
- Integración de 68000 mediante Musashi FFI.
- Integración de Z80.
- Mapa de memoria Neo Geo.
- Work RAM.
- Palette RAM.
- VRAM.
- NVRAM.
- Memcard.
- Cart RAM para placas especiales.
- Watchdog.
- Comunicación 68K/Z80.

### Vídeo

- LSPC.
- Sprites.
- FIX layer.
- Palette handling.
- Shrink/zoom usando L0 cuando está disponible.
- Soporte de máscaras C-ROM.
- Diagnósticos de VRAM, SCB y palette.

### Audio

- YM2610.
- FM/SSG/ADPCM.
- Mixer estéreo.
- Resampler Lanczos-3 en frontend.
- Ring buffer SDL2.
- Auditorías de cadencia y tamaño de frame.

### Save states

- Serialización de estado.
- Versión de save state actualizada al añadir nuevos buffers/estado.
- Backward compatibility donde aplica.

---

## Estado actual validado

- `.neo`: ruta principal estabilizada y auditada durante el trabajo de compatibilidad.
- `.zip`: auditoría runtime local completa `160/160`.
- Audio/vídeo:
  - cadencia exacta de audio por frame;
  - sample count estable;
  - sincronización validada por auditorías.
- Build:
  - `cargo build --release --workspace` correcto.
- Tests relevantes:
  - core ROM tests;
  - frontend audio tests;
  - auditorías ZIP/runtime.

---

## Comandos de verificación útiles

```powershell
cargo fmt --all
cargo test --workspace
cargo test -p core-emulator rom::tests:: -- --test-threads=1
cargo test -p frontend audio::tests:: -- --test-threads=1
cargo build --release --workspace
cargo run --release -p core-emulator --bin audit_zip_library -- roms_zip
cargo run --release -p core-emulator --bin audit_neo_library -- roms_zip 1200
```

