# Compilar NGNEON-EMU para Windows y Linux

Esta guía cubre las builds de 64 bits soportadas por el proyecto:

| Plataforma | Arquitectura | Target de Rust | Salida recomendada |
|---|---|---|---|
| Windows | x64 | `x86_64-pc-windows-msvc` | `NGNEON-EMU-windows-x64/` |
| Windows | ARM64 | `aarch64-pc-windows-msvc` | `NGNEON-EMU-windows-arm64/` |
| Linux | x64 | `x86_64-unknown-linux-gnu` | `NGNEON-EMU-linux-x86_64/` |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` | `NGNEON-EMU-linux-aarch64/` |

Los cuatro targets tienen soporte oficial de Rust. La build Windows x64 es la
validada habitualmente en este repositorio. Los flujos ARM64 requieren todavía
una comprobación final en hardware o una máquina virtual ARM64 real antes de
publicar una release.

Los empaquetadores de NGNEON-EMU compilan para la arquitectura **nativa del
host**. No aceptan un target de cross-compilación y no deben ejecutarse después
de un `cargo build --target ...`, porque volverían a compilar para el host.

## Requisitos comunes

1. Instala Rust estable mediante `rustup`.
2. Comprueba el host activo:

   ```text
   rustc -vV
   rustup target list --installed
   ```

3. La línea `host:` debe coincidir con el target de la build nativa.
4. Todas las dependencias nativas deben tener la misma arquitectura que el
   ejecutable. No se puede mezclar, por ejemplo, un `ngneon-emu.exe` ARM64 con
   una `SDL2.dll` x64.

Antes de empaquetar una release ejecuta:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

## Windows x64

### Requisitos

- Windows 10 o posterior.
- Rust MSVC para `x86_64-pc-windows-msvc`.
- Visual Studio 2022 Build Tools con:
  - Desktop development with C++;
  - herramientas MSVC x64/x86;
  - Windows 10 u 11 SDK.
- SDL2 2.x de desarrollo para x64.

El proyecto enlaza SDL2 dinámicamente. Coloca estos dos archivos x64 en la
raíz del repositorio:

```text
SDL2.lib
SDL2.dll
```

### Compilar y empaquetar

Abre Developer PowerShell for VS 2022 en x64:

```powershell
rustup target add x86_64-pc-windows-msvc
rustc -vV
cargo build --release --workspace
.\package_ngneon_release.bat ".\NGNEON-EMU-windows-x64"
```

Los ejecutables sin empaquetar quedan en `target/release/`. El empaquetador
copia `SDL2.dll` y busca `VCRUNTIME140.dll` en `target/release/`, en la raíz del
proyecto o en `%SystemRoot%\System32`.

Comprueba la arquitectura desde la consola de Visual Studio:

```powershell
dumpbin /headers .\NGNEON-EMU-windows-x64\ngneon-emu.exe |
  Select-String 'machine'
dumpbin /headers .\NGNEON-EMU-windows-x64\SDL2.dll |
  Select-String 'machine'
.\NGNEON-EMU-windows-x64\ngneon-emu.exe --help
```

El valor PE esperado para x64 es `8664`.

## Windows ARM64

### Requisitos

- Windows 11 ARM64 recomendado.
- Toolchain nativo `aarch64-pc-windows-msvc`.
- Visual Studio 2022 Build Tools con MSVC ARM64 y Windows SDK.
- `SDL2.lib` y `SDL2.dll` compilados para ARM64.
- `VCRUNTIME140.dll` ARM64.

La `SDL2.dll` x64 incluida actualmente en el entorno de desarrollo no sirve
para ARM64. Sustituye temporalmente los archivos SDL2 de la raíz por sus
variantes ARM64 o realiza la build desde una copia limpia del repositorio que
contenga las dependencias ARM64. No publiques un paquete sin verificar también
la arquitectura de las DLL.

### Compilar y empaquetar nativamente

En Developer PowerShell for VS 2022 para ARM64:

```powershell
rustup target add aarch64-pc-windows-msvc
rustc -vV
cargo build --release --workspace
.\package_ngneon_release.bat ".\NGNEON-EMU-windows-arm64"
```

`rustc -vV` debe mostrar `host: aarch64-pc-windows-msvc`. Verifica el paquete:

```powershell
dumpbin /headers .\NGNEON-EMU-windows-arm64\ngneon-emu.exe |
  Select-String 'machine'
dumpbin /headers .\NGNEON-EMU-windows-arm64\SDL2.dll |
  Select-String 'machine'
.\NGNEON-EMU-windows-arm64\ngneon-emu.exe --help
```

El valor PE esperado para ARM64 es `AA64`.

## Linux x64

### Dependencias

Debian o Ubuntu:

```sh
sudo apt update
sudo apt install build-essential pkg-config libsdl2-dev libasound2-dev libgl1-mesa-dev libssl-dev
```

Fedora:

```sh
sudo dnf install @development-tools pkgconf-pkg-config SDL2-devel alsa-lib-devel mesa-libGL-devel openssl-devel
```

Arch Linux:

```sh
sudo pacman -S --needed base-devel pkgconf sdl2 alsa-lib mesa openssl
```

### Compilar y empaquetar

En un sistema cuyo `uname -m` sea `x86_64`:

```sh
rustup target add x86_64-unknown-linux-gnu
rustc -vV
chmod +x iniciar-ngneon.sh configurar-ngneon.sh package_ngneon_release.sh
cargo build --release --workspace
NGNEON_PACKAGE_OUT=NGNEON-EMU-linux-x86_64 ./package_ngneon_release.sh
```

Verificación mínima:

```sh
file NGNEON-EMU-linux-x86_64/ngneon-emu
ldd NGNEON-EMU-linux-x86_64/ngneon-emu
NGNEON-EMU-linux-x86_64/ngneon-emu --help
```

`file` debe indicar `x86-64`. `ldd` no debe mostrar dependencias `not found`.

## Linux ARM64

Instala las mismas dependencias desde los repositorios ARM64 de la distribución
y compila en un host cuyo `uname -m` sea `aarch64`:

```sh
rustup target add aarch64-unknown-linux-gnu
rustc -vV
chmod +x iniciar-ngneon.sh configurar-ngneon.sh package_ngneon_release.sh
cargo build --release --workspace
NGNEON_PACKAGE_OUT=NGNEON-EMU-linux-aarch64 ./package_ngneon_release.sh
```

Verificación mínima:

```sh
file NGNEON-EMU-linux-aarch64/ngneon-emu
ldd NGNEON-EMU-linux-aarch64/ngneon-emu
NGNEON-EMU-linux-aarch64/ngneon-emu --help
```

`rustc -vV` debe mostrar `host: aarch64-unknown-linux-gnu`, `file` debe indicar
`ARM aarch64` y `ldd` no debe mostrar dependencias ausentes.

## Compilación cruzada

`rustup target add` instala la biblioteca estándar de Rust del target, pero no
instala el linker, el SDK ni las bibliotecas C de SDL2, ALSA y OpenGL. Por ello,
la vía de release recomendada es usar un host o runner nativo por arquitectura.

La compilación cruzada puede prepararse manualmente con:

- el target Rust correspondiente;
- un linker y sysroot del destino;
- SDL2 y el resto de dependencias compiladas para el destino;
- configuración de Cargo y `pkg-config` para ese sysroot.

En ese caso Cargo deja los binarios en
`target/<target-triple>/release/`. Los scripts `package_ngneon_release.bat` y
`package_ngneon_release.sh` no empaquetan actualmente ese directorio. Ensambla
el paquete manualmente o, preferiblemente, ejecuta el empaquetador en un runner
nativo.

Referencias oficiales:

- [Plataformas soportadas por Rust](https://doc.rust-lang.org/rustc/platform-support.html)
- [Targets Windows MSVC](https://doc.rust-lang.org/stable/rustc/platform-support/windows-msvc.html)
- [Target Linux ARM64](https://doc.rust-lang.org/rustc/platform-support/aarch64-unknown-linux-gnu.html)
- [Cross-compilación con rustup](https://rust-lang.github.io/rustup/cross-compilation.html)

## Contenido y seguridad de los paquetes

- Los empaquetadores no incluyen ROMs ni partidas.
- Copian la carpeta `bios/` local si ya existe, igual que en las releases
  anteriores; revisa su contenido antes de distribuir el paquete.
- Generan una configuración portátil sin credenciales RetroAchievements.
- Se detienen si la salida contiene datos del usuario o configuración
  modificada. `NGNEON_ALLOW_PACKAGE_CLEAN=1` solo debe usarse cuando se desea
  reemplazar conscientemente esa salida.
- Linux enlaza SDL2, ALSA y OpenGL desde el sistema y no incluye bibliotecas
  `.so` de la distribución.
