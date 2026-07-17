# NGNEON-EMU Configurator

Configurador web de desarrollo para NGNEON-EMU. La release utiliza ahora `ngneon-frontend.exe`, un frontend SDL2 nativo e independiente.

- Lee `roms/gamelist.xml` y enlaza vídeos, capturas, fanart, cajas, cartuchos,
  marquesinas, logos y manuales.
- Permite buscar, marcar favoritos, recordar juegos recientes y lanzar la ROM.
- Guarda los cambios antes de lanzar y evita abrir dos sesiones a la vez.
- Incluye modos Normal, Sesión limpia y Diagnóstico con logs en `logs/`.
- Muestra el login básico de RetroAchievements con el token de `rcheevos`.
- Con una Web API Key opcional muestra perfil, rango, puntos, presencia, juegos
  recientes, progreso y últimos logros.
- Conserva todos los ajustes de `config/ngneon.conf`.

## Comandos

```powershell
corepack pnpm install
corepack pnpm build
corepack pnpm start
```

Después abre:

```text
http://127.0.0.1:4177
```

El servidor solo escucha en `127.0.0.1`. Por seguridad, `ra_token`,
`ra_password` y `ra_api_key` no se devuelven al navegador; se conservan
intactos al guardar.

Este servidor ya no se incluye ni se abre en la release. Se conserva en el repositorio únicamente como herramienta de desarrollo y referencia de configuración.
