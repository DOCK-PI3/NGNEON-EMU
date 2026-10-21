# Preparación para RetroAchievements Hardcore

Auditoría realizada el 5 de julio de 2026 contra los
[Hardcore Compliance Requirements](https://docs.retroachievements.org/general/hardcore-compliance-requirements.html).

Este documento describe el estado técnico del emulador. No implica aprobación
oficial de RetroAchievements.

## Protecciones implementadas

| Requisito | Estado en NGNEON-EMU |
|---|---|
| Cargar save states | Bloqueado dentro del núcleo mientras Hardcore esté activo, independientemente de la interfaz usada. |
| Crear save states | Permitido para depuración. Incluye el progreso interno de `rcheevos`, pero no puede cargarse hasta volver a Casual. |
| Casual → Hardcore | Aplica un reset completo y limpia el audio antes de ejecutar otro frame. |
| Hardcore → Casual | Permitido sin reset. |
| Auto-resume | Nunca carga el estado automático cuando Hardcore está activo. |
| Rewind, cámara lenta y frame advance | No están implementados en el emulador. |
| Cheats y dispositivos de cheats | No existe motor de cheats. UniBIOS queda bloqueada y se sustituye por una BIOS oficial MVS/AES antes de activar Hardcore. |
| Hacks de emulación | Overclock 68K, límite de sprites modificado y ADPCM wrap alternativo se desactivan y bloquean. |
| Herramientas de depuración | Dumps automáticos y overlay de depuración se desactivan y bloquean. |
| Pausa | Se consulta `rc_client_can_pause()` antes de detener la emulación. |
| Rich Presence y leaderboards | Permanecen gestionados por `rc_client`; no existe opción para desactivarlos en Hardcore. |
| Identidad del cliente | User-Agent propio y versionado: `NGNEON-EMU/1.2.3` con cláusula `rcheevos`. |
| Indicador de modo | El título de ventana muestra permanentemente `RA:HARDCORE` o `RA:CASUAL`; al cargar un juego también aparece una notificación. |
| Sincronización del modo | La interfaz vuelve a consultar el estado real de `rc_client` si RetroAchievements desactiva Hardcore para una sesión. |

## Trabajo obligatorio antes de solicitar aprobación

1. **Lista completa de logros dentro del emulador.** La pestaña actual muestra
   totales y logros recientes, pero falta una lista navegable con título,
   descripción, puntos, estado, progreso medido y estado Trigger/Primed.
2. **Indicadores de gameplay.** Deben mostrarse los eventos de challenge,
   progreso medido, leaderboard tracker y scoreboard emitidos por `rc_client`.
3. **Cola offline persistente.** `rcheevos` reintenta los envíos durante la
   sesión actual, pero todavía no existe una caché cifrada y recuperable tras
   cerrar el emulador.
4. **Pruebas de compatibilidad de saves.** La Memory Card usa 2 KiB en bruto,
   pero debe documentarse y validarse formalmente contra FBNeo/Geolith.
5. **Avisos de terceros y privacidad.** Antes de solicitar aprobación hay que
   publicar las licencias y enlaces de cada componente distribuido. La política
   de privacidad debe describir con datos reales los servicios utilizados,
   retención y países; no debe publicarse una plantilla con marcadores.
6. **Validación pública.** El proyecto debe cumplir el periodo público mínimo
   exigido por RetroAchievements y cada release debe ser aprobado como cliente
   Hardcore.

## Mejoras opcionales recomendadas

- Integrar RAIntegration DLL en Windows para facilitar tickets y depuración de
  logros.
- Añadir pruebas automáticas que recorran todos los accesos a save states,
  selección de BIOS y cambios de modo desde teclado, mando y configurador.
- Centralizar la versión del cliente para que binario, User-Agent, documentación
  y releases nunca diverjan.
