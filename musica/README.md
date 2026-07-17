# Música del frontend

Copia en esta carpeta los ficheros `.mp3` que quieras escuchar en
`ngneon-frontend`.

- Las canciones se reproducen en orden aleatorio.
- Se completa una vuelta por todas las pistas antes de repetir.
- La opción **Ajustes → Música del frontend** activa o desactiva la reproducción.
- `Ctrl+N` salta a la siguiente canción aleatoria.
- Al lanzar un juego, la pista se pausa.
- Al cerrar el emulador, continúa automáticamente desde la misma posición.
- Los MP3 que no se puedan abrir o decodificar se omiten sin detener el frontend.

Los subdirectorios no se recorren: coloca los `.mp3` directamente en
`musica/`.

La preferencia se guarda en `config/ngneon.conf`:

```ini
frontend_music=on
```

Valores válidos: `on` y `off`. Si la clave no existe, el valor predeterminado
es `on`. El atajo `Ctrl+N` funciona tanto en la biblioteca como en Ajustes; si
la música está desactivada o no hay MP3, el frontend muestra un mensaje temporal.
