# Temas del frontend nativo

Esta es la referencia completa para crear temas XML de `ngneon-frontend`.
Un tema puede cambiar la paleta, mover y redimensionar los bloques funcionales,
crear layouts diferentes, añadir imágenes estáticas y mostrar la media del juego
seleccionado: vídeos, marquees, logos, capturas, cajas, cartuchos y fanart.

## Inicio rápido

1. Copia `ThemeTemplate/` como `themes/MiTema/`; el nombre de la carpeta debe
   describir el tema sin espacios.
2. Renombra `theme.xml.example` a `theme.xml` y crea `assets/` para imágenes,
   fuentes u otros recursos propios del diseño.
3. Cambia `id`, `name`, paleta, componentes y slots `<media>`.
4. Abre **Ajustes → Tema frontend** y selecciona el tema.
5. Pulsa `F6` después de editar el XML o sus imágenes.

No es necesario recompilar. La recarga es segura: si el XML nuevo no es válido,
el frontend conserva la última versión funcional. Si un tema falla durante el
arranque, se utiliza el tema Neon integrado y se muestra un aviso.

## Configuración y rutas

Las claves relacionadas se guardan en `config/ngneon.conf`:

```ini
frontend_theme_dir=themes
frontend_theme=themes/MiTema/theme.xml
rom_path=roms
media_dir=auto
frontend_music=on
```

- `frontend_theme_dir`: carpeta donde se descubren `*.xml` en su raíz y a un
  nivel de profundidad (`NombreSinEspacios/theme.xml`). Acepta una ruta relativa
  a la raíz de NGNEON-EMU o una ruta absoluta.
- `frontend_theme`: selector persistente. Admite `builtin`, el `id`, el nombre,
  el nombre del fichero o la ruta del XML.
- `rom_path`: directorio de ROMs. Aquí se busca primero `gamelist.xml`.
- `media_dir=auto`: usa `<rom_path>/media`.
- `media_dir=<ruta>`: utiliza otro directorio global de media, relativo a la
  raíz de NGNEON-EMU o absoluto.
- `frontend_music`: activa o desactiva los MP3 de `musica/`; no forma parte del
  XML del tema.

Atajos del frontend:

| Acción | Tecla |
|---|---|
| Abrir Ajustes | `S` o `F2` |
| Elegir carpeta de ROMs | `F3` |
| Elegir directorio global de media | `F4` |
| Reescanear biblioteca | `F5` |
| Recargar XML y assets del tema | `F6` |
| Siguiente canción | `Ctrl+N` |
| Pantalla completa | `F11` |

En **Ajustes → Directorio de media**, derecha o `Enter` abre el selector e
izquierda restaura el modo automático `<rom_path>/media`.

## Organización recomendada

Con `rom_path=roms` y `media_dir=auto`:

```text
roms/
├── gamelist.xml
├── Neo-Geo.dat
├── mslug3.neo
├── kof98.zip
└── media/
    ├── videos/
    │   ├── mslug3.mp4
    │   └── kof98.mp4
    ├── arcade-marquee/
    ├── screen-marquees/
    ├── marquees/              # compatibilidad heredada
    │   ├── mslug3.png
    │   └── kof98.png
    ├── logos/
    ├── wheelhd/
    ├── screenshots/
    ├── titles/
    ├── fanart/
    ├── mix/
    ├── box2d/
    ├── box3d/
    ├── cartridges/
    └── manuals/
```

Para la búsqueda convencional, el fichero de media debe usar el mismo nombre
base que la ROM: `mslug3.neo` → `mslug3.png`, `mslug3.mp4`, etc.

## `gamelist.xml`

Las rutas locales declaradas por el catálogo tienen prioridad. Si una ruta
declarada falta, el frontend busca por nombre de ROM en `media_dir` y, como
último fallback, en una carpeta `media` junto a `gamelist.xml`.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<gameList>
  <game>
    <path>./mslug3.neo</path>
    <name>Metal Slug 3</name>
    <desc>Descripción del juego.</desc>
    <developer>SNK</developer>
    <publisher>SNK</publisher>
    <players>1-2</players>
    <video>./media/videos/mslug3.mp4</video>
    <marquee>./media/marquees/mslug3.png</marquee>
    <screenshot>./media/screenshots/mslug3.png</screenshot>
    <box2d>./media/box2d/mslug3.png</box2d>
    <cartridge2d>./media/cartridges/mslug3.png</cartridge2d>
  </game>
</gameList>
```

Se aceptan rutas relativas a `gamelist.xml` y rutas absolutas locales. Las URL
se ignoran. Las etiquetas de media reconocidas son `image`, `thumbnail`,
`marquee`, `video`, `manual`, `fanart`, `titleshot`, `boxart`, `cartridge`,
`screenshot`, `mix`, `wheel`, `box2d`, `box3d` y `cartridge2d`.

`manual` se cataloga para uso futuro, pero no existe `game.manual`: un PDF no
se puede dibujar actualmente dentro de un slot `<media>`.

## Estructura del XML de tema

El elemento raíz debe ser `<theme>` y toda la configuración se expresa mediante
atributos. No se admite texto libre dentro de los elementos.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<theme version="2" id="mi-tema" name="Mi tema" author="Autor">
  <canvas width="960" height="540"/>
  <palette background="#05070B" primary="#EA3357" text="#F5F7FB"/>
</theme>
```

### Atributos de `<theme>`

| Atributo | Obligatorio | Uso |
|---|---:|---|
| `version` | Sí | `1` para compatibilidad o `2` para usar `<media>` |
| `id` | No | ID único; si falta se usa el nombre del fichero |
| `name` | No | Nombre visible en Ajustes; si falta se usa el ID |
| `author` | No | Autor mostrado al recargar el tema |

Los IDs admiten entre 1 y 64 caracteres ASCII: letras, números, guion y guion
bajo. Dos temas no pueden compartir el mismo ID, sin distinguir mayúsculas.

## Canvas y coordenadas

```xml
<canvas width="1920" height="1080"/>
```

El framebuffer real siempre es 960×540. El canvas sólo define el sistema de
coordenadas del XML; el parser normaliza posiciones y tamaños a 960×540. Así,
un tema puede escribirse cómodamente en 960×540, 1280×720 o 1920×1080.

- `width` y `height` son obligatorios, mayores que cero y como máximo 8192.
- `x` e `y` pueden ser negativos para colocar contenido parcialmente fuera.
- Los tamaños finales de componentes, capas y media deben ser positivos.

## Paleta

Los colores aceptan `#RRGGBB`, `#AARRGGBB`, `0xRRGGBB` o `0xAARRGGBB`.
Cuando no se define un color, se hereda el valor del tema Neon integrado.

```xml
<palette
  background="#05070B"
  panel="#E00B111B"
  primary="#EA3357"
  text="#F5F7FB"/>
```

También se admite la forma larga:

```xml
<palette>
  <color id="background" value="#05070B"/>
  <color id="accent" value="#22D3EE"/>
</palette>
```

| Color | Uso principal | Alias aceptados |
|---|---|---|
| `background` | Fondo base | `bg` |
| `header` | Cabecera y pie | — |
| `panel` | Panel principal | — |
| `panel_alt` | Panel secundario | — |
| `primary` | Selección y acción principal | `red` |
| `primary_dark` | Selección oscura | `red_dark` |
| `secondary` | Acento secundario | `blue` |
| `accent` | Bordes y detalles | `cyan` |
| `warning` | Avisos y selección destacada | `gold` |
| `success` | Estados correctos | `green` |
| `text` | Texto principal | — |
| `text_soft` | Texto secundario | — |
| `text_dim` | Texto atenuado y fallbacks | — |
| `border` | Marcos y separadores | — |
| `error` | Errores | — |

## Componentes funcionales

### Fuente propia del tema

Los temas v2 pueden incluir una fuente TrueType/OpenType dentro de su carpeta:

```xml
<font source="assets/Rajdhani-SemiBold.ttf"/>
```

Solo se admite un elemento `<font>`. La ruta debe ser local, relativa al XML y
no puede contener `..`. Si el archivo no se puede cargar, el frontend muestra
un aviso y conserva la fuente del sistema. Los componentes de texto que
declaren `font_size` usan esta fuente; el resto mantiene el tamaño histórico.

Un `<component>` mueve, redimensiona, oculta o cambia de capa un bloque
funcional completo. El contenido se dibuja primero en su geometría nativa y
después se transforma al rectángulo indicado. Los hitboxes del ratón usan la
misma transformación.

```xml
<component id="carousel"
           x="20" y="70" width="188" height="406"
           z="100" visible="true"/>
```

Todos sus atributos salvo `id` son opcionales. Los omitidos conservan el valor
integrado:

| Atributo | Valores |
|---|---|
| `id` | Uno de los 20 IDs de la tabla siguiente |
| `x`, `y` | Enteros; admiten negativos |
| `width`, `height` | Enteros positivos |
| `z` | `-10000..10000`; un valor mayor queda delante |
| `visible` | `true/false`, `on/off`, `yes/no` o `1/0` |
| `artwork` | `logo` o `box3d`; afecta a listas y gamecarousels |
| `chrome` | `panel` (predeterminado) o `none`; elimina fondo, borde, banda de selección y flecha |
| `opacity` | `0..255` o `0.0..1.0`; opacidad global del componente |
| `selected_scale` | `100..200`, porcentaje (`145%`) o factor decimal (`1.45`); sólo gamecarousels |
| `unselected_opacity` | `0..255` o `0.0..1.0`; atenuación de logos no seleccionados |
| `arc` | `0..960`; curvatura horizontal en píxeles lógicos del gamecarousel |
| `font_size` | `6..160`; tamaño tipográfico para los componentes de texto |
| `show_with_logo` | En `game_title`, permite mostrar el nombre aunque exista logo |
| `fields` | En `game_metadata`: lista de `year`, `developer`, `publisher`, `players` o `manufacturer` |
| Efectos/animación | Los atributos comunes de las secciones «Efectos de imagen» y «Animaciones» |
| Estilo de texto | Los atributos `text_*`, tracking, interlineado y autoajuste descritos más abajo |

### Referencia de componentes

| ID | Contenido | Vista/condición | Rectángulo y z por defecto | Visible |
|---|---|---|---|---:|
| `library_background` | Fondo integrado de biblioteca | Biblioteca | `0,0,960,540`, z `0` | Sí |
| `settings_background` | Fondo integrado de Ajustes | Ajustes | `0,0,960,540`, z `0` | Sí |
| `header` | Logo NGNEON y botones Biblioteca/Ajustes | Ambas vistas | `0,0,960,56`, z `200` | Sí |
| `carousel` | Lista vertical clásica de 5 juegos | Biblioteca con juegos | `16,66,194,414`, z `100` | Sí |
| `cabinet` | Cabinet, logo y preview clásicos | Biblioteca con juegos | `216,66,390,414`, z `110` | Sí |
| `game_info` | Título, metadatos, descripción y caja | Biblioteca con juegos | `610,66,334,414`, z `120` | Sí |
| `library_footer` | Acciones Jugar, Ajustes, Media, Tema, etc. | Biblioteca | `0,492,960,48`, z `200` | Sí |
| `empty_library` | Mensaje y selector cuando no hay ROMs | Biblioteca vacía | `128,116,704,302`, z `120` | Sí |
| `settings` | Las 27 opciones y botones inferiores | Ajustes | `0,56,960,484`, z `120` | Sí |
| `status` | Mensajes de éxito y error | Ambas vistas | `0,56,960,52`, z `300` | Sí |
| `vertical_list` | Lista vertical alternativa de 7 juegos | Biblioteca con juegos | `24,42,438,476`, z `130` | No |
| `horizontal_list` | Carrusel horizontal alternativo de 7 juegos | Biblioteca con juegos | `0,404,960,136`, z `130` | No |
| `gamecarousel_left` | Carrusel vertical curvado hacia el centro desde la izquierda | Biblioteca con juegos | `20,48,300,444`, z `130` | No |
| `gamecarousel_right` | Carrusel vertical curvado hacia el centro desde la derecha | Biblioteca con juegos | `640,48,300,444`, z `130` | No |
| `gamecarousel_horizontal` | Carrusel horizontal de cinco cajas 3D grandes | Biblioteca con juegos | `0,190,960,350`, z `130` | No |
| `game_title` | Título grande del juego | Biblioteca con juegos | `514,18,420,92`, z `130` | No |
| `game_description` | Descripción multilínea | Biblioteca con juegos | `510,390,426,128`, z `130` | No |
| `game_metadata` | Año, estudio y jugadores | Biblioteca con juegos | `510,474,426,44`, z `130` | No |
| `game_credits` | Desarrollador en la esquina inferior izquierda y publisher en la derecha | Biblioteca con juegos | `20,496,920,36`, z `130` | No |
| `page_counter` | Posición actual y total de juegos | Biblioteca con juegos | `24,12,260,24`, z `130` | No |

`carousel`, `vertical_list`, `horizontal_list`, `gamecarousel_left`,
`gamecarousel_right` y `gamecarousel_horizontal` se pueden pulsar para
seleccionar un juego y admiten doble
clic para lanzarlo. `artwork="logo"` usa logo, wheel y marquee como fallbacks y
muestra siete elementos. `artwork="box3d"` usa caja 3D, caja 2D, box art y
miniatura como fallbacks; las listas verticales muestran cinco elementos más
grandes con su título. `gamecarousel_horizontal` muestra cinco cajas, agranda
la seleccionada con `selected_scale`, atenúa las demás con
`unselected_opacity` y la dibuja al final para que quede por encima. Al activar
uno de los layouts alternativos normalmente se
ocultan `carousel`, `cabinet`, `game_info`, `library_background`, `header` o
`library_footer`, según el diseño buscado.

Los dos `gamecarousel_*` son imágenes especulares: el izquierdo curva los logos
hacia la derecha y el derecho hacia la izquierda. El seleccionado se dibuja al
final y por encima de sus vecinos, usando `selected_scale`; los demás usan
`unselected_opacity`. `arc="0"` los alinea y un valor mayor acentúa el arco:

```xml
<component id="gamecarousel_right"
           x="620" y="44" width="330" height="452"
           artwork="logo" chrome="none"
           selected_scale="145%"
           unselected_opacity="0.42" arc="72"/>
```

Ejemplo de carrusel horizontal con título y metadatos personalizados:

```xml
<font source="assets/Rajdhani-SemiBold.ttf"/>
<component id="game_title" visible="true"
           font_size="69" show_with_logo="true"/>
<component id="game_credits" visible="true"
           x="20" y="496" width="920" height="36"
           font_size="20" chrome="none"/>
<component id="gamecarousel_horizontal" visible="true"
           artwork="box3d" chrome="none"
           selected_scale="145%" unselected_opacity="0.68"/>
```

`chrome="none"` convierte los componentes alternativos en contenido flotante:
las listas conservan el cambio de tamaño y opacidad para señalar la selección,
pero no dibujan tarjetas, panel exterior, bandas ni flecha. También se puede usar
en título, descripción, metadatos y contador para integrarlos directamente sobre
el fondo del tema. Si se omite, se mantiene la presentación enmarcada histórica.

`game_description` envuelve el texto completo. Cuando excede el recuadro, espera
2,5 segundos, avanza automáticamente línea a línea, conserva el final otros
2,5 segundos y vuelve al principio. El ciclo se reinicia al cambiar de juego o
al volver de Ajustes, por lo que una descripción larga nunca queda inaccesible.

Los componentes alternativos existen también en temas v1, pero están ocultos
por defecto. El elemento `<media>` sí requiere `version="2"`.

## Efectos visuales compartidos

`<component>`, `<layer>` y `<media>` aceptan el mismo conjunto de filtros.
Todos son opcionales y sus valores neutros reproducen exactamente el render
anterior. En un componente el filtro se aplica al bloque funcional completo
después de dibujarlo; en capas y media se procesa la imagen preparada.

```xml
<media id="backdrop" source="game.fanart"
       x="0" y="0" width="960" height="540" fit="cover"
       brightness="0.76" contrast="112%" saturation="0.85"
       gradient_start="#00000000" gradient_end="#B0000000"/>

<media id="logo" source="game.logo"
       x="24" y="20" width="300" height="110"
       shadow_color="#A0000000" shadow_x="5" shadow_y="7"
       shadow_blur="5" glow_color="#70FFD66B" glow_radius="7"/>
```

### Efectos de imagen

| Atributo | Predeterminado | Valores/resultado |
|---|---:|---|
| `brightness` | `100%` | Brillo `0..300%`; acepta `120`, `120%` o factor `1.20` |
| `contrast` | `100%` | Contraste `0..300%` |
| `saturation` | `100%` | Saturación `0..300%`; `0` deja la imagen monocroma |
| `grayscale` | `0` | Mezcla a gris `0..255` o `0.0..1.0` |
| `tint` | transparente | Tinte `#AARRGGBB`; el canal alpha controla su intensidad |
| `blur` | `0` | Desenfoque separable de radio `0..24` |
| `shadow_color` | `#A0000000` | Color/alpha de la sombra; declarar cualquier `shadow_*` la activa |
| `shadow_x`, `shadow_y` | `6`, `8` | Desplazamiento `-256..256` |
| `shadow_blur` | `6` | Suavizado `0..24` |
| `glow_color` | `#80FFFFFF` | Color/alpha del resplandor |
| `glow_radius` | `8` | Radio `0..24`; declarar cualquier `glow_*` lo activa |
| `rotation` | `0` | Rotación en grados enteros, `-180..180` |
| `rounded_corners` | `0` | Radio de esquina `0..512` |
| `gradient_start` | — | Color superior/izquierdo de la superposición |
| `gradient_end` | — | Color inferior/derecho; debe acompañar a `gradient_start` |
| `gradient_direction` | `vertical` | `vertical` u `horizontal` |

Los filtros siguen este orden: colorimetría/tinte, blur, rotación, recorte de
esquinas, gradiente y composición de sombra/glow detrás de la imagen. Rotación,
sombra y glow permanecen recortados por el rectángulo del elemento: deja margen
dentro del asset o del componente si quieres que el halo se vea completo.

Las capas estáticas y la media de imagen del juego se filtran una sola vez y
quedan en caché. Los efectos sobre un `<component>` se calculan después de
renderizar su contenido; evita `blur`, sombra o glow grandes en un componente
de pantalla completa que también tenga `pulse`. Los filtros de un slot de vídeo
se aplican a cada frame, así que para vídeo a pantalla completa conviene usar
brillo/contraste/saturación y reservar los radios altos para imágenes.

## Estilo de texto

Los componentes `game_title`, `game_description`, `game_metadata`,
`game_credits` y `page_counter` aceptan estilos tipográficos. `game_description`
usa también la fuente del tema cuando se declara cualquiera de estos atributos
o un `font_size`.

```xml
<component id="game_title" visible="true" chrome="none"
           x="330" y="18" width="600" height="90"
           font_size="69" text_color="#FFF7E4"
           text_align="center" vertical_align="center"
           text_shadow_color="#C0000000"
           text_shadow_x="3" text_shadow_y="5" text_shadow_blur="3"
           text_outline_color="#B0402008" text_outline="1"
           letter_spacing="2" auto_fit="true" min_font_size="30"/>
```

| Atributo | Predeterminado | Uso |
|---|---:|---|
| `text_color` | paleta del componente | Color `#RRGGBB` o `#AARRGGBB` |
| `text_align` | `auto` | `left`, `center`, `right` o `auto` |
| `vertical_align` | `auto` | `top`, `center`, `bottom` o `auto` |
| `text_shadow_color` | sombra histórica | Color/alpha; `#00000000` la desactiva explícitamente |
| `text_shadow_x`, `text_shadow_y` | `2`, `3` | Desplazamiento `-256..256` |
| `text_shadow_blur` | `2` | Suavizado visual `0..24` |
| `text_outline_color` | `#FF000000` | Color del contorno |
| `text_outline` | `0` | Grosor `0..8` |
| `letter_spacing` | `0` | Tracking entre caracteres, `-20..80` píxeles |
| `line_height` | `100%` | Interlineado `50..300%`; especialmente útil en descripción |
| `auto_fit` | `true` | Reduce el tamaño hasta caber horizontalmente |
| `min_font_size` | mínimo interno | Límite `6..160` para `auto_fit` |

`auto_fit="false"` conserva estrictamente `font_size` y deja que el rectángulo
recorte un texto demasiado ancho. En `game_credits` se conserva la semántica
especial: desarrollador a la izquierda y publisher a la derecha.

## Animaciones

Las animaciones se reinician al cambiar de juego, recargar/cambiar de tema o
entrar en otra vista. Se pueden combinar en el mismo componente, capa o media:

```xml
<media id="logo" source="game.logo"
       x="24" y="18" width="292" height="118"
       fade_in="500" slide_in="left" slide_distance="60"
       slide_duration="650" zoom_in="600" zoom_from="85%"
       pulse="2%" pulse_period="2200"/>
```

| Atributo | Predeterminado | Uso |
|---|---:|---|
| `fade_in` | `0` | Duración de entrada por opacidad, `0..10000` ms |
| `slide_in` | `none` | `left`, `right`, `up`, `down` o `none` |
| `slide_distance` | `64` | Distancia inicial `0..1920` píxeles |
| `slide_duration` | `450` | Duración `0..10000` ms |
| `zoom_in` | `0` | Duración del zoom de entrada, `0..10000` ms |
| `zoom_from` | `85%` | Escala inicial `10..100%`; acepta factor decimal |
| `pulse` | `0` | Amplitud `0..25%`; acepta `4`, `4%` o factor `1.04` |
| `pulse_period` | `1600` | Periodo en ms; mínimo `100` cuando pulse está activo |

`pulse` mantiene el frontend redibujándose de forma continua; úsalo en uno o
dos elementos destacados y con amplitudes pequeñas.

## Media dinámica del juego

Un slot `<media>` dibuja media perteneciente al juego seleccionado. Sólo aparece
en la biblioteca, cuando existe al menos un juego.

```xml
<media id="preview"
       source="game.video,game.screenshot,game.titleshot,game.mix"
       x="270" y="106" width="420" height="252"
       z="130" opacity="255" fit="contain" visible="true"/>
```

`source` es una lista de prioridad de izquierda a derecha. Se usa la primera
fuente cuyo fichero existe. Si la primera fuente válida es `game.video`, las
fuentes restantes se preparan como imagen de fallback mientras arranca el vídeo
o si el decoder lo rechaza.

### Atributos de `<media>`

| Atributo | Obligatorio | Predeterminado | Uso |
|---|---:|---|---|
| `id` | Sí | — | ID único del slot |
| `source` | Sí | — | Lista de fuentes `game.*` separadas por comas |
| `x`, `y` | No | `0` | Posición |
| `width`, `height` | Sí | — | Tamaño positivo |
| `z` | No | `50` | Capa `-10000..10000` |
| `opacity` | No | `255` | `0..255` o decimal `0.0..1.0` |
| `fit` | No | `contain` | `contain`, `cover` o `stretch` |
| `visible` | No | `true` | Muestra u oculta el slot |
| `chrome` | No | `panel` | `panel` dibuja fondo/borde; `none` deja sólo la media |
| Efectos/animación | No | neutros | Todos los atributos visuales compartidos |

No existe atributo `view`: todos los slots media pertenecen a la biblioteca.
Las fuentes duplicadas dentro del mismo `source` y los IDs repetidos no se
admiten.

### Modos `fit`

| Valor | Resultado |
|---|---|
| `contain` | Muestra la imagen completa, conserva proporción y deja bandas |
| `cover` | Conserva proporción, llena el rectángulo y recorta el sobrante |
| `stretch` | Llena exactamente el rectángulo, aunque deforme la imagen |

Para logos, marquees, cajas y cartuchos suele convenir `contain`. Para fanart y
vídeos de fondo suele convenir `cover`. Usa `stretch` sólo si el asset ya tiene
la proporción exacta del slot.

### IDs especiales de media

El ID puede ser cualquier identificador válido, pero tres nombres tienen una
presentación especial:

- `backdrop` o `background`: se dibuja siempre sin panel, borde ni texto
  `SIN MEDIA`; es apropiado para fanart a pantalla completa.
- `logo`: no muestra el texto `SIN MEDIA` cuando no hay asset.
- Cualquier otro ID: usa por defecto el panel y borde de la paleta y muestra
  `SIN MEDIA` si no encuentra contenido. `chrome="none"` desactiva las tres
  cosas para cualquier ID.

### Todas las fuentes `game.*`

Formatos de imagen convencionales: `.png`, `.jpg`, `.jpeg` y `.webp`.

| Fuente XML | Qué representa | Etiqueta de `gamelist.xml` | Búsqueda convencional / fallback |
|---|---|---|---|
| `game.video` | Vídeo preview | `video` | `videos/<rom>.mp4`, `.webm`, `.mkv`, `.avi` |
| `game.fanart` | Fondo o ilustración panorámica | `fanart` | `fanart/<rom>.<imagen>` |
| `game.screenshot` | Captura de gameplay | `screenshot` | `screenshots/<rom>.<imagen>` |
| `game.titleshot` | Pantalla de título | `titleshot` | `titles/<rom>.<imagen>` |
| `game.mix` | Composición scraper | `mix` | `mix/<rom>.<imagen>` |
| `game.image` | Imagen genérica | `image` | Fallback a `game.mix`, después `game.box3d` |
| `game.thumbnail` | Miniatura | `thumbnail` | Fallback a `game.box2d` |
| `game.marquee` | Marquee de la recreativa | `marquee` | `arcade-marquee/<rom>.<imagen>`, ruta declarada, `screen-marquees/<rom>.<imagen>`, luego `marquees/<rom>.<imagen>` |
| `game.boxart` | Caja genérica | `boxart` | Fallback a `game.box2d` |
| `game.cartridge` | Cartucho | `cartridge` | `cartridges/<rom>.<imagen>` |
| `game.wheel` | Wheel/logo transparente | `wheel` | `logos/<rom>.<imagen>`, luego `wheelhd/<rom>.<imagen>` |
| `game.box2d` | Caja frontal 2D | `box2d` | `box2d/<rom>.<imagen>` |
| `game.box3d` | Caja renderizada 3D | `box3d` | `box3d/<rom>.<imagen>` |
| `game.cartridge2d` | Cartucho 2D | `cartridge2d` | `cartridges/<rom>.<imagen>` |
| `game.logo` | Mejor logo disponible | — | Alias del wheel resuelto por `game.wheel` |

Aunque el catálogo reconoce varias extensiones de vídeo, el reproductor
integrado garantiza únicamente **MP4 con vídeo H.264**. Los previews se
decodifican en streaming, silenciados y en bucle. El cambio rápido de selección
usa un debounce de 300 ms y no precarga el vídeo completo.

### Recetas habituales

Marquee grande con fallback a logo:

```xml
<media id="logo"
       source="game.marquee,game.logo,game.wheel"
       x="280" y="8" width="400" height="92"
       z="160" fit="contain"/>
```

Preview de vídeo con fallback completo:

```xml
<media id="preview"
       source="game.video,game.screenshot,game.titleshot,game.mix,game.image"
       x="280" y="106" width="400" height="280"
       z="130" fit="cover"/>
```

Fanart a pantalla completa:

```xml
<media id="backdrop"
       source="game.fanart,game.mix,game.image"
       x="0" y="0" width="960" height="540"
       z="-100" opacity="0.35" fit="cover"/>
```

Caja y cartucho conservando proporciones:

```xml
<media id="box"
       source="game.box2d,game.box3d,game.boxart,game.thumbnail"
       x="40" y="100" width="200" height="300"
       z="130" fit="contain"/>

<media id="cartridge"
       source="game.cartridge2d,game.cartridge"
       x="720" y="170" width="196" height="132"
       z="130" fit="contain"/>
```

## Capas de imagen estáticas

Una `<layer>` muestra un asset propio del tema, no del juego:

```xml
<layer id="marco"
       source="assets/marco.webp"
       x="0" y="0" width="960" height="540"
       z="250" opacity="0.80" fit="contain"
       view="library" visible="true"/>
```

### Atributos de `<layer>`

| Atributo | Obligatorio | Predeterminado | Uso |
|---|---:|---|---|
| `id` | Sí | — | ID único |
| `source` | Sí | — | PNG, JPEG o WebP relativo al XML |
| `x`, `y` | No | `0` | Posición |
| `width`, `height` | Sí | — | Tamaño positivo |
| `z` | No | `50` | Capa `-10000..10000` |
| `opacity` | No | `255` | `0..255` o `0.0..1.0` |
| `fit` | No | `contain` | `contain`, `cover` o `stretch` |
| `view` | No | `all` | `library`, `settings` o `all` |
| `visible` | No | `true` | Muestra u oculta la capa |
| Efectos/animación | No | neutros | Todos los atributos visuales compartidos |

`view` también acepta `biblioteca`, `ajustes`, `common` y combinaciones
`library|settings` o `library,settings`.

`source` debe ser local y relativo al XML. No se permiten URL, rutas absolutas
ni `..`, de forma que un tema no pueda salir de su propio directorio:

```text
themes/
├── README.md
└── MiTema/
    ├── theme.xml
    └── assets/
        ├── fondo.webp
        ├── fuente.ttf
        └── marco.png
```

El XML y todos los recursos exclusivos del diseño deben permanecer en la misma
carpeta. El motor ya resuelve las rutas de `<layer>` respecto a `theme.xml`; una
carpeta `assets/` no se examina como catálogo de temas. Los XML planos del
formato anterior continúan descubriéndose y una configuración antigua como
`themes/neon.xml` selecciona el nuevo tema por su ID.

## Orden de capas

Todos los componentes, capas y slots media se ordenan por `z`:

- Un `z` menor se dibuja primero y queda detrás.
- Un `z` mayor se dibuja después y queda delante.
- Si comparten el mismo `z`, el orden es componente, capa estática y media.
- Es recomendable reservar valores negativos para fondos, `100..199` para
  contenido y `200+` para marcos u overlays.

Ejemplo:

```xml
<media id="backdrop" source="game.fanart" z="-100"
       x="0" y="0" width="960" height="540"/>
<component id="vertical_list" z="130" visible="true"/>
<layer id="frame" source="assets/frame.png" z="200"
       x="0" y="0" width="960" height="540"/>
<component id="status" z="300"/>
```

## Límites y validación

El parser rechaza atributos y elementos desconocidos para detectar errores de
escritura temprano.

| Límite | Valor |
|---|---:|
| Tamaño máximo del XML | 256 KiB |
| Temas XML descubiertos por carpeta | 64 |
| Canvas máximo | 8192×8192 |
| Capas estáticas por tema | 64 |
| Slots media por tema | 32 |
| Tamaño final por capa/slot/componente | 4096×4096 |
| Área máxima individual | 16 megapíxeles |
| Presupuesto total de capas | 16 megapíxeles |
| Presupuesto total de slots media | 16 megapíxeles |
| Valor absoluto máximo de `x`/`y` ya normalizado | 16384 |
| Rango de `z` | `-10000..10000` |
| Brillo/contraste/saturación máximos | `300%` |
| Radio máximo de blur/sombra/glow | `24` |
| Duración máxima por animación | `10000 ms` |
| Amplitud máxima de pulse | `25%` |

Un asset estático ausente genera un aviso y se omite sin inutilizar el resto del
tema. Un XML estructuralmente inválido, una fuente `game.*` desconocida, un ID
duplicado o un tamaño fuera de rango impiden aplicar esa versión.

## Temas incluidos

- `NGNEONNeon/theme.xml`: layout clásico con marquee dinámico sobre la cabinet.
- `AmberArcade/theme.xml`: variante arcade ámbar.
- `GreenPhosphor/theme.xml`: estética de monitor de fósforo.
- `CyberList/theme.xml`: lista vertical de siete logos, marquee, preview y ficha.
- `ConsoleShowcase/theme.xml`: caja, cartucho, marquee, preview y carrusel horizontal.
- `NeoGeoRedCabinet/theme.xml`: cabinet roja, marquee/monitor enmascarados por el marco y carrusel vertical flotante de siete logos, sin tarjetas.
- `NeoGeoGreenCabinet/theme.xml`: cabinet verde, carrusel flotante de siete logos con nombre como fallback, caja 3D sin marco en el hueco central y descripción auto-scroll integrada sobre el fondo.
- `NeoGeo29Cabinet/theme.xml`: cabinet Neo Geo 29 blanca, vídeo y marquee bajo el marco y `gamecarousel_right` de logos con escala, opacidad y arco configurables.
- `FanartHorizon/theme.xml`: fanart tratado con gradiente/colorimetría, logo con glow y entrada animada, nombre real con Rajdhani SemiBold, desarrollador/publisher en las esquinas inferiores y carrusel horizontal de cinco cajas 3D.
- `ThemeTemplate/theme.xml.example`: plantilla comentada para crear un tema nuevo.

Los XML incluidos son ejemplos funcionales y se pueden copiar y modificar.
