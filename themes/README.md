# Temas del frontend nativo

Esta es la referencia completa para crear temas XML de `ngneon-frontend`.
Un tema puede cambiar la paleta, mover y redimensionar los bloques funcionales,
crear layouts diferentes, añadir imágenes estáticas y mostrar la media del juego
seleccionado: vídeos, marquees, logos, capturas, cajas, cartuchos y fanart.

## Inicio rápido

1. Copia `theme-template.xml.example` como `themes/mi-tema.xml`.
2. Cambia `id`, `name`, paleta, componentes y slots `<media>`.
3. Abre **Ajustes → Tema frontend** y selecciona el tema.
4. Pulsa `F6` después de editar el XML o sus imágenes.

No es necesario recompilar. La recarga es segura: si el XML nuevo no es válido,
el frontend conserva la última versión funcional. Si un tema falla durante el
arranque, se utiliza el tema Neon integrado y se muestra un aviso.

## Configuración y rutas

Las claves relacionadas se guardan en `config/ngneon.conf`:

```ini
frontend_theme_dir=themes
frontend_theme=themes/mi-tema.xml
rom_path=roms
media_dir=auto
frontend_music=on
```

- `frontend_theme_dir`: carpeta donde se descubren los `*.xml`. Acepta una ruta
  relativa a la raíz de NGNEON-EMU o una ruta absoluta.
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
    ├── marquees/
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
| `id` | Uno de los 16 IDs de la tabla siguiente |
| `x`, `y` | Enteros; admiten negativos |
| `width`, `height` | Enteros positivos |
| `z` | `-10000..10000`; un valor mayor queda delante |
| `visible` | `true/false`, `on/off`, `yes/no` o `1/0` |

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
| `game_title` | Título grande del juego | Biblioteca con juegos | `514,18,420,92`, z `130` | No |
| `game_description` | Descripción multilínea | Biblioteca con juegos | `510,390,426,128`, z `130` | No |
| `game_metadata` | Año, estudio y jugadores | Biblioteca con juegos | `510,474,426,44`, z `130` | No |
| `page_counter` | Posición actual y total de juegos | Biblioteca con juegos | `24,12,260,24`, z `130` | No |

`carousel`, `vertical_list` y `horizontal_list` se pueden pulsar para seleccionar
un juego y admiten doble clic para lanzarlo. Al activar uno de los layouts
alternativos normalmente se ocultan `carousel`, `cabinet`, `game_info`,
`library_background`, `header` o `library_footer`, según el diseño buscado.

Los componentes alternativos existen también en temas v1, pero están ocultos
por defecto. El elemento `<media>` sí requiere `version="2"`.

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

- `backdrop` o `background`: se dibuja sin panel, borde ni texto `SIN MEDIA`;
  es apropiado para fanart a pantalla completa.
- `logo`: no muestra el texto `SIN MEDIA` cuando no hay asset.
- Cualquier otro ID: usa el panel y borde de la paleta y muestra `SIN MEDIA`
  si no encuentra contenido.

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
| `game.marquee` | Marquee de la recreativa | `marquee` | `marquees/<rom>.<imagen>` |
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

`view` también acepta `biblioteca`, `ajustes`, `common` y combinaciones
`library|settings` o `library,settings`.

`source` debe ser local y relativo al XML. No se permiten URL, rutas absolutas
ni `..`, de forma que un tema no pueda salir de su propio directorio:

```text
themes/
├── mi-tema.xml
└── assets/
    ├── fondo.webp
    └── marco.png
```

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

Un asset estático ausente genera un aviso y se omite sin inutilizar el resto del
tema. Un XML estructuralmente inválido, una fuente `game.*` desconocida, un ID
duplicado o un tamaño fuera de rango impiden aplicar esa versión.

## Temas incluidos

- `neon.xml`: layout clásico con marquee dinámico sobre la cabinet.
- `amber.xml`: variante arcade ámbar.
- `phosphor.xml`: estética de monitor de fósforo.
- `cyber-list.xml`: lista vertical de siete juegos, marquee, preview y ficha.
- `console-showcase.xml`: caja, cartucho, marquee, preview y carrusel horizontal.
- `theme-template.xml.example`: plantilla comentada para crear un tema nuevo.

Los XML incluidos son ejemplos funcionales y se pueden copiar y modificar.
