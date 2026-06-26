# Limitación actual: errores de memoria en r68k

## Problema

El crate `r68k` (versión 0.2.2) provoca un `panic!` interno cuando ocurre un `AddressError` (acceso inválido de memoria) durante la ejecución de instrucciones, debido a un `unwrap()` sobre un `Result::Err` en su código fuente (`src/cpu/mod.rs:1113`).

Esto impide que el emulador reciba errores estructurados desde `r68k`. El frontend actual mitiga el problema capturando el `panic` con `catch_unwind`, pausando la emulación y manteniendo la ventana abierta, pero sigue siendo un workaround porque el crate no expone un `Result` recuperable.

## Consecuencias
- La CPU emulada no puede continuar con garantías tras un acceso inválido.
- La aplicación puede mostrar un mensaje amigable y mantener el frontend activo, pero no puede recuperar la CPU sin reiniciar/cargar ROM de nuevo.

## Workarounds sugeridos
1. **Fork temporal del crate r68k:**
   - Modificar el código fuente para propagar los errores como `Result` en vez de hacer `unwrap()`.
   - Usar el fork como dependencia temporal en el proyecto.
2. **Contribuir un PR al upstream:**
   - Proponer al autor del crate que exponga una API que devuelva `Result` en vez de hacer `unwrap()`.
   - Documentar el caso de uso de emuladores y la necesidad de control robusto de errores.
3. **Validar rangos de acceso en el bus:**
   - Implementar validaciones adicionales en el bus de memoria para minimizar los accesos inválidos, aunque esto no elimina el problema de raíz.

## Referencias
- [r68k en crates.io](https://crates.io/crates/r68k)
- [Ejemplo de panic en AddressError](https://github.com/ivmarkov/r68k/blob/main/src/cpu/mod.rs#L1113)

---

**Estado:**
- El emulador captura el fallo, pausa la emulación y mantiene el frontend activo.
- La solución definitiva sigue siendo usar un fork/parche de `r68k` o cambiar a un core 68000 que devuelva errores estructurados.
