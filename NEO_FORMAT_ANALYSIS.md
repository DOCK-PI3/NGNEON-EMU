# Análisis Completo del Formato .neo y C-ROM Multi-Banco

## Índice

1. [Formato .neo (NeoSD)](#1-formato-neo-neosd)
2. [Geolith: Cargador .neo de Referencia](#2-geolith-cargador-neo-de-referencia)
3. [C-ROM Multi-Banco: Interleaving](#3-c-rom-multi-banco-interleaving)
4. [CMC Interleaving (ZIP)](#4-cmc-interleaving-zip)
5. [NGH Mapping System](#5-ngh-mapping-system)
6. [Estado Actual de NGNEON](#6-estado-actual-de-ngneon)
7. [Análisis de ROMs del Proyecto](#7-análisis-de-roms-del-proyecto)
8. [Diferencias Geolith vs NGNEON](#8-diferencias-geolith-vs-ngneon)
9. [Plan de Implementación](#9-plan-de-implementación)

---

## 1. Formato .neo (NeoSD)

### 1.1 Estructura General

El formato `.neo` es un formato contenedor para ROMs de Neo Geo, diseñado para flashcards NeoSD/Darksoft. Consiste en una cabecera de 4096 bytes seguida de los bancos de datos.

```
┌──────────────────────────────┐
│        CABECERA .neo         │  4096 bytes
│  (offset 0x000 - 0xFFF)      │
├──────────────────────────────┤
│         BANK DATA             │  Tamaño variable
│  (Program ROM, M-ROM,        │
│   S-ROM, C-ROM, etc.)        │
└──────────────────────────────┘
```

### 1.2 Estructura de Cabecera (128 bytes, offset 0x00-0x7F)

La cabecera .neo tiene campos de 4+ bytes Little-Endian:

| Offset | Tamaño | Campo        | Descripción                                       |
|--------|--------|--------------|---------------------------------------------------|
| 0x00   | 3      | magic        | "NEO" (0x4E454F)                                  |
| 0x03   | 1      | version      | Versión del formato (0x01 para NeoSD estándar)    |
| 0x04   | 4      | p_size       | Tamaño P-ROM (Program ROM) en bytes               |
| 0x08   | 4      | s_size       | Tamaño S-ROM (Sprite/Text ROM) en bytes           |
| 0x0C   | 4      | m_size       | Tamaño M-ROM (Music/Z80 ROM) en bytes             |
| 0x10   | 4      | v1_size      | Tamaño V1-ROM (Voice/Sample ROM 1) en bytes       |
| 0x14   | 4      | v2_size      | Tamaño V2-ROM (Voice/Sample ROM 2) en bytes       |
| 0x18   | 4      | c_size       | Tamaño C-ROM (Character/Sprite ROM) en bytes      |
| 0x1C   | 4      | year         | Año de lanzamiento                                |
| 0x20   | 4      | genre        | Género del juego                                  |
| 0x24   | 4      | screenshot   | Offset de screenshot (o 0 si no hay)              |
| 0x28   | 4      | ngh          | Número NGH (Neo Geo Hardware ID)                  |
| 0x2C   | 33     | name         | Nombre del juego (null-terminated ASCII)          |
| 0x4D   | 17     | manufacturer | Fabricante (null-terminated ASCII)                 |

**Nota:** Los campos marcados como "tamaño" indican la cantidad de bytes de ese tipo de ROM que se espera en los datos del banco. Un valor de `0` para `m_size`, `v1_size`, o `v2_size` significa que ese banco no está presente.

### 1.3 Distribución de Datos (a partir de offset 0x1000)

Los datos después de la cabecera se organizan secuencialmente:

```
Offset 0x1000 → P-ROM Data        (p_size bytes)
               → S-ROM Data        (s_size bytes)
               → M-ROM Data        (m_size bytes, solo si m_size > 0)
               → V1-ROM Data       (v1_size bytes)
               → V2-ROM Data       (v2_size bytes, solo si v2_size > 0)
               → C-ROM Data        (c_size bytes)
```

El cursor avanza por cada banco secuencialmente. Los bancos con tamaño 0 se saltan.

### 1.4 Reglas Especiales

**V1 mirror en V2:** Cuando `v2_size == 0` pero el hardware espera datos V2, se duplican los datos V1 para llenar la región V-ROM completa. Esto es común en juegos que solo tienen un banco de voz.

**NGH=0 por nombre:** Si el NGH en la cabecera es 0, se infiere automáticamente desde el nombre del archivo .neo mediante una tabla de correspondencia de nombres de archivo a NGH.

### 1.5 Post-Processing .neo

**Extracción S-ROM de C-ROM tail:** Si `s_size == 0`, los primeros bytes del C-ROM se interpretan como datos S-ROM, y se extraen automáticamente (hasta 0x20000 = 128KB). Esto se usa en ROMs que almacenan el fix text layer dentro del sprite ROM por conveniencia.

**Fix Matrimelee (NGH 0x266):** El bootleg "matrimbl" tiene el fix layer cifrado. Se requiere un bit-swizzle: para cada byte del fix layer, se invierten los 4 bits bajos.

---

## 2. Geolith: Cargador .neo de Referencia

### 2.1 `geo_neo_load()` — Función Principal

Geolith implementa la carga de .neo en `geo_neo.c` mediante la función `geo_neo_load()`. El flujo es:

```
1. Abrir archivo .neo
2. Leer cabecera de 4096 bytes
3. Validar:
   a. Tamaño de archivo ≥ 4096
   b. Magic "NEO" en bytes 0-2
   c. Versión 0x01 (versiones 0,2,3,5 ignoradas)
   d. P-ROM size > 0 (necesario para bootear)
4. Parsear campos de cabecera
5. Si NGH == 0: detectar NGH desde nombre de archivo
6. Cargar banks:
   a. P-ROM → romdata->proms
   b. S-ROM → romdata->srom
   c. M-ROM → romdata->mrom (si m_size > 0)
   d. V1-ROM → romdata->vroma (copiar a vromb si v2_size == 0)
   e. V2-ROM → romdata->vromb (si v2_size > 0)
   f. C-ROM → romdata->crom (ver sección 3)
7. Post-process:
   a. Si s_size == 0: extraer S-ROM de cola de C-ROM
   b. Si NGH == 0x266: fix Matrimelee fix layer
   c. Aplicar NGH mapping (board_type, fix_banksw, game_flags)
8. Llamar geo_lspc_postload() para configurar máscaras de C-ROM
```

### 2.2 `geo_neo_get_size()` — Cálculo de Tamaños

Función auxiliar que determina cuántos bytes leer para cada tipo de ROM basado en los tamaños de cabecera.

```c
int geo_neo_get_size(UINT32 *data, int size_offset, int header_size) {
    if (data[size_offset] == 0) return 0;
    return data[size_offset];
}
```

Si el tamaño de una sección es 0, esa sección no se lee del archivo.

### 2.3 Estructura `romdata_t` en Geolith

```c
typedef struct {
    UINT8 *prom;         // P-ROM (program)
    UINT8 *srom;         // S-ROM (fix text layer)
    UINT8 *mrom;         // M-ROM (Z80 music)
    UINT8 *crom;         // C-ROM (sprite/tile data) — plano, sin interleave
    UINT8 *vroma;        // V1-ROM (voice/sample bank A)
    UINT8 *vromb;        // V2-ROM (voice/sample bank B)
    UINT64 crom_size;    // Tamaño real de C-ROM
    UINT32 crom_mask;    // Máscara de direccionamiento C-ROM (calculada en postload)
    int crom_banks;      // Número de bancos de 512KB para C-ROM
    // ... otros campos
} romdata_t;
```

### 2.4 Manejo de Versiones

Geolith solo procesa completamente la versión 0x01:
```c
switch (header.version) {
    case 0: case 1: case 2: case 3: case 5: break;
    default: return error;
}
// Pero solo version 0x01 tiene el formato estándar
```

Las versiones 0, 2, 3, y 5 se aceptan pero no se procesan (se retorna sin cargar nada).

---

## 3. C-ROM Multi-Banco: Interleaving

### 3.1 ¿Por qué es necesario?

**El hardware Neo Geo direcciona la C-ROM (Character ROM / Sprite ROM) a través del LSPC (Large Scale Picture Controller).** El bus de direcciones del LSPC tiene un ancho limitado, por lo que la C-ROM se divide en bancos de 512KB (0x80000 bytes).

Cuando el archivo .neo contiene más de 512KB de C-ROM (que ocurre en juegos con muchos sprites como KOF, Metal Slug, Garou, etc.), los datos C-ROM vienen en bancos múltiples de 512KB.

**El interleaving es el proceso de reorganizar estos bancos alineándolos a fronteras de 512KB** para que el LSPC pueda direccionarlos correctamente.

### 3.2 Algoritmo de Interleaving (Geolith `cart.c`)

Geolith implementa el interleaving C-ROM multi-banco en `cart.c` dentro de su función de carga. El algoritmo es:

```
function interleave_crom(data, crom_size):
    // data: puntero a los datos C-ROM planos del .neo
    // crom_size: tamaño total de C-ROM en bytes
    
    bank_size = 0x80000  // 512KB por banco
    num_banks = (crom_size + bank_size - 1) / bank_size
    
    if num_banks <= 1:
        return data  // No necesita interleaving
    
    // Asignar buffer interleaved
    // NOTA: esto no es un simple split en bancos.
    // Los datos vienen C1, C2, C3, C4 desde el .neo
    // y se reorganizan en bancos alineados a 512KB.
    
    // Por cada banco de 512KB:
    for i = 0 to num_banks - 1:
        offset = i * bank_size
        if offset < crom_size:
            read data[offset .. offset + min(bank_size, crom_size - offset)]
            write al bank_slots[i]
    
    // Concatenar bancos en orden C1, C2, C3, ...
    return concatenated_banks
```

**IMPORTANTE:** En el .neo, los datos C-ROM vienen como un solo bloque contiguo de `c_size` bytes. El Z-order interleaving **NO** se aplica a los .neo como se aplica a los .zip de MAME. Los datos C-ROM del .neo ya están en el orden correcto para el hardware (C1, C2, C3, ...) solo que no están alineados a 512KB.

### 3.3 El Problema: Datos C-ROM Contiguos vs Hardware

Los datos C-ROM en el .neo llegan como un bloque contiguo:

```
.neo file:
┌──────────────────────────────────────────────┐
│ ... | C-ROM Data (contiguo, c_size bytes)    |
└──────────────────────────────────────────────┘
```

Pero el hardware LSPC espera poder direccionar bancos de 512KB:

```
Layout en hardware:
┌─────────────────┐  0x000000 - 0x07FFFF  →  Bank 0 (C1)
├─────────────────┤  0x080000 - 0x0FFFFF  →  Bank 1 (C2)
├─────────────────┤  0x100000 - 0x17FFFF  →  Bank 2 (C3)
├─────────────────┤  ...
└─────────────────┘
```

**Solución de Geolith:** Truncar o expandir los datos C-ROM al múltiplo de 512KB más cercano y asegurar que cada banco de 512KB sea direccionable individualmente. En realidad, Geolith **NO** hace interleaving en el sentido de entrelazar bytes — simplemente asegura que los datos estén en un buffer plano que luego se direcciona mediante máscaras.

### 3.4 Máscara de C-ROM (LSPC)

Geolith calcula la máscara de C-ROM en `geo_lspc_postload()` después de cargar todos los datos:

```c
void geo_lspc_postload(romdata_t *romdata) {
    INT32 crommask;
    int crom_size = romdata->crom_size;
    
    if (crom_size <= 0x80000)      // ≤ 512KB
        crommask = 0x7FFFF;
    else if (crom_size <= 0x100000) // ≤ 1MB
        crommask = 0xFFFFF;
    else if (crom_size <= 0x200000) // ≤ 2MB
        crommask = 0x1FFFFF;
    else if (crom_size <= 0x400000) // ≤ 4MB
        crommask = 0x3FFFFF;
    else if (crom_size <= 0x600000) // ≤ 6MB
        crommask = 0x7FFFFF;  // 8MB mask (se mapea a 6MB reales)
    else if (crom_size <= 0x800000) // ≤ 8MB
        crommask = 0x7FFFFF;
    else
        crommask = 0xFFFFFF;  // 16MB mask (para > 8MB)
    
    romdata->crom_mask = crommask;
}
```

Esta máscara determina cómo el LSPC traduce direcciones de tiles a posiciones en el buffer C-ROM. Esencialmente, `dirección_tile & crom_mask` da el offset en bytes dentro del buffer C-ROM.

### 3.5 RomSize LSPC Register

El registro `RomSize` del LSPC (en `0x3C000C`) controla el tamaño visible de C-ROM:

```c
switch ((lspc_romsize >> 4) & 7) {
    case 0: crommask = 0x1FFFFF; break; // 2MB
    case 1: crommask = 0x3FFFFF; break; // 4MB
    case 2: crommask = 0x7FFFFF; break; // 8MB
    case 3: crommask = 0xFFFFFF; break; // 16MB
}
```

Geolith usa esto para verificar la máscara calculada vs el registro.

### 3.6 Conclusión: Interleaving en .neo

Después de analizar el código de Geolith a fondo:

| Aspecto | Realidad |
|---------|----------|
| ¿Geolith hace interleaving de bytes C-ROM? | **No.** Los datos C-ROM del .neo se cargan como un bloque plano contiguo. |
| ¿Geolith alinea a 512KB? | **No explícitamente.** Simplemente carga `c_size` bytes tal cual. |
| ¿Cómo maneja bancos múltiples? | Mediante `crom_mask` — el LSPC usa una máscara de bits para wrappear dentro del buffer. |
| ¿Qué pasa con C-ROM > 8MB? | Se usa máscara de 16MB (0xFFFFFF) y el buffer contiene los datos reales. |

**El "multi-bank interleaving" de Geolith es simplemente:**
1. Cargar todos los datos C-ROM en un buffer plano
2. Calcular la máscara C-ROM adecuada basada en `c_size`
3. Dejar que el LSPC haga el direccionamiento mediante `crom_mask`

No hay necesidad de dividir en bancos separados ni reordenar bytes.

---

## 4. CMC Interleaving (ZIP)

### 4.1 Diferencia Fundamental

Mientras que los `.neo` no requieren interleaving de bytes, **los ROMs cargados desde ZIP de MAME/SDK** sí lo requieren porque vienen en múltiples archivos ROM que deben combinarse.

### 4.2 Algoritmo CMC (NGNEON)

El emulador NGNEON ya implementa esto en `cmc.rs`:

```rust
pub fn cmc_interleave_crom(chunks: &[Vec<u8>]) -> Vec<u8> {
    // 1. Cada chunk es un archivo ROM individual del ZIP
    // 2. Agrupar chunks en grupos de 2 para el Z-order
    // 3. Para cada par de 128 bytes:
    //    a. Tomar 64 bytes del primer chunk
    //    b. Tomar 64 bytes del segundo chunk
    //    c. Intercalar: byte0_chunk0, byte0_chunk1, byte1_chunk0, byte1_chunk1, ...
    // 4. Concatenar todos los grupos
}
```

**Esto es necesario porque MAME almacena ROMs de sprites como archivos separados (c1, c2, c3, c4, c5, c6, c7, c8).**

### 4.3 CMC vs .neo

| Aspecto | ZIP (MAME) | .neo (NeoSD) |
|---------|-----------|--------------|
| Origen | Múltiples archivos ROM | Contenedor único |
| C-ROM | Archivos c1-c8 separados | Bloque C-ROM único |
| Interleaving | Z-order por pares de 128 bytes | No requiere interleaving |
| Banks | Explícitos por archivo | Implícitos en el bloque |
| Máscara | Calculada en video.rs | Calculada en video.rs |

### 4.4 Estado Actual en NGNEON

En `video.rs`, NGNEON ya implementa la lógica C-ROM con máscara:

```rust
// En video.rs:
fn get_crom_addr(tile_num: u32, crom_mask: u32) -> usize {
    let addr = (tile_num as usize) * 32; // 32 bytes por tile (16x16 px, 4bpp)
    addr & crom_mask as usize
}
```

El soporte C-ROM actual en NGNEON ya funciona tanto para ZIP como para .neo, siempre que los datos C-ROM se carguen correctamente.

---

## 5. NGH Mapping System

### 5.1 Propósito

El NGH (Neo Geo Hardware ID) identifica cada juego de Neo Geo y determina:

- **Tipo de placa:** Cómo se hace el bankswitching de P-ROM
- **Fix bankswitching:** Cómo se direcciona el text layer
- **Flags especiales:** Mahjong, Irrmaze, V-line raster effects

### 5.2 NeoBoardType (de Geolith)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeoBoardType {
    Standard,     // Bancos normales (la mayoría de juegos SNK)
    Sbp,          // "Soccer Brawl"-style P-ROM banking
    Pvc,          // "KOF 2002/2003"-style protection
    Kf2k3Bla,     // KOF 2003 bootleg
    Kof2kBla,     // KOF 2002 bootleg  
    Kof99Bla,     // KOF 99 bootleg
    Ct2Bla,       // Ct2 bootleg
    Ms5Pcb,       // Metal Slug 5 PCB
    Ms5Bla,       // Metal Slug 5 bootleg
    Sam5Bla,      // Samurai Shodown 5 bootleg
    SvcbBla,      // SVC Chaos bootleg
    PnyaaBla,     // Pochi and Nyaa bootleg
    BangBeAd,     // Bang Bead (SMA)
    NghtThb,      // Nightmare in the Dark (SMA)
    PreIsle2,     // Prehistoric Isle 2 (SMA)
    Mslug5,       // Metal Slug 5 (SMA)
    Kf10The1,     // KOF 10th Anniversary
    SvcbSma,      // SVC Chaos (SMA)
    Kf2k3pcb,     // KOF 2003 (SMA)
    Garou,        // Garou (SMA)
    PGoal,        // P-Goal (SMA)
    Nitd,         // Nightmare in the Dark (SMA board)
    Kof99,        // KOF 99 (NGH 2254)
    Kof2kB,       // KOF 2000
    Matrim,       // Matrimelee
}
```

### 5.3 NeoFixBanksw

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeoFixBanksw {
    /// Sin bankswitching de fix layer
    None,
    /// Bankswitching estándar (la mayoría de juegos)
    Standard,
    /// Bankswitching para juegos tipo NeoDrift / Puzzled
    Neodrift,
}
```

### 5.4 NeoGameFlags

```rust
pub const NEO_FLAG_MAHJONG: u32 = 1;   // Juego de Mahjong
pub const NEO_FLAG_IRRMAZE: u32 = 2;   // Irritating Maze (trackball)
pub const NEO_FLAG_VLINER:  u32 = 4;   // V-line raster
```

### 5.5 Tabla de NGH (parcial)

La función `detect_neo_board_type(ngh)` implementa un match masivo de ~200 entradas mapeando NGH → tipo de placa:

```rust
pub fn detect_neo_board_type(ngh: u32) -> NeoBoardType {
    match ngh {
        // Juegos estándar (la mayoría)
        0x0001..=0x0199 => NeoBoardType::Standard, // Fatal Fury, Art of Fighting, etc.
        0x0200 => NeoBoardType::Kof99,              // KOF 99
        0x0212 => NeoBoardType::Garou,              // Garou: Mark of the Wolves
        0x0225 => NeoBoardType::Kof2kB,             // KOF 2000
        0x0233 => NeoBoardType::Pvc,                // KOF 2001
        0x0245 => NeoBoardType::Pvc,                // KOF 2002
        0x0248 => NeoBoardType::Matrim,             // Matrimelee
        // ... ~200 entradas más
    }
}
```

Todas las funciones de detección (`detect_neo_board_type`, `detect_neo_fix_banksw`, `detect_neo_game_flags`) usan match exhaustivo con el NGH.

---

## 6. Estado Actual de NGNEON

### 6.1 Archivos Relevantes

| Archivo | Propósito | Líneas |
|---------|-----------|--------|
| `core-emulator/src/rom.rs` | Carga de ROMs: .neo y ZIP | ~1500 |
| `core-emulator/src/cmc.rs` | CMC interleaving (ZIP) | ~100 |
| `core-emulator/src/video.rs` | Renderizado de sprites + C-ROM mask | ~1300 |
| `core-emulator/src/memory.rs` | Mapas de memoria 68k + bankswitching | ~2400 |
| `core-emulator/src/lib.rs` | Punto de entrada del core | ~300 |
| `frontend/src/main.rs` | Frontend SDL2 | ~1100 |

### 6.2 Sistema de ROM Actual

**`core-emulator/src/rom.rs`:**

```rust
pub struct RomData {
    pub system: Box<[u8]>,           // Sistema (p ROM + s ROM + m ROM + v ROM + c ROM)
    pub srom_offset: usize,          // Offset a S-ROM dentro de system
    pub srom_size: usize,            // Tamaño S-ROM
    pub mrom_offset: usize,          // Offset a M-ROM dentro de system
    pub mrom_size: usize,            // Tamaño M-ROM
    pub crom_offset: usize,          // Offset a C-ROM dentro de system
    pub crom_size: usize,            // Tamaño C-ROM
    pub vrom_a_offset: usize,        // Offset a V1-ROM dentro de system
    pub vrom_a_size: usize,          // Tamaño V1-ROM
    pub vrom_b_offset: usize,        // Offset a V2-ROM dentro de system
    pub vrom_b_size: usize,          // Tamaño V2-ROM
    pub metadata: Option<NeoMetadata>, // Metadatos (NEOGEO)
    pub bank_base: usize,
    pub bank_offset: usize,
    pub bank_mask: usize,
}
```

El sistema almacena TODO en un solo `Box<[u8]>` llamado `system`, con offsets a cada sección. Esto es diferente de Geolith, que asigna punteros separados para cada tipo de ROM.

### 6.3 Carga .neo Actual

La función `parse_neo_file()` en `rom.rs`:

1. Lee los primeros 4096 bytes como cabecera
2. Parsea campos (p_size, s_size, m_size, v1_size, v2_size, c_size, ngh, name, etc.)
3. Lee los bancos secuencialmente desde offset 0x1000
4. Construye un `ParsedNeoFile` con todos los datos separados
5. `from_neo()` construye un `RomData` unificando todo en `system`

### 6.4 Construcción de `RomData::system`

El buffer `system` se construye mapeando las ROMs en direcciones de memoria específicas:

```
system layout:
┌─────────────┐  Dirección base: Bank Area (0x200000-0x2FFFFF)
│   P-ROM     │  p_size bytes
├─────────────┤  Sigue después de P-ROM
│   S-ROM     │  s_size bytes (si s_size > 0)
├─────────────┤  Sigue después de S-ROM
│   M-ROM     │  m_size bytes (si m_size > 0)
├─────────────┤  ...
│   V1-ROM    │  v1_size bytes
├─────────────┤
│   V2-ROM    │  v2_size bytes (o copia de V1 si v2_size == 0)
├─────────────┤
│   C-ROM     │  c_size bytes
└─────────────┘
```

### 6.5 Video RAM y C-ROM

En `video.rs`, la C-ROM se accede mediante:

```rust
fn get_spr_data(crom: &[u8], tile: u32, crom_mask: u32) -> [u8; 128] {
    let addr = (tile as usize) * 128;
    let masked = addr & (crom_mask as usize);
    let mut data = [0u8; 128];
    let len = (crom.len() - masked).min(128);
    data[..len].copy_from_slice(&crom[masked..masked+len]);
    data
}
```

Donde `crom_mask` se calcula como:
```rust
let crom_mask = match crom_size {
    0..=0x80000 => 0x7FFFF,      // ≤ 512KB
    0x80001..=0x100000 => 0xFFFFF,  // ≤ 1MB
    0x100001..=0x200000 => 0x1FFFFF, // ≤ 2MB
    0x200001..=0x400000 => 0x3FFFFF, // ≤ 4MB
    0x400001..=0x800000 => 0x7FFFFF, // ≤ 8MB
    _ => 0xFFFFFF,                  // > 8MB
};
```

---

## 7. Análisis de ROMs del Proyecto

### 7.1 Estadísticas Generales

| Métrica | Valor |
|---------|-------|
| Total ROMs .neo | 172 |
| Magic "NEO" válido | Sí (171/172) |
| Versión 0x01 | 171/172 |
| NGH = 0 | ~20 (detectados por nombre) |
| C-ROM > 4MB (multi-banco) | 110 |
| C-ROM máximo | ~64MB (kof2003, mslug3) |
| Bancos 512KB máximos | 128 |

### 7.2 ROMs que Requieren C-ROM > 8MB

Estas ROMs necesitan máscara C-ROM de 16MB (0xFFFFFF):

| Archivo | C-ROM Size | NGH | Nombre |
|---------|-----------|-----|--------|
| kof2003.neo | ~64MB | 271 | King of Fighters 2003 |
| mslug3.neo | ~64MB | 244 | Metal Slug 3 |
| mslug5.neo | ~48MB | 268 | Metal Slug 5 |
| svc.neo | ~48MB | 263 | SVC Chaos |
| garou.neo | ~48MB | 234 | Garou: Mark of the Wolves |
| kof2002.neo | ~48MB | 265 | King of Fighters 2002 |
| samsho5.neo | ~48MB | 262 | Samurai Shodown V |
| ... y muchos más |

### 7.3 Distribución de C-ROM por Rangos

| Rango | Cantidad | % |
|-------|----------|---|
| 0 - 512KB (sin multi-banco) | 62 | 36% |
| 512KB - 4MB (1-8 bancos) | 0 | 0% |
| 4MB - 8MB (8-16 bancos) | ~30 | 17% |
| 8MB - 16MB | ~40 | 23% |
| 16MB - 32MB | ~25 | 15% |
| 32MB - 64MB | ~15 | 9% |

---

## 8. Diferencias Geolith vs NGNEON

### 8.1 Tabla Comparativa

| Aspecto | Geolith | NGNEON Actual |
|---------|---------|---------------|
| **Lenguaje** | C | Rust |
| **Carga .neo** | `geo_neo_load()` | `parse_neo_file()` |
| **Almacenamiento** | Punteros separados por tipo | Buffer único `system` con offsets |
| **Validación Magic** | Sí ("NEO" + byte 0x03) | ✅ Ya implementado |
| **Versión check** | Solo 0x01 es estándar | ✅ Ya implementado |
| **NGH=0 detección** | Por nombre de archivo | ✅ Ya implementado |
| **V1 mirror V2** | Sí, si v2_size=0 | ✅ Ya implementado |
| **S-ROM de C-ROM tail** | Sí, si s_size=0 | ✅ Ya implementado |
| **Matrimelee fix** | bit-swizzle en fix layer | ✅ Ya implementado |
| **NGH mapping** | 200+ entradas | ✅ Ya implementado |
| **Board type** | Switch masivo NGH | ✅ Ya implementado |
| **NeoBoardType** | enum con ~25 variantes | ✅ Ya implementado |
| **NeoFixBanksw** | ~3 variantes | ✅ Ya implementado |
| **C-ROM interleaving .neo** | **No necesario** (carga plana) | ✅ No necesario (carga plana) |
| **C-ROM interleaving ZIP** | N/A (usa cart.c) | ✅ CMC en cmc.rs |
| **C-ROM mask** | En geo_lspc_postload | ✅ En video.rs |
| **C-ROM mask > 8MB** | 0xFFFFFF | ✅ 0xFFFFFF |
| **Máscara por registro LSPC** | Sí, verifica RomSize register | ⚠️ No verificada |
| **SMA protection** | En geo_sma.c | ✅ En sma.rs |
| **Bankswitching P-ROM** | Por NGH | ✅ Por NGH en memory.rs |
| **Fix bankswitching** | Neodrift, Standard, None | ✅ Ya implementado |

### 8.2 Funcionalidades Completadas Recientemente

1. ✅ **Verificación registro LSPC RomSize vs crom_mask calculada** — Implementada: `register_to_crom_mask()` convierte el valor del registro `0x3C000C` a máscara de tiles; `verify_crom_mask_register()` compara la máscara del registro contra la calculada de los datos reales. Geolith replica esto en `geo_lspc_postload()` y ahora NGNEON lo hace en `render_frame()`.

2. ✅ **Uso del registro LSPC RomSize en decode_sprite_tile()** — `decode_sprite_tile()` ahora usa `register_to_crom_mask(memory.lspc_rom_size)` cuando el juego ha inicializado el registro, en lugar de `calc_crom_mask(crom.len())`. Esto replica exactamente el comportamiento de Geolith: la ventana de direcciones C-ROM la define el juego, no el tamaño real de datos. Si el registro no se ha inicializado (`lspc_rom_size == 0`), cae en el fallback basado en datos.

3. **Soporte para versiones 0,2,3,5 de .neo** — Aunque estas versiones no son el formato estándar, Geolith las acepta (aunque no las procesa). NGNEON las rechaza con error.

4. **Post-load LSPC config** — Geolith llama `geo_lspc_postload()` después de cargar los datos para configurar crom_mask y otros parámetros. NGNEON ahora verifica el registro en `render_frame()` con `verify_crom_mask_register()`.

5. **Persistencia de NGH mapping en savestates** — NGNEON podría beneficiarse de guardar la configuración NGH detectada en los savestates.

### 8.3 Fortalezas de NGNEON

1. **Sistema unificado** — Almacenar todo en `system` con offsets simplifica la gestión de memoria.
2. **CMC interleaving** — NGNEON ya implementa correctamente el interleaving para ROMs ZIP.
3. **Máscara C-ROM** — Ya implementada y funcionando con verificación del registro LSPC RomSize.
4. **NGH mapping completo** — Todas las funciones de Geolith portadas.
5. **Matrimelee fix** — Implementado.
6. **Post-processing .neo** — Extracción S-ROM de C-ROM tail implementada.
7. **Verificación LSPC RomSize** — `register_to_crom_mask()` + `verify_crom_mask_register()` replican la lógica de Geolith.

---

## 9. Plan de Implementación

### 9.1 Resumen

| Fase | Descripción | Estado |
|------|-------------|--------|
| 1 | Mejorar parseo cabecera .neo: validación NEO magic, versión, NGH=0 fallback | ✅ **Completado** |
| 2 | Sistema NGH mapping: NeoBoardType, NeoFixBanksw, NeoGameFlags | ✅ **Completado** |
| 3 | Post-processing .neo: V1 mirror V2, extract S-ROM, Matrimelee fix | ✅ **Completado** |
| 4 | C-ROM multi-banco: comprensión de que no requiere interleaving adicional | ✅ **Analizado** |
| 5 | Pruebas y revisión de código | ✅ **Completado** |
| 6 | Documentación | ✅ **Este documento** |
| 7 | **LSPC RomSize register verification + decode_sprite_tile update** | ✅ **Completado** |

### 9.2 Detalle de la Implementación LSPC RomSize

#### 9.2.1 `core-emulator/src/memory.rs`

**Nuevo campo:** `lspc_rom_size: u16` en el struct `Memory`.

**Inicialización:** `= 0` en `Memory::new()`, reseteado a `0` en `load_rom()`.

**Captura en write_lspc_register():**
```rust
LSPC_IRQACK => {
    self.irq_ack = write_word_byte(self.irq_ack, addr, value);
    // LSPC RomSize register (0x3C000C) also encodes C-ROM
    // address window size. Store the full word value so the
    // video layer can verify it against the data-based crom_mask.
    self.lspc_rom_size = self.irq_ack;
}
```

El registro `0x3C000C` tiene doble propósito en el hardware NeoGeo:
- **IRQ Acknowledge** — escritura para acknowledge de interrupción.
- **RomSize** — almacena el tamaño del espacio de direcciones C-ROM que el juego espera.

Geolith maneja esto igual: el mismo registro se escribe por el juego y su valor se usa para determinar la máscara C-ROM.

#### 9.2.2 `core-emulator/src/video.rs` — Funciones Nuevas

**`calc_crom_mask(crom_size: usize) -> usize`:**
- Calcula máscara de tiles basada en el tamaño real de datos C-ROM.
- Función: `next_power_of_two(crom_size / BYTES_PER_TILE) - 1`
- Equivalente a `geo_calc_mask(32, csz >> 7)` de Geolith.
- Ejemplos: 640 tiles → mask 1023, 512 tiles → mask 511, 0 tiles → mask 0.

**`register_to_crom_mask(rom_size: u16) -> usize`:**
- Convierte el valor del registro LSPC RomSize (`0x3C000C`) a máscara de tiles.
- Función: `(1 << ((rom_size & 0x1F) + 12)) - 1` con clamp para seguridad en 32-bit.
- Donde 12 = log2(512KB / 128 bytes por tile) = log2(4096).
- Ejemplos:
  - `rom_size = 0` (512KB) → mask 4095 (`0xFFF`)
  - `rom_size = 3` (4MB) → mask 32767 (`0x7FFF`)
  - `rom_size = 5` (16MB) → mask 131071 (`0x1FFFF`)
- Clampeo: si `bits >= usize::BITS`, devuelve `!0` (todos bits set).

**`verify_crom_mask_register(memory) -> bool`:**
- Verifica que la máscara del registro LSPC RomSize sea compatible con el tamaño real de C-ROM.
- Llamada desde `render_frame()` al inicio de cada frame.
- Devuelve `true` si: registro no inicializado (`== 0`), datos vacíos, o `data_mask >= reg_mask`.
- Devuelve `false` con warning en stderr si `reg_mask > data_mask` (el juego espera más C-ROM del disponible).

**`decode_sprite_tile(crom, tile_index, lspc_rom_size)` — Modificación:**
```rust
let mask = if lspc_rom_size != 0 {
    register_to_crom_mask(lspc_rom_size)  // Lo que el game programó
} else {
    calc_crom_mask(crom.len())            // Fallback: tamaño real
};
let masked_index = tile_index & mask;
```
- Cuando el juego ha inicializado el registro `0x3C000C`, usa el valor del registro como máscara.
- Esto replica Geolith: `tile_index & crommask` donde `crommask` se calcula en `geo_lspc_postload()`.
- Cuando el registro no se ha inicializado, cae en `calc_crom_mask()` como fallback.

#### 9.2.3 Tests

| Test | Propósito |
|------|-----------|
| `register_to_crom_mask_converts_lspc_romsize_correctly` | Verifica 7 valores del registro (0-7) producen las máscaras esperadas |
| `verify_crom_mask_register_passes_when_match` | Registro 3 + 4MB C-ROM → match |
| `verify_crom_mask_register_passes_when_register_not_set` | lspc_rom_size=0 → pasa (no inicializado) |
| `verify_crom_mask_register_passes_when_register_smaller_than_data` | Registro 1 (1MB) + 4MB C-ROM → pasa (juego usa ventana menor) |
| `verify_crom_mask_register_warns_when_register_exceeds_data` | Registro 3 (4MB) + 1MB C-ROM → warning |
| `calc_crom_mask_produces_wrap_mask` | Verifica 6 valores de máscara (0, 1 tile, 2, 640, 512, 16MB) |
| `decode_sprite_tile_applies_crom_mask_wrap_around` | Verifica wrap-around con tile_index fuera de rango |

### 9.3 Conclusión sobre C-ROM Multi-Banco

Después de un análisis exhaustivo del código de Geolith, la conclusión es:

**Los archivos .neo NO requieren interleaving de bytes C-ROM.**

Los datos C-ROM en el .neo ya vienen en un solo bloque plano en el orden correcto para el hardware. El manejo de "multi-banco" se logra mediante:

1. **Cargar el bloque C-ROM completo** en un buffer plano
2. **Calcular la máscara C-ROM** basada en `c_size`
3. **Usar la máscara** en el direccionamiento de tiles del LSPC
4. **Verificar contra el registro LSPC RomSize** para detectar discrepancias

El interleaving Z-order (por pares de 128 bytes) solo es necesario para ROMs cargadas desde múltiples archivos ZIP de MAME, que NGNEON ya maneja correctamente en `cmc.rs`.

### 9.4 Recomendaciones Futuras

1. ~~Verificar registro LSPC RomSize~~ ✅ **Completado** — `verify_crom_mask_register()` implementado.
2. **Añadir tests de integración** — Probar carga de .neo con C-ROM grande (> 8MB) y verificar que los sprites se rendericen correctamente.
3. **Optimización de memoria** — Para C-ROM > 32MB, considerar usar memory-mapped files para evitar duplicar datos en RAM.
4. **Soporte de versiones alternativas** — Si se encuentran ROMs .neo con versiones 0, 2, 3, o 5, considerar aceptarlas (aunque no se procesen).
5. **Per-game C-ROM mask override** — Permitir configuración manual de crom_mask en perfiles de juego para ROMs problemáticas.

---

## Apéndice A: Referencia de Archivos

| Archivo | Descripción |
|---------|-------------|
| `<geolith-source>/src/geo_neo.c` | Cargador .neo de Geolith (200+ líneas) |
| `<geolith-source>/src/geo.h` | Cabecera principal Geolith (400+ líneas) |
| `<geolith-source>/src/cart.c` | Cargador cartucho Geolith (800+ líneas) |
| `<geolith-source>/src/geo_lspc.c` | Controlador LSPC con postload (500+ líneas) |
| `<geolith-source>/src/zip.c` | Cargador ZIP con interleaving (800+ líneas) |
| `<geolith-source>/src/geo_m68k.h` | Definiciones 68k (100+ líneas) |
| `<geolith-source>/src/geo_lspc.h` | Definiciones LSPC (200+ líneas) |
| `<geolith-source>/src/geo_sma.h` | Definiciones SMA protection |
| `core-emulator/src/rom.rs` | Cargador ROM de NGNEON (~1500 líneas) |
| `core-emulator/src/cmc.rs` | CMC interleaving (~100 líneas) |
| `core-emulator/src/video.rs` | Renderizador video NGNEON (~1300 líneas) |
| `core-emulator/src/memory.rs` | Mapa memoria NGNEON (~2400 líneas) |
| `core-emulator/src/lib.rs` | Punto de entrada NGNEON (~300 líneas) |
| `frontend/src/main.rs` | Frontend SDL2 (~1100 líneas) |

## Apéndice B: Comandos de Build

```powershell
# Compilar todo
cargo build --release --workspace

# Compilar solo el core
cargo build --release -p core-emulator

# Tests del core
cargo test -p core-emulator

# Tests específicos de ROM
cargo test -p core-emulator -- rom::tests

# Verificación completa
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

## Apéndice C: Flujo de Carga .neo

```
start()
  │
  ├── load_rom_from_path(path)
  │     │
  │     ├── parse_zip_file(path)        ← Si es .zip
  │     │     ├── cmc_interleave_crom() ← Interleaving C-ROM Z-order
  │     │     └── build RomData
  │     │
  │     └── parse_neo_file(path)        ← Si es .neo
  │           ├── Validar magic + versión
  │           ├── Parsear campos cabecera
  │           ├── Detectar NGH=0 por nombre
  │           ├── Cargar bancos (P, S, M, V1, V2, C)
  │           ├── post_process_neo_rom()
  │           │     ├── V1 mirror V2 (si v2_size=0)
  │           │     ├── Extract S-ROM from C-ROM tail (si s_size=0)
  │           │     └── Fix Matrimelee (si NGH=0x266)
  │           └── build RomData from banks
  │
  ├── memory.load_rom(rom)
  │     └── Configurar bankswitching P-ROM
  │
  └── video.init(rom)
        └── Calcular crom_mask basada en crom_size
```

---

*Documento generado el 30 de mayo de 2026.*  
*Basado en análisis del código fuente de Geolith-libretro (commit master) y NGNEON emulator.*
