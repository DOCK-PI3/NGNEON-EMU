# Auditoría de paridad de hardware con Geolith

Fecha de la línea base: 20 de julio de 2026.

## Referencias fijadas

- Geolith libretro: `c5b57a6b31b7abef4a8a9b521cae58d653e28154` (26-06-2026).
- Geolith upstream: `4c0db390fdda159bd0e3e2298e006e6ff076daef` (07-07-2026).

Archivos locales usados en la auditoría:

| Archivo | SHA-256 |
|---|---|
| `zedblade.neo` | `8B943B6249C033A8D1D89AABEC0D07C6EB455E04115204EAD15232C1EB77EB86` |
| `irrmaze.neo` | `619AF7B3AF3209766FC03D0631E93735AF39A20BB70BD6AD9FB3B806170C889A` |
| `jockeygp.neo` | `59E2B72AC6EBA028574703B13250B7F1C019275BB3BAE8DDDA1567B6F74ED6A4` |
| `aes.zip` | `1D3936D2D5B7809B23EA0C3BEF22A352C9F0B34F7DF8CC3AD53120A7B9C1CDB4` |
| `neogeo.zip` | `BF05CC0D9E9B9C5408EC20916C9101E964B97314B4D92AB42D4586060AE8715B` |
| `uni-bios-40.zip` | `FFC164DADB804F1C8C8426F301FBC528C3ABEC58B3F8B90C820E69B62D55C3E3` |

## Cambios de hardware aplicados

- Frecuencia AES YM2610 derivada del reloj maestro y resto temporal incluido en savestate.
- Reset normal del Z80 conserva bancos; la carga de cartucho usa reset frío.
- `POUTPUT`, espejo AES, P1–P4 y multiplexación FTC1B.
- Trackball de Irritating Maze y puertos especiales de V-Liner.
- Cartridge RAM Brezzasoft persistente en un archivo separado de 8192 bytes.
- Asignación determinista de hasta cuatro mandos y configuración P1/P2 de teclado.
- Pacing A/V por ocupación del ring buffer sin alterar el núcleo determinista.
- Eliminación del filtrado FIX específico de Zed Blade.

Neo Geo CD y controles Mahjong quedan fuera de esta auditoría.

## Resultado Zed Blade

La referencia libretro y NGNEON se ejecutaron 600 frames con UniBIOS 4.0, hardware MVS y overscan lateral de 8 píxeles. Los hashes se calculan sobre el área visible 304×224.

| Frame | Píxeles no negros Geolith/NGNEON | Hash FNV-1a Geolith/NGNEON |
|---:|---:|---|
| 120 | 9497 / 9497 | `9307B46B395A38FF` / `9307B46B395A38FF` |
| 300 | 0 / 0 | `3F35BB40BEA5EB25` / `3F35BB40BEA5EB25` |
| 600 | 5318 / 5318 | `57394352A4D52071` / `57394352A4D52071` |

Ambos producen 563200 pares PCM nativos en 600 frames. Las salidas de auditoría se guardan en `target/hardware-sync-audit/` y no forman parte del paquete.

## Compatibilidad

- `four_player=off` conserva el comportamiento clásico por defecto.
- `NeoGeo::set_input()` continúa siendo el wrapper de P1.
- Los archivos antiguos de teclado se cargan como P1.
- Los nombres de configuración antiguos de mandos siguen siendo un fallback.
- Los savestates antiguos siguen cargando; no se garantiza que ejecutables antiguos carguen el nuevo formato.
