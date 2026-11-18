# 🎧 Melodyfy Rust — Reproductor de Música Libre, Rápido y Sin Anuncios

<p align="center">
  <img src="assets/logo.jpg" alt="Melodyfy Rust Official Logo" width="220" style="border-radius: 18px;" />
</p>

[![Rust](https://img.shields.io/badge/Language-Rust_2021-DEA584.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![GUI](https://img.shields.io/badge/GUI-egui_%2F_eframe-1ED760.svg)](https://github.com/emilk/egui)
[![Audio](https://img.shields.io/badge/Audio-Direct_Hardware_COM-0078D7.svg)](https://learn.microsoft.com/windows)
[![Ads](https://img.shields.io/badge/Ads-0%25_Zero_Ads-brightgreen.svg)](#)
[![RAM](https://img.shields.io/badge/RAM_Usage-%3C25MB_vs_600MB_Spotify-success.svg)](#)

**Melodyfy Rust** es un reproductor de música en streaming nativo para Windows desarrollado en **Rust**. Diseñado para ofrecer una alternativa superior a Spotify: **100% gratis, sin anuncios, sin telemetría pesada y consumiendo menos de 25 MB de memoria RAM**.

---

## ⚡ ¿Por qué es mejor que Spotify?

| Característica | Spotify Desktop | Melodyfy Rust |
| :--- | :--- | :--- |
| **Consumo de Memoria RAM** | ~600 MB – 1 GB (Chromium / Electron) | **~20 – 25 MB** (Nativo Rust + GPU) |
| **Tiempo de Arranque** | 3 – 6 segundos | **< 100 ms (Instantáneo)** |
| **Anuncios comerciales** | Interrupciones constantes en cuenta free | **0 anuncios (100% libre)** |
| **Catálogo disponible** | Limitado por acuerdos discográficos | **Millones de canciones, directos, remixes y podcasts (vía YouTube Music)** |
| **Suscripción de pago** | ~$10.99 / mes para no oír publicidad | **Completamente gratis para siempre** |
| **Salto de canciones** | Limitado a 6 skips/hora en free | **Saltos y retrocesos ilimitados** |
| **Modo Mini-Player** | Solo con ciertas cuentas o widgets | **Integrado con 1 clic (`🕹️`)** |

---

## ✨ Características Principales

* 🚀 **Streaming Directo Híbrido (InnerTube + `yt-dlp`):** Búsqueda y resolución de audio nativa en Rust puro (~120–250 ms) con caché persistente y pre-carga automática (*0ms Gapless Prefetch*).
* 🎨 **Interfaz Moderna Estilo Spotify (Escritorio + Móvil 9:16):**
  * Paleta oscura editorial (`#121212`, `#181818`) con acentos en Verde Spotify (`#1ED760`) o Ámbar Clásico (`#E87A30`) alternables con un clic.
  * Modo **Smartphone 9:16 (`📱 Modo Móvil`)** integrado y ejecutable dedicado (`Melodyfy-Mobile.exe`).
  * Barra lateral con acceso a: *Inicio*, *Buscar*, *Tus Me Gusta*, *Mis Playlists*, *Descargas Offline*, *Historial*, *Cola Actual* y *Letras Karaoke*.
* 📂 **Gestor Completo de Playlists Personalizadas:** Crea tus propias listas de reproducción, añade o quita canciones desde cualquier tabla con el menú `📂` y reproduce listas enteras con un clic.
* 💾 **Descargas Offline sin DRM (`.m4a`):** Descarga cualquier pista a la carpeta local `downloads/` y escúchala sin conexión a internet desde la pestaña *Descargas Offline*.
* 🎤 **Letras Sincronizadas (Modo Karaoke LRCLIB):** Letras en tiempo real con desplazamiento automático, visualizador de espectro de 24 barras, salto por clic en cada verso y ajuste fino de sincronización (`-0.5s` / `+0.5s`).
* ⚡ **Efectos de Velocidad DSP Nativos:** Alterna en tiempo real desde la barra inferior entre `1.0x Normal`, `1.25x Nightcore ⚡`, `1.5x Fast ⏩` y `0.85x Slowed Chill 🌙`.
* 📻 **Radio Infinita (Auto-Mix):** Cuando termina tu cola de reproducción, busca y añade automáticamente canciones similares para que la música nunca se detenga.
* 🌙 **Temporizador de Sueño (Sleep Timer):** Programa el apagado automático de la reproducción en `15m`, `30m`, `45m` o `60m`.
* 🕹️ **Modo Mini-Player Flotante (`Always on Top`):** Alterna a una ventana compacta y siempre visible para escuchar música sin distracciones mientras juegas o programas.
* ⌨️ **Atajos de Teclado:**
  * `Espacio`: Pausar / Reanudar.
  * `Ctrl + Derecha`: Siguiente canción.
  * `Ctrl + Izquierda`: Canción anterior o reiniciar pista.
  * `Ctrl + Arriba / Abajo`: Subir / Bajar volumen.
  * `M`: Silenciar (*Mute*) / Restaurar volumen.

---

## 🏗️ Arquitectura Técnica

1. **GUI (egui / eframe):** Interfaz en modo inmediato acelerada por hardware (DirectX/OpenGL/WGPU), garantizando 60+ FPS fluidos sin sobrecargar la CPU.
2. **Motor de Audio (Win32 COM IDispatch):** Decodificación y renderizado de audio nativo mediante Windows Media Foundation a nivel de kernel/user-mode, asegurando bajo consumo de CPU (0%) y soporte universal de códecs AAC/M4A/MP3.
3. **Streamer Asíncrono (`yt-dlp`):** Extracción multihilo sin bloquear la interfaz de usuario, con caché de URLs activas y descarga rápida de metadatos.

---

## 🚀 Cómo Ejecutar

### Opción 1: Ejecutable directo (Listo para usar)
Simplemente haz doble clic en:
```text
Melodyfy-Rust.exe
```
O ejecuta:
```cmd
iniciar_melodyfy_rust.bat
```

### Opción 2: Compilar desde código fuente (Rust)
Requisitos: Tener instalado [Rust y Cargo](https://rustup.rs/).
```cmd
build_rust.bat
```
O directamente con Cargo:
```cmd
cd melodyfy-rust
cargo run --release
```

---

## 📁 Estructura del Proyecto

```text
lite-music-player/
│
├── Melodyfy-Rust.exe         # Ejecutable nativo compilado en Rust (6.8 MB)
├── iniciar_melodyfy_rust.bat # Lanzador rápido del reproductor Rust
├── build_rust.bat            # Script de compilación release
├── yt-dlp.exe                # Motor de extracción de audio y metadatos
├── library.json              # Base de datos local (favoritos, historial, volumen)
│
├── melodyfy-rust/            # Código fuente completo en Rust
│   ├── Cargo.toml            # Dependencias y optimizaciones LTO
│   └── src/
│       ├── main.rs           # Punto de entrada y configuración de ventana
│       ├── app.rs            # Interfaz gráfica egui (Spotify Dark, cola, eventos)
│       ├── audio.rs          # Motor de audio nativo COM Win32 IDispatch
│       ├── models.rs         # Modelos de datos (Track, Library, RepeatMode)
│       └── streamer.rs       # Resolver asíncrono y caché de streams
│
├── Melodyfy.cs               # Versión legacy en C# WinForms
└── README.md                 # Esta documentación
```
