# Changelog de NGNEON-EMU

Historial consolidado de cambios implementados durante el desarrollo del emulador.  
El proyecto combina un núcleo Neo Geo en Rust, un frontend de biblioteca SDL2 y la presentación del emulador mediante SDL2/OpenGL.

> Nota legal: NGNEON-EMU no distribuye BIOS ni ROMs comerciales. El usuario debe aportar sus propios archivos.

---

## 2026-07-26 — Resoluciones HD y filtros de escalado GPU

- Añadidos presets de salida `auto`, `720p`, `1080p` y `1440p`; el modo
  automático conserva `window_scale` y pantalla completa usa la resolución del
  escritorio.
- Añadidos los filtros `nearest`, `pixel-perfect` y `sharp-bilinear`.
  Pixel Perfect centra el mayor múltiplo entero de la imagen visible 304×224;
  Sharp Bilinear mantiene zonas de píxel planas y suaviza únicamente sus
  fronteras en escalas no enteras.
- La relación de aspecto 4:3 sigue siendo el valor predeterminado. Todos los
  cambios se aplican en SDL2/OpenGL después del framebuffer, sin modificar el
  render LSPC ni la precisión del núcleo 320×224.
- Integrados los nuevos ajustes en el menú interno, el frontend nativo, la
  configuración de ejemplo y la documentación.

---

## 2026-07-25 — Efectos visuales, tipografía y animaciones para temas

- Ampliado el parser XML sin romper temas existentes: `<component>`, `<layer>`
  y `<media>` comparten ahora `brightness`, `contrast`, `saturation`,
  `grayscale`, `tint`, `blur`, sombra, glow, rotación, esquinas redondeadas y
  gradiente vertical/horizontal.
- Añadidos estilos a `game_title`, `game_description`, `game_metadata`,
  `game_credits` y `page_counter`: `text_color`, alineación horizontal/vertical,
  sombra suavizada, contorno, `letter_spacing`, `line_height`, `auto_fit` y
  `min_font_size`.
- Añadidas animaciones combinables `fade_in`, `slide_in`, `zoom_in` y `pulse`.
  Las entradas se reinician al cambiar de juego, tema o vista; `pulse` mantiene
  el refresco activo sólo cuando un elemento visible lo necesita.
- Los porcentajes aceptan entero, sufijo `%` o factor decimal según corresponda.
  El parser valida límites de radios, offsets, escalas y duraciones, exige los
  dos extremos de un gradiente y sigue rechazando atributos desconocidos.
- Las capas y la media estática se filtran una sola vez dentro de la caché de
  imágenes, cuya clave incluye la configuración de efectos. Los componentes se
  postprocesan tras su render funcional y los frames de vídeo sólo recorren la
  ruta de filtros cuando el slot declara algún efecto.
- Refinado **Fanart Horizon** como muestra funcional: fanart con colorimetría y
  gradiente, logo con sombra/glow/zoom/pulso sutil, título y créditos con
  tracking/contorno/sombra, y entradas escalonadas del carrusel.
- Actualizados `themes/README.md`, la plantilla editable, README y guía de
  agentes con atributos, orden de filtros, límites y recomendaciones de coste.
- Añadidas pruebas unitarias del parser, filtros, composición alpha,
  transformaciones animadas, tracking y wrapping de texto.
- Preparada la distribución de prueba
  `NGNEON-EMU-Windows-theme-effects-test`, configurada para abrir inicialmente
  con **Fanart Horizon** y sin ROMs, media, música ni partidas.

---

## 2026-07-24 — Fanart Horizon, fuentes de tema y carrusel horizontal 3D

- Añadido el componente `gamecarousel_horizontal`, interactivo y con cinco
  cajas 3D. La caja seleccionada se escala hasta ocupar más de media pantalla,
  se dibuja por encima de sus vecinas y las demás se reducen y atenúan.
- Los temas v2 pueden incluir una fuente TTF/OTF autocontenida mediante
  `<font source="assets/fuente.ttf"/>`. La ruta queda confinada a la carpeta
  del tema y, si la fuente falla, se conserva el fallback del sistema.
- `game_title` y `game_metadata` admiten `font_size`; el título añade
  `show_with_logo` y los metadatos pueden elegir `year`, `developer`,
  `publisher`, `players` y `manufacturer` con `fields`.
- Añadido **Fanart Horizon**: fanart a pantalla completa, logo flotante, nombre
  real a tamaño inicial 69, desarrollador/publisher y carrusel horizontal de
  cajas 3D, sin descripción ni paneles.
- Refinado **Fanart Horizon**: el carrusel de cajas se desplazó exactamente
  50 píxeles hacia arriba y el nuevo componente `game_credits` sitúa el
  desarrollador en la esquina inferior izquierda y el publisher en la derecha.
- Incluida **Rajdhani SemiBold** bajo licencia SIL Open Font License junto al
  tema, con su fichero `OFL.txt`.
- Actualizados parser, navegación, paginación, hitboxes, recarga en caliente,
  pruebas con media real, documentación y plantilla para los 20 componentes.

---

## 2026-07-23 — Prioridad arcade-marquee

- `game.marquee` usa ahora `roms/media/arcade-marquee` como colección
  predeterminada para todos los temas. Las rutas locales válidas de
  `gamelist.xml`, `screen-marquees` y el antiguo `marquees` quedan como
  fallbacks, en ese orden.
- Añadida una capa negra completamente opaca detrás de la zona de controles de
  **Neo Geo 29 Cabinet**, ocultando la abertura transparente del PNG sin cubrir
  la cabinet ni alterar el carrusel.
- Preparada la distribución de prueba
  `NGNEON-EMU-Windows-arcade-marquee-test`.
- Refinada la altura y posición vertical de `arcade-marquee` en **NGNEON
  Neon**, **Cyber List**, **Console Showcase** y las cabinets **Neo Geo 29**,
  **Neo Geo Green** y **Neo Geo Red**. Los tres layouts sin máscara usan
  `stretch` para conservar la composición completa sin recortar su franja
  inferior.
- Preparada la distribución de prueba
  `NGNEON-EMU-Windows-marquee-fit-test`.

---

## 2026-07-23 — Gamecarousels curvados y Neo Geo 29

- Añadidos los componentes `gamecarousel_left` y `gamecarousel_right`, con siete
  logos en vertical y una curva especular hacia el centro de la pantalla. Se
  aceptan también los alias `gamecarrousel_left/right`.
- El logo seleccionado se renderiza al final, por encima de sus vecinos, y usa
  `selected_scale` (`100..200`, porcentaje o factor decimal). Los demás usan
  `unselected_opacity` (`0..255` o `0.0..1.0`) y `arc` controla el desplazamiento
  horizontal que forma la curva.
- Todos los componentes aceptan ahora `opacity`; capas y slots media ya
  disponían de esta propiedad. Los valores omitidos conservan el comportamiento
  anterior, por lo que los temas existentes siguen siendo compatibles.
- Añadido **Neo Geo 29 Cabinet** con el asset original autocontenido, monitor y
  marquee enmascarados por la cabinet y un `gamecarousel_right` de logos sin
  paneles. El seleccionado usa escala 145 %, los demás 42 % de opacidad y la
  curva usa 72 píxeles lógicos.
- Refinada la geometría del marquee de **Neo Geo 29 Cabinet** para mantenerlo
  completamente dentro de la abertura superior de la máquina.
- Actualizados parser, hitboxes, navegación, paginación, render de temas,
  plantilla y documentación para los 18 componentes disponibles.
- Preparada la distribución de prueba
  `NGNEON-EMU-Windows-neogeo29-gamecarousel-test`.

---

## 2026-07-22 — Temas autocontenidos y cabinets Neo Geo

### Una carpeta aislada por tema

- Reorganizados todos los diseños incluidos como
  `themes/NombreSinEspacios/theme.xml`; sus imágenes y demás recursos propios
  viven ahora bajo `assets/` dentro de esa misma carpeta.
- El catálogo descubre XML tanto en la raíz de `frontend_theme_dir` como a un
  nivel de profundidad. Los temas planos antiguos siguen siendo válidos para
  no romper diseños externos existentes.
- Las rutas guardadas del formato anterior, como `themes/neon.xml`, se migran
  automáticamente usando el ID del tema. La nueva configuración predeterminada
  apunta a `themes/NGNEONNeon/theme.xml`.
- La plantilla pasó a `themes/ThemeTemplate/theme.xml.example` y documenta el
  nuevo layout autocontenido. Los empaquetadores Windows/Linux ya copian toda
  la jerarquía junto con sus assets.

### Carruseles de logos y cajas 3D

- Los componentes `vertical_list` y `horizontal_list` aceptan el nuevo atributo
  `artwork="logo|box3d"`.
- `logo` conserva la cadena de fallback logo → wheel → marquee y muestra siete
  juegos. `box3d` usa caja 3D → caja 2D → box art → miniatura; en vertical
  muestra cinco elementos de mayor tamaño acompañados por el título.
- La paginación, la rueda del ratón y los hitboxes se adaptan al número de
  elementos del carrusel activo.

### Nuevos temas Cabinet

- Añadido **Neo Geo Red Cabinet**, basado en la cabinet roja facilitada por el
  usuario: vídeo/captura dentro del monitor, marquee en la marquesina y
  carrusel vertical de logos a la derecha.
- Añadido **Neo Geo Green Cabinet**, basado en la cabinet verde: vídeo/captura
  en el monitor, carrusel vertical de siete logos con el nombre como fallback,
  caja 3D seleccionada entre la cabinet y la lista, y ficha de
  descripción/metadatos.
- Refinada la geometría de ambos temas usando las ventanas transparentes del
  asset original. El marquee usa un rectángulo `395×107`, con 6 píxeles de
  sangrado por cada lado respecto a la abertura `383×95`, y el monitor conserva
  `333×243`. La capa PNG de la cabinet se dibuja por encima del marquee y del
  vídeo, recortando el exceso bajo los biseles y evitando que asome el fondo al
  cambiar de juego.
- Añadido `chrome="panel|none"` a componentes alternativos y slots media. El
  valor predeterminado `panel` mantiene compatibles los temas existentes;
  `none` deja logos, imágenes y textos flotantes sin panel, borde, tarjeta,
  banda de selección, flecha ni aviso `SIN MEDIA`.
- Los dos temas Cabinet usan la variante sin marcos: Red presenta un carrusel
  limpio de logos; Green integra también caja 3D, descripción, metadatos y
  contador directamente sobre el fondo.
- Ambos PNG originales se conservan con transparencia y sin recomprimir dentro
  de `themes/NeoGeo*Cabinet/assets/cabinet.png`.

### Descripciones con auto-scroll y validación

- `game_description` ya no deja inaccesible el texto que excede su recuadro:
  espera 2,5 segundos, avanza línea a línea, pausa en el final y reinicia el
  ciclo. Al cambiar de juego o de vista vuelve al principio.
- Añadidas pruebas del parser `artwork`, fallbacks de cajas, descubrimiento de
  subcarpetas, compatibilidad de selectores antiguos y ciclo completo del
  auto-scroll.
- Los siete temas incluidos se renderizan en tests tanto en Biblioteca como en
  Ajustes; los layouts ricos se validaron además con las 157 ROMs y la media
  real del checkout local, incluyendo vídeo H.264, carruseles de logos y la
  caja 3D independiente del tema verde.
- Preparada la distribución independiente
  `NGNEON-EMU-Windows-cabinet-themes-refined-test`, sin ROMs, saves ni media
  del usuario.
- Preparada la revisión
  `NGNEON-EMU-Windows-cabinet-themes-frameless-test` con el sangrado del
  marquee y la presentación flotante sin recuadros.

---

## 2026-07-22 — Explorador de ROMs con media seleccionable

### Detección y selección de imágenes

- El explorador interno ya no usa una prioridad cerrada e inmutable de
  carátulas. Ahora detecta dinámicamente las subcarpetas disponibles bajo la
  raíz configurada en `media_dir`, que por defecto es `roms/media/`.
- Solo se ofrecen fuentes que contienen al menos una imagen cuyo nombre base
  coincide con una ROM de la biblioteca. Esto admite tanto categorías
  convencionales (`box2d`, `box3d`, `mix`, `screenshots`, `titles`, `fanart`,
  `logos`, `wheelhd`, `marquees`, `cartridges`) como carpetas creadas por otros
  scrapers (`steamgrid`, `flyers`, `figurines`, etc.).
- Añadida la opción `Ctrl+S → RUTAS → Imagen explorador`. La fuente activa se
  puede cambiar también mientras el explorador está abierto con `Tab` o la
  acción Neo Geo `C` del gamepad —botón X en el mapeo predeterminado—.
- La selección global persiste en `config/ngneon.conf` mediante
  `rom_browser_media=auto|root|<subcarpeta>`.
- El modo `AUTO` conserva fallbacks por juego: raíz, `box2d`, `box3d`, `mix`,
  `screenshots` y después las demás fuentes detectadas en orden estable.
- El nombre de la fuente activa se muestra en la cabecera del explorador y en
  el menú de configuración.

### Formatos, seguridad y rendimiento

- Se admiten imágenes PNG, JPG, JPEG y WebP; nombres y extensiones se comparan
  sin distinguir mayúsculas/minúsculas, también en sistemas de archivos
  sensibles a mayúsculas.
- Los valores de configuración con rutas inseguras, separadores, componentes
  ascendentes o caracteres reservados se rechazan y vuelven a `AUTO`.
- Las carpetas se indexan una vez al actualizar la biblioteca, evitando volver
  a recorrer una colección completa de media por cada miniatura.
- Al cambiar la fuente se invalidan únicamente las miniaturas del explorador;
  la página visible se recarga al instante y el resto continúa siendo lazy.

### Configuración, paquetes y pruebas

- `config/ngneon.example.conf`, README, AGENTS y ambos empaquetadores incluyen
  la nueva clave `rom_browser_media=auto`.
- Los paquetes nuevos usan `media_dir=auto`, preparan `roms/media/` y eliminan
  la antigua carpeta portátil `media/` para evitar dos ubicaciones ambiguas.
- Añadidas pruebas de detección dinámica, exclusión de vídeos y media no
  coincidente, resolución sin distinguir mayúsculas, rechazo de subdirectorios
  inseguros, prioridad `AUTO` y selección explícita de una fuente.
- Validado con la biblioteca local: se reconocieron 16 categorías de imágenes
  con media coincidente para las ROMs instaladas.
- Generada y verificada la distribución independiente
  `NGNEON-EMU-Windows-rom-browser-media-test`, sin ROMs ni saves y con
  `roms/media/` preparado.

---

## 2026-07-21 — Frontend, mandos, audio, vídeo y pantalla completa

### Retorno seguro al frontend

- Corregido el relanzamiento involuntario del último juego al regresar a la
  biblioteca. SDL seguía acumulando eventos mientras el proceso hijo del
  emulador tenía el control; ahora esa cola se bombea y se descartan botones,
  ejes y teclas obsoletos antes de volver a aceptar acciones.
- Un evento `Quit` recibido durante la partida no se pierde al vaciar la cola:
  se conserva como petición real de cierre del frontend.
- Después de cerrar el juego se vuelven a abrir los mandos activos, se limpian
  sus estados analógicos, se restaura la ventana y se reanuda la música del
  frontend sin reutilizar entradas de la sesión anterior.

### Asignación y navegación con gamepad

- La asignación automática de mandos sigue ahora estrictamente el orden de
  conexión: el primer gamepad USB es P1, el segundo P2, el tercero P3 y el
  cuarto P4. Las preferencias antiguas guardadas por GUID ya no pueden hacer
  que el primer dispositivo aparezca directamente como Player 2.
- La misma regla se aplica tanto a los mandos presentes al arrancar como a los
  conectados mediante hotplug.
- La asignación elegida se comunica también a SDL mediante `player_index`, y
  la reasignación manual intercambia jugadores sin duplicarlos.
- El stick analógico izquierdo controla la biblioteca y los ajustes del
  frontend. Cada cruce de la zona muerta produce un solo movimiento y exige
  volver al centro o cambiar de dirección, evitando navegación repetida por
  drift.
- Al activar el gamepad del frontend se oculta el cursor; al desactivarlo se
  muestra de nuevo. El cambio se aplica en caliente junto con la reapertura de
  controladores y la limpieza de estados del stick.
- Añadidas pruebas para el orden automático P1–P4 y para el comportamiento por
  flancos del stick analógico.

### Audio Z80/YM2610 en sesiones largas

- Corregido el cálculo de T-states consumidos por cada instrucción Z80 cuando
  el contador circular de `z80emu` cruza de `i32::MAX` a `i32::MIN`, alrededor
  de los nueve minutos de ejecución continua.
- El desbordamiento ya no se interpreta como un bloque de más de dos mil
  millones de ciclos: se evita el tirón de varios segundos y la pérdida
  intermitente de música observados durante partidas largas de Metal Slug 5.
- Añadido un límite defensivo en el mezclador para impedir reservas de audio
  descontroladas si vuelve a recibirse un bloque temporal imposible.
- Corregida una segunda causa independiente de pérdida de música: el pulso NMI
  pendiente del Z80 ya no se borra antes de saber si `z80emu` lo aceptó.
  Inmediatamente después de `EI` o mientras existe un prefijo DD/FD pendiente,
  el núcleo puede aplazar la NMI; NGNEON mantiene ahora el comando latched hasta
  que el Z80 entra realmente en el handler.
- Esto evita perder un comando musical de una sola vez mientras los comandos
  posteriores de efectos de sonido continúan funcionando, síntoma observado
  en Metal Slug 5.
- Añadidas regresiones para el wrap firmado del contador y para una NMI
  aplazada por un límite de prefijo Z80.
- La auditoría de 32.000 frames conserva la cadencia PCM de Geolith y una sonda
  adicional de 40.000 frames —más de once minutos emulados— termina sin fallos
  CPU ni desincronización del planificador Z80/YM2610.

### Franja negra horizontal en Samurai Shodown

- Corregida la banda negra horizontal persistente que podía aparecer de lado
  a lado de la pantalla después de un watchdog/reset.
- La causa era el borrado de los dos buffers de línea de sprites LSPC al
  terminar cada frame del planificador host. Cuando la fase LSPC dejaba de
  coincidir con ese límite, ya estaban precalculadas dos líneas del siguiente
  tramo y se eliminaban antes de presentarlas.
- Los buffers `linebuf`/`lbactive` se preservan ahora entre frames host, igual
  que en Geolith; el framebuffer visible sí continúa limpiándose normalmente.
- Añadida la prueba
  `host_frame_boundary_preserves_precalculated_lspc_sprite_lines`, que fuerza
  el desfase y comprueba que las líneas preparadas sobreviven al cambio de
  frame.
- Verificado en el attract mode de Samurai Shodown durante 2.300 frames: ya no
  reaparece el hueco persistente de dos líneas observado en la captura original.

### Pantalla completa persistente del frontend nativo

- Añadida la preferencia `frontend_fullscreen=on/off`, independiente de
  `fullscreen`, que sigue controlando exclusivamente la ventana del emulador.
- La biblioteca puede arrancar directamente a pantalla completa, cambiarse
  desde Ajustes o alternarse con `F11`; todas las rutas guardan la misma clave.
- Los textos del menú distinguen ahora claramente «Emulador pantalla completa»
  y «Frontend pantalla completa».
- Si SDL rechaza el cambio de modo, el estado y la configuración vuelven al
  valor realmente aplicado y se muestra un error, evitando guardar una opción
  que la ventana no pudo cumplir.
- Actualizadas las pruebas de carga de configuración, índices y scroll de
  ajustes, persistencia y rollback completo ante errores de escritura.

### Validación y distribuciones

- `cargo fmt --all -- --check`, Clippy estricto para todo el workspace, la
  suite completa y `cargo build --release --workspace` terminan correctamente.
- Grupos principales tras los últimos cambios: 372 pruebas del núcleo, 112 de
  la biblioteca frontend superadas —una prueba opcional de media local
  ignorada— y 38 del binario `ngneon-emu`.
- Generadas y verificadas las distribuciones de prueba
  `NGNEON-EMU-Windows-z80-audio-fix`,
  `NGNEON-EMU-Windows-audio-video-fullscreen-test` y
  `NGNEON-EMU-Windows-rom-browser-media-test`.

---

## 2026-07-20 — Paridad de hardware AES/MVS con Geolith

### Temporización, audio y estados

- La frecuencia AES del YM2610 se calcula ahora desde el reloj maestro real
  (`24,167829 MHz / 6 / 72`) en lugar de usar una aproximación.
- El save state pasa a la versión 14 y conserva el resto temporal del YM2610 y
  el registro `POUTPUT`; los estados de versiones anteriores siguen cargando.
- Al cargar un estado se vacían el ring buffer y el resampler, se inserta el
  preroll y se reinicia el planificador A/V para evitar audio antiguo y saltos
  de sincronización.
- Separados el reset frío y el reset normal del Z80: cargar otro cartucho
  reinicializa sus bancos, mientras que reset, watchdog y cambio de BIOS los
  conservan como Geolith.
- Añadido un controlador de ocupación del audio en el frontend con objetivo de
  4096 muestras intercaladas, EWMA de 0,05 y corrección de pacing limitada a
  ±0,5 %. El núcleo continúa siendo determinista y el audio solo ajusta el
  ritmo de presentación.
- Ampliado `NGNEON_AUDIO_DIAG` con ocupación filtrada, corrección aplicada y
  contadores de underrun/overrun.

### Cartridge RAM y hardware de entrada

- La cartridge RAM Brezzasoft de 8 KiB se guarda por juego en
  `saves/<juego>.cartram`, con detección de cambios, aislamiento entre
  cartuchos y rechazo seguro de archivos truncados.
- Se permite importar un `.sav` de Geolith únicamente cuando mide exactamente
  8192 bytes, evitando confundirlo con la backup RAM de 64 KiB de NGNEON.
- Añadidos `PlayerId` y entradas independientes para P1–P4; la API anterior
  `set_input()` se conserva como wrapper compatible para P1.
- Implementados `POUTPUT`, su espejo AES y la multiplexación FTC1B usada por
  los mandos P3/P4.
- Añadida la opción `four_player=off/on` a la configuración y a ambos menús.
  Permanece desactivada por defecto para conservar el comportamiento clásico.
- Irritating Maze dispone de trackball X/Y seleccionable por `POUTPUT`, ratón
  relativo, stick analógico, cuatro botones y Start en sus puertos correctos.
- V-Liner implementa el puerto especial `0x280000`, STATUS-A y las entradas
  Coin 1/2, Operator Menu, Clear Credit y Hopper Out.
- Las entradas auxiliares y de trackball usan APIs separadas de los botones
  normales de los jugadores.

### Teclado y mandos

- El teclado configurable admite P1 y P2. P1 conserva flechas, ZXCV, Enter y
  Espacio; P2 usa WASD, FGHJ, T y R.
- El formato nuevo usa claves `P1.Up=...`, `P2.Up=...` y `Aux...`; los archivos
  antiguos sin prefijo siguen interpretándose como controles de P1.
- El overlay permite alternar entre P1, P2 y acciones especiales y evita que
  una misma tecla pertenezca a varias acciones globales.
- Hasta cuatro mandos pueden asignarse a P1–P4 con preferencia persistente por
  GUID, resolución determinista de dispositivos idénticos y compatibilidad con
  los nombres de configuración antiguos.
- Al desconectar un mando se liberan sus botones y ejes retenidos. Los mandos
  adicionales sin jugador conservan los atajos globales.

### Vídeo, validación y distribución

- Eliminado el filtrado específico por NGH de Zed Blade; ya no existe ninguna
  rama de renderizado condicionada por el cartucho `0x076`.
- La salida visible de Zed Blade coincide exactamente con Geolith en los frames
  120, 300 y 600 auditados, tanto en número de píxeles activos como en hash de
  framebuffer; ambas implementaciones produjeron además la misma cantidad de
  pares PCM durante 600 frames.
- Las pruebas largas AES y MVS de 3600 frames mantienen el FIFO acotado, sin
  pérdida acumulativa, starvation, underruns ni overruns.
- Verificados con ROM real el arranque y metadata de Irritating Maze y Jockey
  Grand Prix; POUTPUT, FTC1B, V-Liner y bancos Z80 tienen además pruebas
  sintéticas deterministas.
- La referencia auditada queda fijada en Geolith libretro `c5b57a6b31b7abef`
  y Geolith upstream `4c0db390fdda159b`; hashes y resultados se documentan en
  `docs/hardware-parity-geolith.md`.
- El empaquetador Windows admite un directorio de salida opcional sin cambiar
  su comportamiento por defecto.
- Los empaquetadores Windows y Linux crean `musica/` con la guía de música del
  frontend, comprueban su existencia y protegen los MP3 existentes antes de
  limpiar una salida.
- Añadida `BUILDING.md` con procedimientos nativos para Windows y Linux x64 y
  ARM64, requisitos por arquitectura, verificación de binarios y límites de la
  compilación cruzada; los paquetes futuros incluyen también esta guía.
- Generada y comprobada la distribución independiente
  `NGNEON-EMU-Windows-hardware-sync` y su ZIP, sin ROMs, partidas ni
  credenciales del usuario.
- Validación final: 367 pruebas del núcleo, 110 de la biblioteca frontend y 37
  del binario superadas; `cargo fmt`, Clippy estricto y build release correctos.

---

## 2026-07-18 — Scripts nativos para Linux

### Compilación y distribución

- Añadidos `iniciar-ngneon.sh`, `configurar-ngneon.sh` y
  `package_ngneon_release.sh` como equivalentes Linux de los tres scripts de
  Windows.
- El launcher localiza el binario release y compila automáticamente
  `ngneon-frontend` si todavía no existe.
- El empaquetador genera `NGNEON-EMU-linux-<arquitectura>/` con ejecutables
  nativos, configuración portátil, temas, estructura `roms/media/` y
  documentación de dependencias.
- La protección del empaquetador detecta ROMs, media, música, partidas,
  capturas, logs y configuración personalizada antes de limpiar una salida.
- Documentadas las dependencias y órdenes de instalación para Debian/Ubuntu,
  Fedora y Arch Linux.

## 2026-07-17 — Temas completos, media configurable y música del frontend

### Frontend nativo

- Añadidos los layouts XML v2 `Cyber List` y `Console Showcase`, diferentes al
  diseño clásico y construidos con componentes transformables.
- El parser admite 16 componentes funcionales, hasta 32 slots de media dinámica
  y 15 fuentes semánticas `game.*` para vídeo, fanart, capturas, títulos,
  mixes, marquees, wheels/logos, cajas y cartuchos.
- Los slots media soportan prioridad/fallback, `contain`, `cover`, `stretch`,
  opacidad, posición, tamaño y orden `z`.
- `media_dir=auto` resuelve la biblioteca como `<rom_path>/gamelist.xml` y
  `<rom_path>/media`; también se puede elegir una ruta global personalizada
  desde Ajustes o con `F4`.
- El navegador interno y el frontend nativo comparten la misma interpretación
  de `media_dir`; el navegador reconoce cajas en `box2d`, `box3d`, `mix` y
  `screenshots`, además del formato plano anterior.
- La carpeta `musica/` reproduce MP3 aleatoriamente sin repetir hasta completar
  la vuelta. Ajustes permite activarla o desactivarla y `Ctrl+N` cambia de pista.
- La música se pausa al lanzar un juego y se reanuda al cerrar el emulador.
- Los mensajes correctos de configuración y recarga de tema desaparecen
  automáticamente después de unos segundos.
- `themes/README.md` documenta todos los componentes, fuentes, atributos,
  subdirectorios, fallbacks, límites y recetas; la plantilla XML se valida
  automáticamente con el parser real.

## 2026-07-12 — Planificador Geolith y launcher de sesiones

### Emulación

- Sustituido el reparto del frame en 264 bloques por el orden de ejecución de Geolith: una instrucción 68K completa, watchdog/RTC/IRQ2, LSPC y sincronización Z80/YM2610.
- Los eventos LSPC cruzan los ciclos 29, 573 y 712 conservando la fase entre frames.
- La fase de ciclo LSPC se guarda en save states versión 11 y resincroniza el reloj maestro al cargar.
- La auditoría estructural ZIP frente a `.neo` compara también el offset ADPCM-B: 146 de 153 referencias son idénticas; las diferencias restantes son buffers M1 CMC expandidos o bancos ausentes en el ZIP fuente.

### Frontend nativo

- Añadido `ngneon-frontend.exe`, una biblioteca SDL2 de escritorio independiente del proceso de emulación.
- Vista tipo wheel con media de `gamelist.xml` y fallback por nombre en `media/` para PNG, JPEG y WebP.
- Configuración nativa de BIOS, región, vídeo, autoestado, gamepad y hacks compatibles con Geolith.
- Modos de arranque Normal, Sesión limpia y Diagnóstico.
- Sesión limpia y Diagnóstico aíslan SRAM/memory card del almacenamiento del host sin borrar los saves existentes.
- Todos los modos generan un manifiesto reproducible sin credenciales en `logs/launch-*.log`.
- Cada juego se abre en un proceso `ngneon-emu.exe` separado; al cerrarlo se vuelve al frontend.
- El modo diagnóstico escribe progreso CPU, LSPC, IRQ2, watchdog y audio en `logs/session-*.log`.
- Eliminados Edge, Chrome, Node, el servidor local y el configurador web del flujo y del paquete de release.
- El empaquetador protege ROMs, media, saves, capturas, logs y configuración existente; ya no los limpia sin autorización explícita.

Las pruebas automatizadas validan contratos internos y estructura de ROM; la validación de jugabilidad se realiza manualmente por el usuario.

---

## 2026-07-05 — Preparación RetroAchievements Hardcore

- El cambio Casual → Hardcore aplica un reset completo antes del siguiente frame.
- UniBIOS queda bloqueada en Hardcore y se sustituye por una BIOS oficial disponible.
- La carga de save states sigue bloqueada en el núcleo, pero ahora se permite crearlos para depuración junto con el progreso de `rcheevos`.
- Dumps, overlay de depuración y hacks Geolith se desactivan en Hardcore.
- El título de ventana y la notificación de carga indican `RA:HARDCORE` o `RA:CASUAL`.
- Añadida la matriz de trabajo pendiente en `HARDCORE_COMPLIANCE.md`.

## 2026-07-05 — Paridad adicional con Geolith y persistencia segura

### Opciones avanzadas de emulación

- Añadido overclock opcional del 68K a 24 MHz, equivalente a `geolith_oc`.
  - El vídeo, Z80, YM2610, RTC e IRQ2 conservan la cadencia de hardware.
  - Metal Slug 2 mantiene exactamente la misma duración de audio con o sin overclock.
- Límite de sprites configurable: 96 hardware, 192, 288 o 381.
- Opción para desactivar ADPCM accumulator wrap, útil para Ganryu y Nightmare in the Dark.
- Los hacks se desactivan y bloquean automáticamente en RetroAchievements Hardcore.

### Memory Card y saves

- Tarjeta de memoria insertada por defecto, como Geolith.
- Inserción y protección de escritura configurables desde SYSTEM.
- La tarjeta es ahora compartida entre cartuchos en `saves/memory-card.mem`.
- Compatibilidad de migración con antiguos saves `<juego>.mem`.
- Corregida contaminación de backup RAM entre juegos al cambiar de cartucho.
- El cartucho anterior se guarda antes de sustituir su ruta y la SRAM del nuevo juego empieza limpia.

### Auditoría

- FBNeo actualizado contrastado hasta `3c489fec`.
- Geolith actualizado contrastado hasta `c5b57a6b`.
- Loader ZIP: 160/160 sets cargados.
- Ganryu, Metal Slug 2, Nightmare in the Dark y su bootleg pasan 1.800 frames con cadencia de audio exacta.
- 413 pruebas superadas, Clippy estricto sin avisos y build release correcto.

---

## 2026-07-04 — Rendimiento de sprites, audio estable y aviso RetroAchievements

### Vídeo y rendimiento

- El render scanline decodifica únicamente la fila visible de 16 píxeles de cada tile de sprite.
- Se elimina la decodificación repetida de las 16 filas completas por cada sprite y scanline.
- Metal Slug 2 mejora aproximadamente un 30% en la prueba de 1.200 frames:
  - antes: ~10,7 s;
  - después: ~7,6 s;
  - coste final: ~6,38 ms/frame frente al presupuesto MVS de 16,896 ms.
- Los hashes de vídeo y audio permanecen idénticos, por lo que la optimización no altera la emulación.

### Audio

- El temporizador BUSY del YM2610 vuelve a acumular escrituras como el backend YMFM de Geolith.
- SDL inicia después de cargar la ROM y con un callback estéreo de preroll silencioso.
- Los resets y cambios de ROM limpian y vuelven a cebar el búfer para evitar carreras y cortes.
- Auditorías de 3.600 frames en Metal Slug 2 y Shock Troopers:
  - cadencia exacta;
  - cero starvation del resampler;
  - cero overruns;
  - cero underruns.

### RetroAchievements

- El aviso especial `101000001` («Warning: Unknown Emulator») se clasifica como validación del cliente y deja de tratarse como un logro del juego.
- Ya no suma progreso, puntos ni genera un popup de logro falso.
- Se conserva el User-Agent versionado `NGNEON-EMU/1.2.3` y el flujo normal de logros reales.
- La aprobación de hardcore sigue dependiendo de la validación oficial de RetroAchievements para cada cliente/release.

### Calidad

- `cargo clippy --workspace --all-targets -- -D warnings` sin avisos.
- `cargo test --workspace`: 408 pruebas superadas.
- `cargo build --release --workspace` correcto.

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
