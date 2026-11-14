# Task Plan: Melodyfy Rust - High-Performance Spotify-Alternative Music Player

## 1. Goal
Build a blazing-fast, free, ad-free desktop music player in Rust that outperforms Spotify in memory footprint (<25 MB RAM vs 600+ MB), startup time (<100 ms), and user freedom, streaming high quality audio from YouTube.

## 2. Architecture & Components
- **Language & Runtime:** Rust (2021 edition), native Windows MSVC build.
- **GUI Engine:** `eframe` / `egui` (Immediate-mode native GUI, hardware accelerated, customizable theme, ultra-low resource usage).
- **Audio Engine:** `WmpAudioEngine` COM direct hardware acceleration backend in Rust (IDispatch Win32 COM, hardware decoding, 0% CPU, pause/seek/volume/loop).
- **Music Resolver & Streamer:** Asynchronous worker integrating `yt-dlp` for metadata, streaming URLs, thumbnails, and cache management.
- **Features:**
  - Real-time search (songs, artists, albums, playlists).
  - Queue management (Next, Previous, Loop/Repeat, Shuffle).
  - Local favorites & history persistence (`library.json`).
  - Sleek modern UI (Spotify Dark style, interactive seekbar, volume slider).
  - Hotkeys (Space to toggle, arrows to seek/skip).

## 3. Milestones
- [x] Phase 1: Environment & dependency verification (Rust, yt-dlp, cargo).
- [x] Phase 2: Audio engine & COM WMP stream resolver verification.
- [x] Phase 3: Core data structures (Track, Playlist, AppLibrary, RepeatMode).
- [x] Phase 4: UI layout & modern styling (Sidebar, Search, TrackList, PlayerBar).
- [x] Phase 5: Asynchronous background worker (Search & Stream pipeline).
- [x] Phase 6: Favorites, History & Mini-player mode.
- [x] Phase 7: Optimization (LTO, strip, opt-level 3), release binary deployed (`Melodyfy-Rust.exe`).
