# Comparación y sincronización de proyectos — 21 de octubre de 2026

La copia de desarrollo se comparó por SHA-256 con la copia local vinculada a
GitHub, después de actualizar `origin/main`. Ambas referencias de Git estaban
en `7db74f3` y el árbol de trabajo de destino estaba limpio antes de empezar.

## Resultado

- 277 archivos de proyecto en origen y 273 en destino dentro del ámbito revisado.
- 260 archivos comunes idénticos; no había diferencias aisladas de finales de línea.
- Seis archivos con cambios pendientes: cuatro módulos del frontend y dos README.
- Once archivos ausentes, de los que nueve se incorporan y dos se excluyen por obsolescencia.
- Siete archivos exclusivos de destino, conservados íntegramente: LICENSE y seis XML/plantillas heredados.
- El núcleo, las dependencias vendorizadas, los manifiestos, Cargo.lock, los shaders,
  el configurador, los scripts de arranque/empaquetado y la integración de Raspberry Pi coinciden.

## Cambios incorporados

- Neo Collection y Cinema Neo, incluidos XML, imágenes, fuente y licencia OFL.
- Cuadrícula de carátulas, tira de capturas, botón Jugar/Play, traducciones,
  contraste y navegación por teclado, mando y ratón, con sus pruebas.
- README principal y referencia de temas actualizados.
- BUILDING.md, HARDCORE_COMPLIANCE.md y config/ngneon.example.conf sin credenciales.
- Compatibilidad adicional para los cinco XML planos antiguos presentes en
  destino: se mantienen sus archivos, pero el catálogo prioriza los temas en
  carpetas con el mismo ID para evitar avisos y cargar la versión actual.
  Se añade una prueba de regresión para esa migración.

## Archivos que no se recuperan

- `.github/workflows/pages.yml`: intenta publicar `webhub/`, directorio que ya
  no existe en ninguna copia. Reintroducirlo dejaría un despliegue inválido.
- `r68k_limitacion.md`: fue retirado expresamente en `b2a27d9` y describe una
  CPU r68k anterior a la implementación actual basada en Musashi.
- Instrucciones locales de agentes, skills y notas de trabajo no se publican
  como código del producto.

## Datos preservados

La comparación de código excluye .git, compilaciones, paquetes, node_modules,
dist, ROMs, BIOS, media de juegos, música, logs, capturas, saves y configuración
personal. No se copian credenciales ni se modifica config/ngneon.conf.
La plantilla *.example.conf sí se revisa y se incorpora por ser configuración
de distribución. Se conserva la LICENSE exclusiva de GitHub.

El repositorio ya tenía versionados config/ngneon.conf y artefactos del
configurador (node_modules/dist), aunque están en .gitignore. No se alteran
como parte de esta sincronización ni se vuelven a copiar desde desarrollo.

Los cambios se dejan en el árbol de trabajo para revisión. Esta sincronización
no crea un commit ni publica cambios en el remoto.

## Validación final

- `cargo fmt --all -- --check`: correcto.
- `cargo clippy --workspace --all-targets -- -D warnings`: correcto.
- `cargo test --workspace --quiet -- --test-threads=1`: 554 pruebas correctas;
  una prueba opcional con biblioteca comercial queda ignorada por diseño.
- `cargo build --release --workspace`: correcto; ejecutables de emulación y
  frontend generados para Windows x64.
- `git diff --check`: correcto.
- Los 66 archivos del núcleo siguen idénticos al origen y no se han modificado.
- Los assets declarados por ambos temas existen; las licencias de fuente se incluyen.
- LICENSE, Cargo.lock, configuración personal y XML heredados conservan sus hashes.

La captura de temas en tests se estabiliza después del fade-in, para no medir
un primer frame transparente. Durante la ejecución paralela se observó un fallo
intermitente del test de Timer B del audio nativo; el puente YMFM tiene estado
global compartido y la batería completa pasa en serie. Se documenta esta
limitación de las pruebas sin cambiar código del núcleo.
