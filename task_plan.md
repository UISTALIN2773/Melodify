# Task Plan: Versión Móvil de Melodyfy (Spotify Mobile Rust)

## Objetivo
Crear una versión móvil nativa y completa de Melodyfy basada en la versión actual:
1. **Motor InnerTube en Rust Puro (`src/innertube.rs`):** Extracción de streaming de audio directa vía HTTP (sin dependencia de `yt-dlp.exe` de Python, latencia ~120ms).
2. **Arquitectura y UI Spotify Mobile (9:16 Portrait):**
   - Viewport vertical calibrado (390×820 px).
   - Barra de navegación inferior móvil fija (🏠 Inicio, 🔍 Buscar, 📚 Tu Biblioteca).
   - Mini-Player flotante sticky sobre la barra de navegación con toque para expandir.
   - Pantalla completa de reproducción móvil (*BottomSheet / Fullscreen Player*): Carátula grande 280×280, tipografía móvil, barra de progreso táctil completa, botones grandes y controles de audio.
3. **Binario Dedicado Móvil:** `Melodyfy-Mobile.exe` configurado en resolución smartphone, conmutador en tiempo real entre modo escritorio y móvil, y lanzador `iniciar_melodyfy_mobile.bat`.
4. **Soporte Android NDK / Cargo-APK:** Configuración de `Cargo.toml` y manifiesto para compilación cruzada hacia APK.

## Fases
- [x] Fase 1: Implementar módulo InnerTube en Rust (`src/innertube.rs`) para extracción autónoma sin Python.
- [x] Fase 2: Implementar la UI completa de Spotify Mobile en `src/app.rs` (Bottom Nav, Sticky Mini-player, Fullscreen Now-Playing View).
- [x] Fase 3: Agregar binario dedicado `melodyfy-mobile` y comando de arranque `--mobile` en `main.rs`.
- [x] Fase 4: Compilar `Melodyfy-Mobile.exe` y `Melodyfy-Rust.exe`, probar navegación y verificar que no haya errores.
- [x] Fase 5: Crear lanzador `.bat` y documentar en `findings.md` y `progress.md`.

## Fase 6: Letras Sincronizadas (Karaoke), Búsqueda Nativa Rust & Descarga Offline
- [x] Motor de letras sincronizadas con LRCLIB (`src/lyrics.rs`) con parsing LRC y scroll reactivo interactivo.
- [x] Búsqueda nativa en Rust puro a través de InnerTube (`src/innertube.rs`) sin subprocesos de Python.
- [x] Motor de descarga directa de audio sin DRM en formato M4A/AAC (`src/streamer.rs`).
- [x] Integración de vista `ViewTab::Lyrics` y controles `🎤` y `⬇` en barra inferior y móvil.
- [x] Compilación release optimizada y despliegue a la raíz del repositorio.
