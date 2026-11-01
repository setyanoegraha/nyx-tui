# nyx-tui

### Dashboard de terminal no oficial para VulNyx

<p align="center">
  <img src="assets/dashboard-machines-es.png" alt="nyx-tui — pestaña Machines con el tema Nord" width="100%">
</p>

**[English](README.md) | [Español](README.es.md)**

---

**nyx-tui** es un dashboard interactivo de terminal para [VulNyx](https://vulnyx.com): explora el catálogo de máquinas, envía flags first-blood, lee y publica writeups de la comunidad y sigue tu posición en el leaderboard — todo sin salir de la terminal.

Las descargas se completan de principio a fin en la terminal. Pulsa `d` y nyx pide la página de descarga, lee la imagen CAPTCHA por sí mismo, envía el código, resuelve el enlace de Proton Drive y descarga el archivo en tu carpeta con una línea de progreso en vivo. Sin pasar por el navegador y sin escribir el código a mano.

Un comando, una pantalla: al ejecutar `nyx` se abre el dashboard. Escrito en **Rust** puro (ratatui), distribuido como un único binario estático. Tema Nord. Toda la interfaz está en **inglés o español**, cambiable con `l`. **No se necesita cuenta** — solo un nombre de usuario.

---

## Capturas de pantalla

| Machines | Progress |
| :---: | :---: |
| ![Pestaña Machines: catálogo con colores de dificultad, etiquetas y estado del first-blood](assets/dashboard-machines-es.png) | ![Pestaña Progress: identidad, estadísticas, tus writeups publicados y la tabla hechas — sin writeup con el cursor dentro](assets/dashboard-progress-done-es.png) |
| **Detalles de la máquina** | **Writeups de la comunidad** |
| ![Ventana de detalles: dificultad, plataforma, MD5, etiquetas y poseedores del first-blood](assets/popup-details-es.png) | ![Ventana de writeups: artículos y vídeos con autor, fecha y URL](assets/popup-writeups-es.png) |
| **Atajos de teclado** | **Carpeta de descargas** |
| ![Panel de atajos con secciones de navegación, máquinas, progreso y ventanas](assets/popup-help-es.png) | ![Ventana de carpeta de descargas con una ruta absoluta](assets/popup-download-dir-es.png) |
| **First blood** | **Interfaz en inglés** |
| ![Informe de first blood: ambos huecos ya tomados, con sus poseedores](assets/report-firstblood-es.png) | ![Interfaz en inglés en la pestaña Machines](assets/dashboard-machines.png) |

## Características

* **Un comando** — `nyx` abre el dashboard: catálogo, flags first-blood, writeups y tu posición en el leaderboard en una sola pantalla.
* **Machines** — el catálogo completo con los colores oficiales de dificultad del sitio (Low/Easy/Medium/Hard), SO (Linux/Windows), etiquetas de tecnologías y estado de first-blood por máquina. Filtrado instantáneo con `/` y ordenamiento con `s` (orden del sitio → nombre → fecha → dificultad).
* **Descargas desde la propia terminal** — `d` resuelve el CAPTCHA y descarga el archivo de la máquina desde Proton Drive en tu carpeta, con velocidad y tamaño en el pie. Un intento por pulsación; el límite de la plataforma se informa, nunca se reintenta.
* **Envío de flags first-blood** — `f` abre una ventana para enviar las flags User y Root (MD5). Los huecos ya ocupados se muestran como avisos de solo lectura; los huecos libres quedan listos para tu hash MD5. Tu nombre de usuario se adjunta automáticamente.
* **Filtro first-blood** — `b` muestra solo las máquinas con algún hueco de first-blood libre, para que seas el primero en completar un lanzamiento nuevo.
* **Writeups** — ventana de writeups de la comunidad por máquina (`w`): artículos 📝 y vídeos 🎥 con autor, idioma y fecha. Publica el tuyo (`u`) — la ventana de envío usa selectores para el **tipo** (Text/Video, selección única) y el **idioma** (los 62 códigos de vulnyx.com, multi-selección), así no se escribe nada a mano.
* **Seguimiento de completadas** — las máquinas que terminas se marcan `✓` en verde: automático cuando uno de tus writeups se publica en el sitio, o manual por máquina con `m` (pide confirmación y se guarda localmente). `x` oculta las completadas para quedarte solo con las pendientes.
* **Progress** — tus first bloods, writeups y posición en el leaderboard (calculada a partir de los datos públicos con las reglas de puntuación del propio sitio, incluidas las exclusiones de staff del sitio, para que el puesto coincida con vulnyx.com).
* **Gestión del nombre de usuario** — `a` abre la ventana de usuario; el nombre que pongas se adjunta a todos los envíos y se usa para calcular tu posición en el leaderboard.
* **Idioma de la interfaz** — `l` alterna toda la interfaz entre **English** y **Español** al instante (mensajes de estado, ventanas, ayudas, todo). Tu elección se guarda y se restaura en cada arranque.
* **Referencia de atajos** — `h` abre un panel desplazable con todas las teclas, agrupadas por contexto.

---

## Requisitos

* **SO**: Linux (desarrollado y probado en Arch Linux); se publican binarios para macOS y Windows, tanto x86_64 como arm64.
* **No** se necesita cuenta de [VulNyx](https://vulnyx.com) — la plataforma funciona con un nombre de usuario declarado por ti.

---

## Instalación

### 1. Desde un binario de release (lo más fácil)

Descarga el archivo de tu plataforma desde la página de [Releases](https://github.com/setyanoegraha/nyx-tui/releases):

| Plataforma | Archivo |
| :--- | :--- |
| Linux x86_64 | `nyx-v0.3.2-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 | `nyx-v0.3.2-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `nyx-v0.3.2-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `nyx-v0.3.2-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `nyx-v0.3.2-x86_64-pc-windows-msvc.zip` |
| Windows arm64 | `nyx-v0.3.2-aarch64-pc-windows-msvc.zip` |

```bash
tar xzf nyx-v0.3.2-x86_64-unknown-linux-gnu.tar.gz
install -m 755 nyx ~/.local/bin/nyx
```

### 2. Desde el código fuente

```bash
git clone https://github.com/setyanoegraha/nyx-tui.git
cd nyx-tui
cargo install --path .
```

### 3. Directamente desde git

```bash
cargo install --git https://github.com/setyanoegraha/nyx-tui.git
```

> Requiere la toolchain de Rust (1.96+): https://rustup.rs

---

## Primer arranque

Simplemente ejecuta:

```bash
nyx
```

El dashboard carga de inmediato — sin ventana de login. Pulsa `a` para poner tu **nombre de usuario** (p. ej. `noneofyour`). Ese nombre se adjunta a todos los envíos de flags y writeups y se usa para calcular tu posición en el leaderboard. Puedes cambiarlo en cualquier momento.

### Línea de comandos

```
nyx            abrir el dashboard interactivo
nyx --version  mostrar la versión
nyx --help     mostrar la ayuda corta
```

Todo lo demás vive dentro del dashboard.

---

## Guía de uso

Dos pestañas manejadas por teclado — **Machines** y **Progress**:

| Teclas | Acción |
| :--- | :--- |
| `Tab` / `←` `→` | Cambiar de pestaña |
| `↑` `↓` / `j` `k` | Mover la selección |
| `g` / `Home` | Ir al principio de la lista |
| `/` | Filtrar la lista actual (escribe para acotar, `Enter` confirma, `Esc` limpia y sale) |
| `s` | **Machines** — ciclar orden: orden del sitio → nombre → fecha → dificultad |
| `b` | **Machines** — mostrar solo máquinas con hueco de first-blood libre |
| `m` | **Machines** — marcar/desmarcar la máquina como completada (se guarda localmente); pide confirmación, con un texto distinto si ya llevaba la marca |
| `D` | **En cualquier lugar** — carpeta de descargas; la ruta debe ser absoluta (`~` se expande) |
| `h` | **En cualquier lugar** — panel completo de atajos, desplazable con `j`/`k` |
| `x` | **Machines** — ocultar/mostrar las máquinas completadas |
| `d` | **Machines** — resolver el CAPTCHA de descarga y descargar el archivo desde Proton Drive |
| `f` | **Machines** — ventana de flags first-blood: enviar flags User y/o Root (MD5). Los huecos ocupados se muestran como avisos de solo lectura |
| `w` | **Machines** — ventana de writeups de la comunidad de la máquina seleccionada: `j`/`k` para elegir, `Enter` abre el enlace |
| `u` | **Machines** — enviar una URL de writeup para la máquina seleccionada (pendiente de revisión de un admin) |
| `i` / `Enter` | **Machines** — ventana de descripción con etiquetas, plataformas, MD5 y poseedores del first-blood |
| `a` | **En cualquier pestaña** — ventana de nombre de usuario |
| `l` | **En cualquier pestaña** — alternar idioma EN/ES (se guarda) |
| `Enter` | **Progress** — abrir el writeup seleccionado en tu navegador (solo filas de writeup) |
| `r` | Volver a obtener todos los datos |
| `q` / `Esc` / `Ctrl-C` | Salir |

### Descargas

VulnyX protege la descarga de máquinas con una imagen CAPTCHA de 5 caracteres. nyx-tui lee la imagen por sí mismo:

1. Pulsa `d` sobre una máquina — nyx pide la página de descarga y el CAPTCHA en una sola sesión, decodifica
   la imagen con el alfabeto fijo de 30 caracteres del sitio (`A-Z` sin `I`, `O`, `S`; dígitos sin
   `0`, `1`, `5`) y envía el código.
2. Si acierta, resuelve el enlace de Proton Drive y descarga el archivo en tu directorio de descargas,
   mostrando el progreso en el pie (`↓ APex · APex.zip · 188.0 MB/1.2 GB · 3.6 MB/s`). El directorio
   por defecto es `~/Downloads/nyx`; cámbialo con `D` (solo rutas absolutas) o editando `"download_dir"` en
   `~/.nyx-tui/config.json`.
   Un archivo que ya está en disco con el tamaño correcto no se vuelve a descargar.
3. Si no puede leer la imagen (el sitio cambió la fuente) o Proton rechaza la transferencia, abre el
   enlace en tu navegador para que continúes a mano.

nyx permite **como máximo dos transferencias simultáneas**; un tercer `d` se rechaza con un aviso hasta que termine una. La plataforma permite **2 descargas por minuto**; nyx hace un único intento por pulsación de `d` e informa
del límite en vez de reintentar. Las transferencias parciales usan un archivo `.part` que se descarta al
fallar o al salir, así que no hay reanudación. Salir con transferencias activas abre una confirmación:
`Enter` cancela todo y sale, `Esc` las mantiene en marcha.

### Dónde viven tus datos

- `~/.nyx-tui/config.json` — tu **nombre de usuario**, idioma de la interfaz, carpeta de descargas y las máquinas marcadas como completadas localmente. Nada más.
- No se guarda ninguna contraseña — VulNyx no tiene cuentas y el nombre de usuario lo declara cada uno.

---

## Actualizar

```bash
cargo install --git https://github.com/setyanoegraha/nyx-tui.git --force
```

o descarga el último binario de la página de [Releases](https://github.com/setyanoegraha/nyx-tui/releases).

### Desinstalación y limpieza

```bash
cargo uninstall nyx
```

Borra `~/.nyx-tui/` para eliminar todos los datos locales.

---

## Aviso

nyx-tui es una herramienta comunitaria **no oficial** y no está afiliada a VulNyx. Solo usa los datos JSON públicos de la plataforma y completa el mismo CAPTCHA que la aplicación web, una descarga a la vez — sé amable con el servicio.

---

## Agradecimientos

Gracias al equipo y la comunidad de VulNyx por la plataforma, y al equipo de [ratatui](https://github.com/ratatui/ratatui) por el toolkit.

Proyectos hermanos:
- [hmv-tui](https://github.com/setyanoegraha/hmv-tui) — el mismo concepto de dashboard para HackMyVM
- [dl-tui](https://github.com/setyanoegraha/dl-tui) — el mismo concepto de dashboard para DockerLabs

---

Hecho con ❤️ por [Ouba](https://github.com/setyanoegraha).

*¡Happy hacking en VulNyx!*
