# Guía de Compilación APK para Android: Melodyfy Mobile

Este documento detalla los pasos para empaquetar **Melodyfy Mobile** en un archivo `.apk` instalable en teléfonos Android.

---

## 1. Requisitos Previos

- **Java JDK 21:** Ya instalado en el sistema (`Temurin-21.0.12.1`).
- **Rust Android Target:**
  ```powershell
  rustup target add aarch64-linux-android
  rustup target add armv7-linux-androideabi
  ```
- **Android SDK & NDK:**
  Descargar [Android Command Line Tools](https://developer.android.com/studio#command-tools) o instalar **Android Studio** para disponer de `sdkmanager` y NDK 26+.

---

## 2. Métodos de Empaquetado

### Opción A: Empaquetado Directo con `cargo-apk` (Recomendado)
`cargo-apk` compila el código Rust con `android-activity` y genera el `.apk` directamente sin necesidad de abrir Android Studio:
```powershell
cargo install cargo-apk
cargo apk build --release
```
El archivo final se ubicará en `target/release/apk/melodyfy-rust.apk`.

### Opción B: Wrapper Tauri Mobile v2
```powershell
npm create tauri-app@latest
# Seleccionar Rust y plataforma Android
cargo tauri android build
```

---

## 3. Características de la Versión Móvil Creada

1. **Resolución Vertical 9:16:** Calibrada a 390×820 px (ratio estándar de smartphones modernos).
2. **Barra de Navegación Inferior (Bottom Nav Bar):**
   - 🏠 **Inicio:** Chips de géneros, cuadrícula de mixes curados y lista recomendada.
   - 🔍 **Buscar:** Barra de búsqueda táctil con sugerencias e historial.
   - 📚 **Biblioteca:** Pestañas directas para Canciones Favoritas (`💚`), Historial y Cola.
3. **Sticky Mini-Player:**
   - Tarjeta flotante táctil sobre la barra inferior con carátula 40×40, título, artista, botón me gusta y play/pausa.
   - Barra de progreso delgada de 2 px en el borde inferior.
4. **Reproductor Completo Móvil (*Now Playing / BottomSheet*):**
   - Se abre al tocar el mini-player.
   - Carátula gigante 280×280 px con esquinas redondeadas.
   - Barra de búsqueda táctil amplia con perilla circular táctil.
   - Botón Play/Pausa circular gigante de 58×58 px en verde Spotify.
   - Herramientas: Slider de volumen y botón de cola.
5. **Conmutador Bidireccional PC <-> Móvil:**
   - Botón `💻 PC` en la cabecera móvil para volver a 1150×740 px.
   - Botón `📱 Modo Móvil` en la barra lateral de PC para pasar a 390×820 px.
