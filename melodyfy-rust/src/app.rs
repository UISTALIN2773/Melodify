use crate::audio::AudioEngine;
use crate::models::{AppLibrary, RepeatMode, Track};
use crate::streamer::Streamer;
use eframe::egui::{self, Color32, FontId, Rounding, Vec2};
use std::os::windows::process::CommandExt;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Instant;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewTab {
    Home,
    Search,
    Favorites,
    Playlists,
    Downloads,
    History,
    Queue,
    Lyrics,
}

pub struct MelodyfyApp {
    audio: Arc<AudioEngine>,
    streamer: Arc<Streamer>,
    library: AppLibrary,

    // Navigation & View
    current_tab: ViewTab,
    search_query: String,
    is_searching: bool,
    search_results: Vec<Track>,
    search_rx: Option<Receiver<Vec<Track>>>,

    // Queue & Playback
    queue: Vec<Track>,
    current_index: Option<usize>,
    is_loading_stream: bool,
    stream_rx: Option<Receiver<(String, Option<String>)>>, // (track_id, Option<stream_url>)
    repeat_mode: RepeatMode,
    is_shuffle: bool,
    last_known_state: i32,
    last_seek_time: Instant,
    volume: i32,
    playback_rate: f64,

    // Infinite Radio / Auto-Mix
    is_auto_radio: bool,
    is_loading_radio: bool,
    radio_rx: Option<Receiver<Vec<Track>>>,

    // Sleep Timer
    sleep_timer_minutes: u32, // 0 = Off, 15, 30, 45, 60
    sleep_timer_end: Option<Instant>,

    // Custom Playlists UI
    selected_playlist: Option<String>,
    new_playlist_name: String,

    // Notification toast
    toast_message: Option<(String, Instant)>,

    // Mini Player
    is_mini_mode: bool,

    // Video PiP (Picture-in-Picture) Floating Overlay Mode for Gaming
    pub is_pip_video_mode: bool,
    pub pip_corner: usize, // 0 = Bottom-Right, 1 = Top-Right, 2 = Bottom-Left, 3 = Top-Left
    pub pip_opacity: f32, // 1.0, 0.85, 0.70
    pub pip_last_interaction: Instant,
    pub pip_video_child: Option<std::process::Child>,
    pub pip_video_rx: Option<Receiver<(String, Option<String>, f64)>>,
    pub pip_active_track_id: Option<String>,
    pub pip_styled: bool,

    // Theme Accent (true = Spotify Green, false = Melodyfy Amber)
    use_spotify_green: bool,

    // Tag Customization
    is_editing_tags: bool,
    show_add_tag_dialog: bool,
    new_tag_label: String,
    new_tag_query: String,

    // Mobile Mode (Genuine Spotify Mobile 9:16 UI)
    pub is_mobile_mode: bool,
    pub show_mobile_fullscreen_player: bool,
    pub mobile_library_subtab: usize,

    // Category filter for the Home Popular Grid
    pub popular_category_filter: usize,

    // Lyrics & Karaoke (LRCLIB)
    lyrics_data: Option<crate::lyrics::LyricsData>,
    is_loading_lyrics: bool,
    lyrics_rx: Option<Receiver<(String, Option<crate::lyrics::LyricsData>)>>,
    lyrics_offset_secs: f64,

    // Offline Downloads (Sin DRM)
    download_rx: Option<Receiver<(String, bool, Option<(Track, String)>)>>,
}

impl MelodyfyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, start_in_mobile: bool) -> Self {
        let audio = Arc::new(AudioEngine::new());
        let streamer = Arc::new(Streamer::new());
        let mut library = AppLibrary::load();
        library.scan_local_downloads();
        let volume = library.last_volume.unwrap_or(100);
        let is_auto_radio = library.auto_radio.unwrap_or(true);
        audio.set_volume(volume);

        let mut app = Self {
            audio,
            streamer,
            library,
            current_tab: ViewTab::Home,
            search_query: String::new(),
            is_searching: false,
            search_results: Vec::new(),
            search_rx: None,
            queue: Vec::new(),
            current_index: None,
            is_loading_stream: false,
            stream_rx: None,
            repeat_mode: RepeatMode::All,
            is_shuffle: false,
            last_known_state: 1,
            last_seek_time: Instant::now(),
            volume,
            playback_rate: 1.0,
            is_auto_radio,
            is_loading_radio: false,
            radio_rx: None,
            sleep_timer_minutes: 0,
            sleep_timer_end: None,
            selected_playlist: None,
            new_playlist_name: String::new(),
            toast_message: None,
            is_mini_mode: false,
            is_pip_video_mode: false,
            pip_corner: 0,
            pip_opacity: 1.0,
            pip_last_interaction: Instant::now(),
            pip_video_child: None,
            pip_video_rx: None,
            pip_active_track_id: None,
            pip_styled: false,
            use_spotify_green: true,
            is_editing_tags: false,
            show_add_tag_dialog: false,
            new_tag_label: String::new(),
            new_tag_query: String::new(),
            is_mobile_mode: start_in_mobile,
            show_mobile_fullscreen_player: false,
            mobile_library_subtab: 0,
            popular_category_filter: 0,
            lyrics_data: None,
            is_loading_lyrics: false,
            lyrics_rx: None,
            lyrics_offset_secs: 0.0,
            download_rx: None,
        };

        // Default initial home query: Anime Openings & J-Pop
        app.trigger_search("Anime Openings J-Pop Hits");
        app
    }

    pub fn accent_color(&self) -> Color32 {
        if self.use_spotify_green {
            Color32::from_rgb(30, 215, 96) // Spotify Green #1ED760
        } else {
            Color32::from_rgb(232, 122, 48) // Melodyfy Amber #E87A30
        }
    }

    pub fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast_message = Some((msg.into(), Instant::now()));
    }

    pub fn trigger_search(&mut self, query: &str) {
        let q = query.trim();
        if q.is_empty() {
            return;
        }
        let (tx, rx) = channel();
        self.is_searching = true;
        self.search_results.clear();
        self.search_rx = Some(rx);
        self.streamer.search_async(q.to_string(), tx);
    }

    pub fn play_track_now(&mut self, track: Track) {
        self.library.record_history(track.clone());

        // Insert into queue if not present, or set current index
        let pos = self.queue.iter().position(|t| t.id == track.id).unwrap_or_else(|| {
            self.queue.push(track.clone());
            self.queue.len() - 1
        });
        self.current_index = Some(pos);

        self.start_resolving_stream(track);
    }

    pub fn play_queue_index(&mut self, index: usize) {
        if index < self.queue.len() {
            self.current_index = Some(index);
            let track = self.queue[index].clone();
            self.library.record_history(track.clone());
            self.start_resolving_stream(track);
        }
    }

    fn start_resolving_stream(&mut self, track: Track) {
        self.is_loading_stream = true;
        let streamer = Arc::clone(&self.streamer);
        let (tx, rx) = channel();
        self.stream_rx = Some(rx);

        std::thread::spawn(move || {
            let stream_url = streamer.resolve_stream_url(&track);
            let _ = tx.send((track.id, stream_url));
        });

        // Instant prefetch next track in queue for 0ms gapless transition
        if let Some(curr_idx) = self.current_index {
            let next_idx = curr_idx + 1;
            if next_idx < self.queue.len() {
                self.streamer.prefetch_stream_url(&self.queue[next_idx]);
            }
        }

        self.fetch_lyrics_for_current();

        if self.is_pip_video_mode {
            self.start_pip_video_stream();
        }
    }

    pub fn start_pip_video_stream(&mut self) {
        let Some(track) = self.current_track().cloned() else { return };
        let streamer = Arc::clone(&self.streamer);
        let (tx, rx) = channel();
        self.pip_video_rx = Some(rx);
        let track_clone = track.clone();
        let current_pos = self.audio.get_status().position;
        self.show_toast(format!("📺 Conectando video en vivo: {}...", track.title));

        std::thread::spawn(move || {
            let video_url = streamer.resolve_video_stream_url(&track_clone);
            let _ = tx.send((track_clone.id, video_url, current_pos));
        });
    }

    pub fn fetch_lyrics_for_current(&mut self) {
        if let Some(track) = self.current_track().cloned() {
            if let Some(ref existing) = self.lyrics_data {
                if existing.track_id == track.id {
                    return;
                }
            }
            self.lyrics_data = None;
            self.is_loading_lyrics = true;
            let (tx, rx) = channel();
            self.lyrics_rx = Some(rx);
            let id = track.id.clone();
            let title = track.title.clone();
            let artist = track.artist.clone();
            let dur = track.duration_seconds;
            std::thread::spawn(move || {
                let data = crate::lyrics::LyricsFetcher::fetch_lyrics(&title, &artist, dur, &id);
                let _ = tx.send((id, data));
            });
        }
    }

    pub fn start_download(&mut self, track: Track) {
        self.show_toast(format!("💾 Descargando: {}...", track.title));
        let (tx, rx) = channel();
        self.download_rx = Some(rx);
        self.streamer.download_track_async(track, tx);
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|i| self.queue.get(i))
    }

    pub fn cycle_playback_rate(&mut self) {
        self.playback_rate = if (self.playback_rate - 1.0).abs() < 0.05 {
            1.25 // Nightcore / Speed Up
        } else if (self.playback_rate - 1.25).abs() < 0.05 {
            1.5 // Fast
        } else if (self.playback_rate - 1.5).abs() < 0.05 {
            0.85 // Slowed / Chill
        } else {
            1.0 // Normal
        };
        self.audio.set_playback_rate(self.playback_rate);
        let mode_label = match () {
            _ if (self.playback_rate - 1.25).abs() < 0.05 => "⚡ Modo Nightcore (1.25x)",
            _ if (self.playback_rate - 1.5).abs() < 0.05 => "⏩ Velocidad Rápida (1.5x)",
            _ if (self.playback_rate - 0.85).abs() < 0.05 => "🌙 Modo Slowed Chill (0.85x)",
            _ => "🎵 Velocidad Normal (1.0x)",
        };
        self.show_toast(mode_label);
    }

    pub fn cycle_sleep_timer(&mut self) {
        self.sleep_timer_minutes = match self.sleep_timer_minutes {
            0 => 15,
            15 => 30,
            30 => 45,
            45 => 60,
            _ => 0,
        };
        if self.sleep_timer_minutes > 0 {
            self.sleep_timer_end = Some(Instant::now() + std::time::Duration::from_secs(self.sleep_timer_minutes as u64 * 60));
            self.show_toast(format!("🌙 Temporizador activado: {} min", self.sleep_timer_minutes));
        } else {
            self.sleep_timer_end = None;
            self.show_toast("🌙 Temporizador desactivado");
        }
    }

    pub fn trigger_auto_radio(&mut self) {
        if self.is_loading_radio {
            return;
        }
        let seed_query = if let Some(curr) = self.current_track() {
            format!("{} {} Mix", curr.artist, curr.title)
        } else {
            "Anime Openings J-Pop Hits".to_string()
        };
        self.is_loading_radio = true;
        self.show_toast("📻 Radio Infinita: Buscando pistas similares...");
        let (tx, rx) = channel();
        self.radio_rx = Some(rx);
        self.streamer.search_async(seed_query, tx);
    }

    pub fn play_next(&mut self) {
        if self.queue.is_empty() {
            if self.is_auto_radio {
                self.trigger_auto_radio();
            }
            return;
        }

        if self.is_shuffle {
            use std::collections::hash_map::RandomState;
            use std::hash::{BuildHasher, Hasher};
            let random_idx = (RandomState::new().build_hasher().finish() as usize) % self.queue.len();
            self.play_queue_index(random_idx);
            return;
        }

        let next_idx = match self.current_index {
            Some(i) => {
                if i + 1 < self.queue.len() {
                    Some(i + 1)
                } else if self.is_auto_radio {
                    self.trigger_auto_radio();
                    None
                } else if self.repeat_mode == RepeatMode::All {
                    Some(0)
                } else {
                    None
                }
            }
            None => Some(0),
        };

        if let Some(idx) = next_idx {
            self.play_queue_index(idx);
        }
    }

    pub fn play_prev(&mut self) {
        let status = self.audio.get_status();
        if status.position > 3.0 {
            self.audio.seek(0.0);
            return;
        }

        if let Some(i) = self.current_index {
            if i > 0 {
                self.play_queue_index(i - 1);
            } else if self.repeat_mode == RepeatMode::All && !self.queue.is_empty() {
                self.play_queue_index(self.queue.len() - 1);
            }
        }
    }

    fn check_auto_advance(&mut self) {
        let status = self.audio.get_status();
        // State 8 = MediaEnded in WMP
        if self.last_known_state == 3 && (status.state == 8 || (status.duration > 0.0 && status.position >= status.duration - 0.5)) {
            if self.repeat_mode == RepeatMode::One {
                self.audio.seek(0.0);
                self.audio.play();
            } else {
                self.play_next();
            }
        }
        self.last_known_state = status.state;
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Toggle PiP Video Mode: Ctrl + P (or Esc if currently in PiP)
        if ctx.input(|i| (i.modifiers.ctrl && i.key_pressed(egui::Key::P)) || (self.is_pip_video_mode && i.key_pressed(egui::Key::Escape))) {
            self.toggle_pip_mode(ctx);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && !ctx.wants_keyboard_input() {
            self.audio.toggle_play_pause();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowRight)) {
            self.play_next();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowLeft)) {
            self.play_prev();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowUp)) {
            self.volume = (self.volume + 5).min(100);
            self.audio.set_volume(self.volume);
            self.library.last_volume = Some(self.volume);
            self.library.save();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowDown)) {
            self.volume = (self.volume - 5).max(0);
            self.audio.set_volume(self.volume);
            self.library.last_volume = Some(self.volume);
            self.library.save();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::M)) && !ctx.wants_keyboard_input() {
            if self.volume > 0 {
                self.volume = 0;
            } else {
                self.volume = 80;
            }
            self.audio.set_volume(self.volume);
        }
    }
}

impl eframe::App for MelodyfyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let is_playing = self.audio.get_status().state == 3;
        if is_playing || self.is_searching || self.is_loading_stream {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // Background workers checking
        if let Some(ref rx) = self.search_rx {
            while let Ok(tracks) = rx.try_recv() {
                self.search_results = tracks;
                self.is_searching = false;
            }
        }

        if let Some(ref rx) = self.stream_rx {
            if let Ok((track_id, maybe_url)) = rx.try_recv() {
                if let Some(curr) = self.current_track() {
                    if curr.id == track_id {
                        self.is_loading_stream = false;
                        if let Some(stream_url) = maybe_url {
                            // Only play internal audio if NOT in active video PiP mode
                            if !self.is_pip_video_mode || self.pip_video_child.is_none() {
                                self.audio.play_url(stream_url);
                            }
                        } else {
                            self.show_toast("Error al cargar stream de audio");
                        }
                    }
                }
            }
        }

        // Background Video PiP stream resolution
        if let Some(ref rx) = self.pip_video_rx {
            if let Ok((track_id, maybe_url, start_pos)) = rx.try_recv() {
                if let Some(curr) = self.current_track() {
                    if curr.id == track_id && self.is_pip_video_mode {
                        if let Some(video_url) = maybe_url {
                            if let Some(ffplay_path) = Streamer::find_ffplay_path() {
                                if let Some(mut prev) = self.pip_video_child.take() {
                                    let _ = prev.kill();
                                }
                                self.audio.pause();

                                let corner_pos = Self::get_screen_corner_pos(self.pip_corner, 426.0, 240.0);
                                let left = corner_pos.x as i32;
                                let top = corner_pos.y as i32;

                                let mut cmd = std::process::Command::new(&ffplay_path);
                                cmd.args(&[
                                    "-loglevel", "quiet",
                                    "-nostats",
                                    "-alwaysontop",
                                    "-x", "426",
                                    "-y", "240",
                                    "-left", &left.to_string(),
                                    "-top", &top.to_string(),
                                    "-window_title", "Melodyfy Video PiP",
                                    "-ss", &format!("{:.1}", start_pos),
                                    &video_url,
                                ])
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .creation_flags(CREATE_NO_WINDOW);

                                match cmd.spawn() {
                                    Ok(child) => {
                                        self.pip_video_child = Some(child);
                                        self.pip_active_track_id = Some(track_id);
                                        self.pip_styled = false;
                                        self.show_toast("🔴 Video PiP en reproducción activa");
                                    }
                                    Err(e) => {
                                        eprintln!("[Melodyfy PiP] Error al ejecutar ffplay: {:?}", e);
                                        self.audio.play();
                                        self.show_toast("Error al ejecutar reproductor de video");
                                    }
                                }
                            } else {
                                self.show_toast("⚠️ No se encontró ffplay.exe");
                            }
                        } else {
                            self.show_toast("⚠️ No se pudo obtener el stream de video");
                        }
                    }
                }
            }
        }

        // Monitor PiP video child process (exit / close detection) & Apply DWM Dark Titlebar
        if let Some(ref mut child) = self.pip_video_child {
            if !self.pip_styled {
                if Self::apply_dark_titlebar_to_pip() {
                    self.pip_styled = true;
                }
            }
            if let Ok(Some(_)) = child.try_wait() {
                self.pip_video_child = None;
                self.pip_active_track_id = None;
                self.pip_styled = false;
                if self.is_pip_video_mode {
                    self.toggle_pip_mode(ctx);
                } else {
                    self.audio.play();
                }
            }
        }

        if let Some(ref rx) = self.lyrics_rx {
            if let Ok((track_id, maybe_data)) = rx.try_recv() {
                if let Some(curr) = self.current_track() {
                    if curr.id == track_id {
                        self.lyrics_data = maybe_data;
                        self.is_loading_lyrics = false;
                    }
                }
            }
        }

        let mut download_msgs = Vec::new();
        let mut new_downloads = Vec::new();
        if let Some(ref rx) = self.download_rx {
            while let Ok((msg, _success, downloaded_info)) = rx.try_recv() {
                download_msgs.push(msg);
                if let Some(info) = downloaded_info {
                    new_downloads.push(info);
                }
            }
        }
        for (track, local_path) in new_downloads {
            self.library.record_download(track, local_path);
        }
        for msg in download_msgs {
            self.show_toast(msg);
        }

        // Check Infinite Radio Auto-Mix receiver
        let mut incoming_radio: Option<Vec<Track>> = None;
        if let Some(ref rx) = self.radio_rx {
            while let Ok(tracks) = rx.try_recv() {
                incoming_radio = Some(tracks);
            }
        }
        if let Some(radio_tracks) = incoming_radio {
            let prev_len = self.queue.len();
            let mut added_count = 0;
            for t in radio_tracks {
                if !self.queue.iter().any(|existing| existing.id == t.id) {
                    self.queue.push(t);
                    added_count += 1;
                    if added_count >= 10 {
                        break;
                    }
                }
            }
            if added_count > 0 {
                self.is_loading_radio = false;
                self.radio_rx = None;
                self.show_toast(format!("📻 Radio Infinita: +{} canciones añadidas", added_count));
                if prev_len == 0 || self.current_index == Some(prev_len.saturating_sub(1)) {
                    self.play_queue_index(prev_len);
                }
            }
        }

        // Check Sleep Timer countdown
        if let Some(end_time) = self.sleep_timer_end {
            if Instant::now() >= end_time {
                self.audio.pause();
                self.sleep_timer_end = None;
                self.sleep_timer_minutes = 0;
                self.show_toast("🌙 Temporizador finalizado: Música en pausa");
            }
        }

        self.check_auto_advance();
        self.handle_shortcuts(ctx);

        let accent = self.accent_color();

        // Sophisticated Obsidian Glass Dark theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(16, 16, 16);
        visuals.window_fill = Color32::from_rgb(16, 16, 16);
        visuals.selection.bg_fill = accent;
        visuals.hyperlink_color = accent;
        visuals.widgets.noninteractive.rounding = Rounding::same(10.0);
        visuals.widgets.inactive.rounding = Rounding::same(10.0);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(26, 26, 26);
        visuals.widgets.hovered.rounding = Rounding::same(10.0);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 38, 38);
        visuals.widgets.active.rounding = Rounding::same(10.0);
        visuals.widgets.active.bg_fill = Color32::from_rgb(48, 48, 48);
        ctx.set_visuals(visuals);

        // Toast overlay if active
        if let Some((ref msg, time)) = self.toast_message {
            if time.elapsed().as_secs_f32() < 2.5 {
                let screen_rect = ctx.screen_rect();
                let toast_rect = egui::Rect::from_center_size(
                    egui::pos2(screen_rect.center().x, screen_rect.max.y - 120.0),
                    Vec2::new(260.0, 36.0),
                );
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("toast")));
                painter.rect_filled(toast_rect, Rounding::same(18.0), Color32::from_rgb(34, 197, 94));
                painter.text(
                    toast_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    msg,
                    FontId::proportional(13.0),
                    Color32::BLACK,
                );
            } else {
                self.toast_message = None;
            }
        }

        // Video PiP (Picture-in-Picture) Floating Overlay Mode for Gaming
        if self.is_pip_video_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(Color32::from_black_alpha((self.pip_opacity * 255.0) as u8)))
                .show(ctx, |ui| {
                    self.render_pip_video_ui(ui, accent);
                });
            return;
        }

        // Mini-Player / Segundo Plano Mode: Compact Floating HUD
        if self.is_mini_mode {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_mini_player_ui(ui, accent);
            });
            return;
        }

        // Mobile Mode (Genuine Spotify Mobile 9:16 vertical layout)
        if self.is_mobile_mode {
            self.render_mobile_ui(ctx, accent);
            return;
        }

        // BOTTOM PLAYBACK BAR (Always fixed at bottom)
        egui::TopBottomPanel::bottom("bottom_bar")
            .exact_height(90.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(14, 14, 14))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(28, 28, 28)))
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            )
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::new(12.0, 4.0);
                self.render_bottom_player_bar(ui, accent);
            });

        // LEFT SIDEBAR
        egui::SidePanel::left("left_sidebar")
            .exact_width(230.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(12, 12, 12))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(24, 24, 24)))
                    .inner_margin(egui::Margin::symmetric(10.0, 10.0))
            )
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::new(0.0, 8.0);
                self.render_sidebar(ui, accent);
            });

        // CENTRAL MAIN AREA
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(18, 18, 18))
                    .inner_margin(egui::Margin::symmetric(16.0, 10.0))
            )
            .show(ctx, |ui| {
                self.render_main_content(ui, accent);
            });
    }
}

impl MelodyfyApp {
    fn render_sidebar(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.add_space(8.0);

        // 1. BRANDING HEADER
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.add(
                egui::Image::new(crate::APP_LOGO_URI)
                    .fit_to_exact_size(Vec2::new(36.0, 36.0))
                    .rounding(Rounding::same(8.0)),
            );
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Melodyfy").strong().size(18.0).color(Color32::WHITE));
                ui.label(egui::RichText::new("SPOTIFY KILLER • RUST").size(8.5).color(accent).strong());
            });
        });

        ui.add_space(10.0);

        // 2. MAIN NAVIGATION TABS (Strictly Left-Aligned & Pixel-Perfect)
        let tabs = [
            (ViewTab::Home, "🏠", "Inicio"),
            (ViewTab::Search, "🔍", "Buscar"),
            (ViewTab::Favorites, "💚", "Tus Me Gusta"),
            (ViewTab::Playlists, "📂", "Mis Playlists"),
            (ViewTab::Downloads, "💾", "Descargas Offline"),
            (ViewTab::History, "🕒", "Historial"),
            (ViewTab::Queue, "📑", "Cola Actual"),
            (ViewTab::Lyrics, "🎤", "Letras Karaoke"),
        ];

        let tab_btn_w = (ui.available_width() - 8.0).max(180.0);

        for (tab, icon, label) in tabs {
            let is_active = self.current_tab == tab;
            let bg = if is_active { Color32::from_rgb(28, 28, 28) } else { Color32::TRANSPARENT };
            let text_color = if is_active { Color32::WHITE } else { Color32::from_rgb(175, 175, 175) };

            let (rect, resp) = ui.allocate_exact_size(Vec2::new(tab_btn_w, 32.0), egui::Sense::click());

            if resp.hovered() && !is_active {
                ui.painter().rect_filled(rect, Rounding::same(8.0), Color32::from_rgb(22, 22, 22));
            } else if is_active {
                ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
                // Active accent bar on left edge
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(egui::pos2(rect.left(), rect.top() + 5.0), Vec2::new(3.0, 22.0)),
                    Rounding::same(1.5),
                    accent,
                );
            }

            // Left-aligned icon and label in exact columns
            ui.painter().text(
                egui::pos2(rect.left() + 14.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                icon,
                FontId::proportional(14.5),
                if is_active { accent } else { Color32::from_rgb(180, 180, 180) },
            );
            ui.painter().text(
                egui::pos2(rect.left() + 38.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                FontId::proportional(13.0),
                text_color,
            );

            if resp.clicked() {
                self.current_tab = tab;
                if tab == ViewTab::Lyrics {
                    self.fetch_lyrics_for_current();
                } else if tab == ViewTab::Downloads {
                    self.library.scan_local_downloads();
                }
            }

            ui.add_space(2.0);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        // 3. FOOTER PINNED AT BOTTOM & SCROLLABLE MIDDLE (Mixes & Géneros)
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(4.0);

            ui.label(egui::RichText::new("⚡ 100% Libre • Sin Publicidad").size(9.5).color(Color32::from_rgb(100, 100, 100)));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.add_space(4.0);
                // Theme toggle chip
                let theme_label = if self.use_spotify_green { "🟢 Verde" } else { "🟠 Ámbar" };
                let theme_btn = egui::Button::new(egui::RichText::new(theme_label).size(11.0).color(Color32::from_rgb(200, 200, 200)))
                    .fill(Color32::from_rgb(26, 26, 26))
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(88.0, 26.0));
                if ui.add(theme_btn).on_hover_text("Cambiar color de acento visual").clicked() {
                    self.use_spotify_green = !self.use_spotify_green;
                }

                ui.add_space(4.0);

                // Mobile Mode pill
                let mobile_btn = egui::Button::new(egui::RichText::new("📱 Móvil").size(11.0).color(accent))
                    .fill(Color32::from_rgb(26, 26, 26))
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(88.0, 26.0));
                if ui.add(mobile_btn).on_hover_text("Cambiar a interfaz smartphone (390x820)").clicked() {
                    self.is_mobile_mode = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(390.0, 820.0)));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title("📱 Melodyfy Mobile".into()));
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // 4. SCROLLABLE MIDDLE SECTION: MIXES & GÉNEROS
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("MIXES & GÉNEROS").size(10.5).color(Color32::from_rgb(120, 120, 120)).strong());
                });
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let genres = [
                            ("🎧 Lo-Fi Chill Gaming", "Lo-Fi Gaming Chill Beats"),
                            ("⚡ Synthwave & Cyberpunk", "Synthwave Cyberpunk 80s"),
                            ("🎸 Rock Legendario", "Rock Clásico 80s 90s"),
                            ("🔥 Top Global 2026", "Top Éxitos Mundiales 2026"),
                            ("🌸 Anime Openings", "Anime Openings HD"),
                            ("🚗 Phonk Drift Music", "Phonk Drift Bass boosted"),
                        ];

                        for (display, query) in genres {
                            let (g_rect, g_resp) = ui.allocate_exact_size(Vec2::new(tab_btn_w, 26.0), egui::Sense::click());
                            if g_resp.hovered() {
                                ui.painter().rect_filled(g_rect, Rounding::same(6.0), Color32::from_rgb(24, 24, 24));
                            }
                            let g_color = if g_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(170, 170, 170) };
                            ui.painter().text(
                                egui::pos2(g_rect.left() + 8.0, g_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                display,
                                FontId::proportional(12.0),
                                g_color,
                            );

                            if g_resp.clicked() {
                                self.search_query = query.to_string();
                                self.current_tab = ViewTab::Search;
                                self.trigger_search(query);
                            }
                        }
                    });
            });
        });
    }

    fn render_main_content(&mut self, ui: &mut egui::Ui, accent: Color32) {
        // Ambient Glow & Subtle Mouse Parallax Depth (0 Extra MB RAM, pure mathematical gradient)
        let content_rect = ui.available_rect_before_wrap();
        let pointer_pos = ui.ctx().pointer_latest_pos().unwrap_or_else(|| content_rect.center());
        let rel_x = ((pointer_pos.x - content_rect.center().x) / (content_rect.width() * 0.5).max(1.0)).clamp(-1.0, 1.0);
        let rel_y = ((pointer_pos.y - content_rect.center().y) / (content_rect.height() * 0.5).max(1.0)).clamp(-1.0, 1.0);
        let glow_center = egui::pos2(
            content_rect.left() + 240.0 + rel_x * 26.0,
            content_rect.top() + 90.0 + rel_y * 18.0,
        );
        let glow_color = Color32::from_rgba_premultiplied(
            accent.r() / 8,
            accent.g() / 8,
            accent.b() / 8,
            18,
        );
        ui.painter().circle_filled(glow_center, 280.0, glow_color);

        ui.add_space(14.0);

        // Search Input Bar with Clear button & Action
        ui.horizontal(|ui| {
            ui.add_space(16.0);

            let search_edit = ui.add_sized(
                [420.0, 36.0],
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍  ¿Qué canción, artista o URL quieres escuchar?")
                    .font(FontId::proportional(14.0)),
            );

            if !self.search_query.is_empty() {
                if ui.small_button(egui::RichText::new("✕").color(Color32::GRAY).size(13.0)).on_hover_text("Limpiar búsqueda").clicked() {
                    self.search_query.clear();
                }
            }

            let enter_pressed = search_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let btn_clicked = ui.add_sized([90.0, 36.0], egui::Button::new(egui::RichText::new("Buscar").strong().color(Color32::BLACK)).fill(accent)).clicked();

            if enter_pressed || btn_clicked {
                let query = self.search_query.trim().to_string();
                if !query.is_empty() {
                    self.current_tab = ViewTab::Search;
                    self.trigger_search(&query);
                }
            }

            if self.is_searching {
                ui.add_space(10.0);
                ui.spinner();
                ui.label(egui::RichText::new("Buscando pistas...").color(accent).size(12.0));
            }
        });

        ui.add_space(8.0);

        // Quick Search Chips / Pill Filters (Personalizables)
        let quick_tags = self.library.get_quick_tags();
        let mut remove_index = None;

        ui.horizontal_wrapped(|ui| {
            ui.add_space(16.0);
            for (idx, tag) in quick_tags.iter().enumerate() {
                let is_active = self.search_query == tag.query;
                let bg_color = if is_active { accent } else { Color32::from_rgb(34, 34, 34) };
                let text_color = if is_active { Color32::BLACK } else { Color32::from_rgb(220, 220, 220) };

                ui.horizontal(|ui| {
                    let chip_btn = egui::Button::new(egui::RichText::new(&tag.label).size(12.0).strong().color(text_color))
                        .fill(bg_color)
                        .rounding(Rounding::same(14.0));

                    if ui.add(chip_btn).clicked() {
                        self.search_query = tag.query.clone();
                        self.current_tab = ViewTab::Search;
                        self.trigger_search(&tag.query);
                    }

                    if self.is_editing_tags {
                        if ui.small_button(egui::RichText::new("✕").color(Color32::from_rgb(239, 68, 68)).size(10.0)).on_hover_text("Eliminar este botón").clicked() {
                            remove_index = Some(idx);
                        }
                    }
                });
                ui.add_space(4.0);
            }

            // Button to Add New Tag
            let add_btn = egui::Button::new(egui::RichText::new("➕ Tag").size(11.5).strong().color(Color32::WHITE))
                .fill(Color32::from_rgb(45, 45, 45))
                .rounding(Rounding::same(14.0));
            if ui.add(add_btn).on_hover_text("Agregar nuevo género o artista").clicked() {
                self.show_add_tag_dialog = !self.show_add_tag_dialog;
            }

            // Button to toggle Edit/Delete mode
            let edit_label = if self.is_editing_tags { "✓ Listo" } else { "✏️ Quitar / Editar" };
            let edit_btn = egui::Button::new(egui::RichText::new(edit_label).size(11.5).color(if self.is_editing_tags { accent } else { Color32::GRAY }))
                .fill(Color32::from_rgb(28, 28, 28))
                .rounding(Rounding::same(14.0));
            if ui.add(edit_btn).on_hover_text("Activar para quitar botones con la cruz").clicked() {
                self.is_editing_tags = !self.is_editing_tags;
            }
        });

        if let Some(del_idx) = remove_index {
            self.library.remove_quick_tag(del_idx);
            self.show_toast("Botón eliminado");
        }

        // Inline Add Tag Form
        if self.show_add_tag_dialog {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(egui::RichText::new("Nuevo Botón:").strong().size(12.0).color(accent));
                ui.add_sized([120.0, 24.0], egui::TextEdit::singleline(&mut self.new_tag_label).hint_text("Ej: 🎸 Metal"));
                ui.add_sized([160.0, 24.0], egui::TextEdit::singleline(&mut self.new_tag_query).hint_text("Búsqueda en YouTube"));

                if ui.add(egui::Button::new(egui::RichText::new("Guardar").strong().color(Color32::BLACK)).fill(accent)).clicked() {
                    let label = self.new_tag_label.trim().to_string();
                    let query = if self.new_tag_query.trim().is_empty() { label.clone() } else { self.new_tag_query.trim().to_string() };
                    if !label.is_empty() {
                        self.library.add_quick_tag(label, query);
                        self.new_tag_label.clear();
                        self.new_tag_query.clear();
                        self.show_add_tag_dialog = false;
                        self.show_toast("¡Nuevo botón guardado!");
                    }
                }
                if ui.button("Cancelar").clicked() {
                    self.show_add_tag_dialog = false;
                }
            });
        }

        ui.add_space(10.0);
        ui.separator();

        match self.current_tab {
            ViewTab::Home => self.render_home_view(ui, accent),
            ViewTab::Search => self.render_search_view(ui, accent),
            ViewTab::Favorites => self.render_favorites_view(ui, accent),
            ViewTab::Playlists => self.render_playlists_view(ui, accent),
            ViewTab::Downloads => self.render_downloads_view(ui, accent),
            ViewTab::History => self.render_history_view(ui, accent),
            ViewTab::Queue => self.render_queue_view(ui, accent),
            ViewTab::Lyrics => self.render_lyrics_view(ui, accent),
        }
    }

    fn render_home_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.add(
                    egui::Image::new(crate::APP_LOGO_URI)
                        .fit_to_exact_size(Vec2::new(44.0, 44.0))
                        .rounding(Rounding::same(8.0)),
                );
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.heading(egui::RichText::new("Buenas tardes").size(26.0).strong().color(Color32::WHITE));
                    ui.label(egui::RichText::new("Melodyfy Rust Engine • Audio Nativo Sin Anuncios").size(11.5).color(accent));
                });
            });

            ui.add_space(16.0);

            // Spotify 6-Card Grid: J-Pop, Anime Openings & Covers
            let grid_cards = [
                ("🌸 J-Pop Hits", "J-Pop Hits 2026", Color32::from_rgb(180, 45, 110)),
                ("⚔️ Openings Anime", "Anime Openings HD", Color32::from_rgb(45, 70, 185)),
                ("🎸 Covers de OPs", "Anime Rock Metal Covers", Color32::from_rgb(175, 40, 30)),
                ("🔥 Shonen Hits", "Shonen Anime Openings", Color32::from_rgb(195, 100, 20)),
                ("🌃 City Pop 80s", "Japanese City Pop 80s", Color32::from_rgb(130, 30, 120)),
                ("🎧 Anime Lo-Fi", "Anime Lofi Chill Beats", Color32::from_rgb(30, 120, 70)),
            ];

            let card_width = ((ui.available_width() - 60.0) / 3.0).max(180.0);
            let card_height = 56.0;

            for chunk in grid_cards.chunks(3) {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    for (title, query, bg_color) in chunk {
                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_width, card_height), egui::Sense::click());
                        let hover_bg = if resp.hovered() { Color32::from_rgb(55, 55, 55) } else { Color32::from_rgb(38, 38, 38) };

                        ui.painter().rect_filled(rect, Rounding::same(6.0), hover_bg);

                        // Left color accent square
                        let art_rect = egui::Rect::from_min_size(rect.min, Vec2::new(56.0, 56.0));
                        ui.painter().rect_filled(art_rect, Rounding::same(6.0), *bg_color);
                        ui.painter().text(art_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(22.0), Color32::WHITE);

                        // Card Title
                        ui.painter().text(
                            egui::pos2(rect.min.x + 68.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            title,
                            FontId::proportional(13.5),
                            Color32::WHITE,
                        );

                        // Green Play button on hover
                        if resp.hovered() {
                            let play_circle = egui::Rect::from_center_size(
                                egui::pos2(rect.max.x - 24.0, rect.center().y),
                                Vec2::new(32.0, 32.0),
                            );
                            ui.painter().circle_filled(play_circle.center(), 16.0, accent);
                            ui.painter().text(play_circle.center(), egui::Align2::CENTER_CENTER, "▶", FontId::proportional(14.0), Color32::BLACK);
                        }

                        if resp.clicked() {
                            self.search_query = query.to_string();
                            self.current_tab = ViewTab::Search;
                            self.trigger_search(query);
                        }

                        ui.add_space(10.0);
                    }
                });
                ui.add_space(10.0);
            }

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("Canciones Populares • J-Pop & Anime OPs").size(22.0).strong().color(Color32::WHITE));
            });

            ui.add_space(10.0);
            self.render_popular_cards_grid(ui, accent);
        });
    }

    fn render_popular_cards_grid(&mut self, ui: &mut egui::Ui, accent: Color32) {
        // Category Filter Chips
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            let filters = [
                (0, "🔥 Todo"),
                (1, "🌸 J-Pop"),
                (2, "⚔️ Openings Anime"),
                (3, "🎸 Covers de OPs"),
                (4, "🌃 City Pop"),
            ];

            for (idx, name) in filters {
                let is_selected = self.popular_category_filter == idx;
                let bg_color = if is_selected { accent } else { Color32::from_rgb(38, 38, 38) };
                let text_color = if is_selected { Color32::BLACK } else { Color32::WHITE };

                let btn = egui::Button::new(egui::RichText::new(name).size(12.5).strong().color(text_color))
                    .fill(bg_color)
                    .rounding(Rounding::same(16.0));

                if ui.add(btn).clicked() {
                    self.popular_category_filter = idx;
                }
                ui.add_space(4.0);
            }
        });

        ui.add_space(12.0);

        struct PopularCardItem {
            title: &'static str,
            artist: &'static str,
            category: usize, // 1 = J-Pop, 2 = Openings Anime, 3 = Covers de OPs, 4 = City Pop
            query: &'static str,
            thumbnail_url: &'static str,
        }

        let items: &[PopularCardItem] = &[
            // --- OPENINGS DE ANIME ---
            PopularCardItem {
                title: "Idol (アイドル)",
                artist: "YOASOBI • Oshi no Ko",
                category: 2,
                query: "YOASOBI Idol Official Music Video",
                thumbnail_url: "https://i.ytimg.com/vi/ZRtdQ81jPUQ/hqdefault.jpg",
            },
            PopularCardItem {
                title: "KICK BACK",
                artist: "Kenshi Yonezu • Chainsaw Man",
                category: 2,
                query: "Kenshi Yonezu KICK BACK Official",
                thumbnail_url: "https://i.ytimg.com/vi/M2cckDmNLMI/hqdefault.jpg",
            },
            PopularCardItem {
                title: "SPECIALZ",
                artist: "King Gnu • Jujutsu Kaisen S2",
                category: 2,
                query: "King Gnu SPECIALZ Official Video",
                thumbnail_url: "https://i.ytimg.com/vi/fhzKLfZq6mg/hqdefault.jpg",
            },
            PopularCardItem {
                title: "The Rumbling",
                artist: "SiM • Attack on Titan",
                category: 2,
                query: "SiM The Rumbling Official Video",
                thumbnail_url: "https://i.ytimg.com/vi/OB8_w51Pv0E/hqdefault.jpg",
            },
            PopularCardItem {
                title: "紅蓮華 (Gurenge)",
                artist: "LiSA • Demon Slayer OP",
                category: 2,
                query: "LiSA 紅蓮華 Gurenge Official",
                thumbnail_url: "https://i.ytimg.com/vi/CwkzK-F0Y00/hqdefault.jpg",
            },
            PopularCardItem {
                title: "廻廻奇譚 (Kaikai Kitan)",
                artist: "Eve • Jujutsu Kaisen OP",
                category: 2,
                query: "Eve 廻廻奇譚 Kaikai Kitan Official",
                thumbnail_url: "https://i.ytimg.com/vi/1tk1P1sQuX0/hqdefault.jpg",
            },
            PopularCardItem {
                title: "unravel",
                artist: "TK from 凛として時雨 • Tokyo Ghoul",
                category: 2,
                query: "TK from 凛として時雨 unravel Official",
                thumbnail_url: "https://i.ytimg.com/vi/Fve_lHIPa-I/hqdefault.jpg",
            },
            PopularCardItem {
                title: "残響散歌 (Zankyou Sanka)",
                artist: "Aimer • Demon Slayer Yuukaku",
                category: 2,
                query: "Aimer 残響散歌 Zankyou Sanka",
                thumbnail_url: "https://i.ytimg.com/vi/tLQLa6lM3Us/hqdefault.jpg",
            },
            PopularCardItem {
                title: "Bling-Bang-Bang-Born",
                artist: "Creepy Nuts • Mashle S2",
                category: 2,
                query: "Creepy Nuts Bling-Bang-Bang-Born Official",
                thumbnail_url: "https://i.ytimg.com/vi/210R0ozmLwg/hqdefault.jpg",
            },
            PopularCardItem {
                title: "シルエット (Silhouette)",
                artist: "KANA-BOON • Naruto OP 16",
                category: 2,
                query: "KANA-BOON シルエット Silhouette Official",
                thumbnail_url: "https://i.ytimg.com/vi/dlFA0Zq1k2A/hqdefault.jpg",
            },
            PopularCardItem {
                title: "青のすみか (Where Our Blue Is)",
                artist: "Tatsuya Kitani • Jujutsu Kaisen",
                category: 2,
                query: "Tatsuya Kitani 青のすみか Where Our Blue Is",
                thumbnail_url: "https://i.ytimg.com/vi/zuoVs1kflu8/hqdefault.jpg",
            },
            PopularCardItem {
                title: "心臓を捧げよ! (Sasageyo)",
                artist: "Linked Horizon • Attack on Titan",
                category: 2,
                query: "Linked Horizon Shinzou wo Sasageyo Official",
                thumbnail_url: "https://i.ytimg.com/vi/CID-sYQNCew/hqdefault.jpg",
            },

            // --- COVERS DE OPENINGS (ROCK / METAL / ENGLISH / ACOUSTIC) ---
            PopularCardItem {
                title: "Unravel (Rock Cover)",
                artist: "Jonathan Young • Tokyo Ghoul",
                category: 3,
                query: "Unravel Jonathan Young Tokyo Ghoul Rock Cover",
                thumbnail_url: "https://i.ytimg.com/vi/s0c2QzTq2_s/hqdefault.jpg",
            },
            PopularCardItem {
                title: "The Rumbling (Metal Cover)",
                artist: "Raon Lee • Attack on Titan",
                category: 3,
                query: "Raon Lee The Rumbling Attack on Titan Cover",
                thumbnail_url: "https://i.ytimg.com/vi/lq4sQkU26lQ/hqdefault.jpg",
            },
            PopularCardItem {
                title: "Silhouette (English Cover)",
                artist: "NateWantsToBattle • Naruto OP",
                category: 3,
                query: "NateWantsToBattle Silhouette Naruto English Cover",
                thumbnail_url: "https://i.ytimg.com/vi/Dq0G3hLw2s4/hqdefault.jpg",
            },
            PopularCardItem {
                title: "Kaikai Kitan (Metal Cover)",
                artist: "PelleK • Jujutsu Kaisen OP",
                category: 3,
                query: "PelleK Kaikai Kitan Jujutsu Kaisen Metal Cover",
                thumbnail_url: "https://i.ytimg.com/vi/z9iGv5XQZfs/hqdefault.jpg",
            },
            PopularCardItem {
                title: "Gurenge (Metal Cover)",
                artist: "RichaadEB & Jonathan Young",
                category: 3,
                query: "Jonathan Young RichaadEB Gurenge Demon Slayer Cover",
                thumbnail_url: "https://i.ytimg.com/vi/J_8YqfMZZcM/hqdefault.jpg",
            },
            PopularCardItem {
                title: "Shinzo wo Sasageyo (Cover)",
                artist: "AmaLee • Attack on Titan OP",
                category: 3,
                query: "AmaLee Shinzo wo Sasageyo Attack on Titan English Cover",
                thumbnail_url: "https://i.ytimg.com/vi/4vjVb6k_rXg/hqdefault.jpg",
            },

            // --- J-POP & VOCAL ---
            PopularCardItem {
                title: "唱 (Show)",
                artist: "Ado • J-Pop Smash Hit",
                category: 1,
                query: "Ado 唱 Show Official Music Video",
                thumbnail_url: "https://i.ytimg.com/vi/pgXpM4l_MwI/hqdefault.jpg",
            },
            PopularCardItem {
                title: "夜に駆ける (Racing)",
                artist: "YOASOBI • J-Pop Anthem",
                category: 1,
                query: "YOASOBI 夜に駆ける Official Music Video",
                thumbnail_url: "https://i.ytimg.com/vi/x8VYWazR5mE/hqdefault.jpg",
            },
            PopularCardItem {
                title: "うっせぇわ (Usseewa)",
                artist: "Ado • Breakout J-Pop",
                category: 1,
                query: "Ado うっせぇわ Usseewa Official",
                thumbnail_url: "https://i.ytimg.com/vi/Qp3b-RXtz4w/hqdefault.jpg",
            },
            PopularCardItem {
                title: "カワキヲアメク (Rain)",
                artist: "Minami (美波) • Domestic GF",
                category: 1,
                query: "Minami カワキヲアメク Crying for Rain Official",
                thumbnail_url: "https://i.ytimg.com/vi/0YF8vecQWYs/hqdefault.jpg",
            },
            PopularCardItem {
                title: "Lemon",
                artist: "Kenshi Yonezu (米津玄師)",
                category: 1,
                query: "Kenshi Yonezu Lemon Official Music Video",
                thumbnail_url: "https://i.ytimg.com/vi/SX_ViT4Ra7k/hqdefault.jpg",
            },
            PopularCardItem {
                title: "真夜中のドア / Stay With Me",
                artist: "Miki Matsubara • City Pop 80s",
                category: 4,
                query: "Miki Matsubara Stay With Me Official Audio",
                thumbnail_url: "https://i.ytimg.com/vi/9GxP8b0XG5Y/hqdefault.jpg",
            },
        ];

        let filtered_items: Vec<&PopularCardItem> = items
            .iter()
            .filter(|item| self.popular_category_filter == 0 || item.category == self.popular_category_filter)
            .collect();

        let avail_w = ui.available_width() - 32.0;
        let card_w = 174.0_f32;
        let card_h = 246.0_f32;
        let spacing = 14.0_f32;
        let cols = ((avail_w + spacing) / (card_w + spacing)).floor().max(2.0) as usize;

        let mut trigger_query: Option<&'static str> = None;

        for chunk in filtered_items.chunks(cols) {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                for item in chunk {
                    let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, card_h), egui::Sense::click());
                    let is_hovered = resp.hovered();

                    // Spotify Card Background
                    let bg_color = if is_hovered {
                        Color32::from_rgb(42, 42, 42)
                    } else {
                        Color32::from_rgb(24, 24, 24)
                    };
                    ui.painter().rect_filled(card_rect, Rounding::same(8.0), bg_color);

                    // Subtle border on hover
                    if is_hovered {
                        ui.painter().rect_stroke(card_rect, Rounding::same(8.0), egui::Stroke::new(1.0_f32, Color32::from_rgb(65, 65, 65)));
                    }

                    // Cover Art Area (148x148)
                    let art_rect = egui::Rect::from_min_size(
                        egui::pos2(card_rect.min.x + 13.0, card_rect.min.y + 12.0),
                        Vec2::new(148.0, 148.0),
                    );
                    ui.painter().rect_filled(art_rect, Rounding::same(6.0), Color32::from_rgb(32, 32, 32));
                    ui.painter().text(art_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(32.0), Color32::from_rgb(70, 70, 70));

                    ui.put(
                        art_rect,
                        egui::Image::new(item.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(148.0, 148.0))
                            .rounding(Rounding::same(6.0)),
                    );

                    // Floating Green Play Button on Card Hover
                    if is_hovered {
                        let play_btn_center = egui::pos2(art_rect.max.x - 22.0, art_rect.max.y - 18.0);
                        // Shadow
                        ui.painter().circle_filled(
                            egui::pos2(play_btn_center.x, play_btn_center.y + 2.0),
                            20.0,
                            Color32::from_black_alpha(140),
                        );
                        // Spotify Green Circle
                        ui.painter().circle_filled(play_btn_center, 19.0, accent);
                        // Black Play Triangle
                        ui.painter().text(
                            egui::pos2(play_btn_center.x + 1.5, play_btn_center.y),
                            egui::Align2::CENTER_CENTER,
                            "▶",
                            FontId::proportional(17.0),
                            Color32::BLACK,
                        );
                    }

                    // Card Title
                    let title_clipped = if item.title.chars().count() > 18 {
                        format!("{}...", item.title.chars().take(16).collect::<String>())
                    } else {
                        item.title.to_string()
                    };
                    ui.painter().text(
                        egui::pos2(card_rect.min.x + 13.0, art_rect.max.y + 16.0),
                        egui::Align2::LEFT_CENTER,
                        title_clipped,
                        FontId::proportional(13.5),
                        Color32::WHITE,
                    );

                    // Card Artist / Anime Tag
                    let artist_clipped = if item.artist.chars().count() > 22 {
                        format!("{}...", item.artist.chars().take(20).collect::<String>())
                    } else {
                        item.artist.to_string()
                    };
                    ui.painter().text(
                        egui::pos2(card_rect.min.x + 13.0, art_rect.max.y + 34.0),
                        egui::Align2::LEFT_CENTER,
                        artist_clipped,
                        FontId::proportional(11.5),
                        Color32::from_rgb(160, 160, 160),
                    );

                    if resp.clicked() {
                        trigger_query = Some(item.query);
                    }

                    ui.add_space(spacing);
                }
            });
            ui.add_space(spacing);
        }

        if let Some(query) = trigger_query {
            self.search_query = query.to_string();
            self.current_tab = ViewTab::Search;
            self.trigger_search(query);
            self.show_toast("⚡ Reproduciendo selección...");
        }
    }

    fn render_search_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                let title = if self.search_query.is_empty() {
                    "Resultados de Búsqueda".to_string()
                } else {
                    format!("Resultados para \"{}\"", self.search_query)
                };
                ui.heading(egui::RichText::new(title).size(22.0).strong().color(Color32::WHITE));
            });

            ui.add_space(10.0);
            self.render_track_table(ui, &self.search_results.clone(), accent);
        });
    }

    fn render_favorites_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("💚 Tus Canciones Favoritas").size(22.0).strong().color(Color32::WHITE));
                ui.label(egui::RichText::new(format!("({} canciones guardadas)", self.library.favorites.len())).color(Color32::GRAY));
                if !self.library.favorites.is_empty() {
                    ui.add_space(12.0);
                    if ui.add(egui::Button::new(egui::RichText::new("▶ Reproducir Todo").strong().color(Color32::BLACK)).fill(accent)).clicked() {
                        self.queue = self.library.favorites.clone();
                        self.play_queue_index(0);
                        self.show_toast("▶ Reproduciendo Tus Me Gusta");
                    }
                }
            });

            ui.add_space(10.0);

            if self.library.favorites.is_empty() {
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Aún no tienes canciones favoritas.").size(16.0).color(Color32::GRAY));
                    ui.label(egui::RichText::new("Haz clic en el corazón 💚 al lado de cualquier canción para tenerla aquí.").color(Color32::DARK_GRAY));
                });
            } else {
                self.render_track_table(ui, &self.library.favorites.clone(), accent);
            }
        });
    }

    fn render_playlists_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("📂 Mis Playlists Personalizadas").size(22.0).strong().color(Color32::WHITE));
                ui.add_space(16.0);

                ui.add_sized(
                    [200.0, 28.0],
                    egui::TextEdit::singleline(&mut self.new_playlist_name)
                        .hint_text("➕ Nombre de nueva playlist..."),
                );

                if ui.add(egui::Button::new(egui::RichText::new("Crear Playlist").strong().color(Color32::BLACK)).fill(accent)).clicked() {
                    let name = self.new_playlist_name.trim().to_string();
                    if !name.is_empty() {
                        if self.library.create_playlist(name.clone()) {
                            self.selected_playlist = Some(name.clone());
                            self.new_playlist_name.clear();
                            self.show_toast(format!("📂 Playlist '{}' creada", name));
                        } else {
                            self.show_toast("Esa playlist ya existe");
                        }
                    }
                }
            });

            ui.add_space(12.0);

            let mut playlist_names: Vec<String> = self.library.playlists.keys().cloned().collect();
            playlist_names.sort();

            if playlist_names.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Aún no has creado ninguna playlist personalizada.").size(16.0).color(Color32::GRAY));
                    ui.label(egui::RichText::new("Escribe un nombre arriba o pulsa el botón 📂 en cualquier canción para guardarla.").color(Color32::DARK_GRAY));
                });
                return;
            }

            if self.selected_playlist.is_none() || !playlist_names.contains(self.selected_playlist.as_ref().unwrap()) {
                self.selected_playlist = playlist_names.first().cloned();
            }

            // Playlist selector chips
            let mut to_delete: Option<String> = None;
            ui.horizontal_wrapped(|ui| {
                ui.add_space(16.0);
                for name in &playlist_names {
                    let count = self.library.playlists.get(name).map(|v| v.len()).unwrap_or(0);
                    let is_sel = self.selected_playlist.as_deref() == Some(name.as_str());
                    let bg = if is_sel { accent } else { Color32::from_rgb(36, 36, 36) };
                    let fg = if is_sel { Color32::BLACK } else { Color32::WHITE };

                    let btn = egui::Button::new(egui::RichText::new(format!("📂 {} ({})", name, count)).strong().size(13.0).color(fg))
                        .fill(bg)
                        .rounding(Rounding::same(14.0));
                    if ui.add(btn).clicked() {
                        self.selected_playlist = Some(name.clone());
                    }
                    ui.add_space(4.0);
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            if let Some(sel_name) = self.selected_playlist.clone() {
                let tracks = self.library.playlists.get(&sel_name).cloned().unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new(format!("📂 {}", sel_name)).size(18.0).strong().color(Color32::WHITE));
                    ui.label(egui::RichText::new(format!("({} pistas)", tracks.len())).color(Color32::GRAY));

                    if !tracks.is_empty() {
                        ui.add_space(10.0);
                        if ui.add(egui::Button::new(egui::RichText::new("▶ Reproducir Playlist").strong().color(Color32::BLACK)).fill(accent)).clicked() {
                            self.queue = tracks.clone();
                            self.play_queue_index(0);
                            self.show_toast(format!("▶ Reproduciendo '{}'", sel_name));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if ui.button(egui::RichText::new("🗑 Eliminar Playlist").color(Color32::from_rgb(239, 68, 68))).clicked() {
                            to_delete = Some(sel_name.clone());
                        }
                    });
                });

                ui.add_space(8.0);
                if tracks.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Esta playlist está vacía. Usa el botón 📂 en cualquier canción para añadirla.").color(Color32::GRAY));
                    });
                } else {
                    self.render_track_table(ui, &tracks, accent);
                }
            }

            if let Some(del_name) = to_delete {
                self.library.delete_playlist(&del_name);
                self.selected_playlist = None;
                self.show_toast(format!("🗑 Playlist '{}' eliminada", del_name));
            }
        });
    }

    fn render_downloads_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("💾 Descargas Offline (Sin DRM)").size(22.0).strong().color(Color32::WHITE));
                ui.label(egui::RichText::new(format!("({} archivos locales)", self.library.downloads.len())).color(Color32::GRAY));

                if !self.library.downloads.is_empty() {
                    ui.add_space(10.0);
                    if ui.add(egui::Button::new(egui::RichText::new("▶ Reproducir Offline").strong().color(Color32::BLACK)).fill(accent)).clicked() {
                        self.queue = self.library.downloads.clone();
                        self.play_queue_index(0);
                        self.show_toast("💾 Reproduciendo biblioteca offline");
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    if ui.button("📁 Abrir Carpeta").on_hover_text("Abrir carpeta 'downloads' en el Explorador de Windows").clicked() {
                        let _ = std::fs::create_dir_all("downloads");
                        let _ = std::process::Command::new("explorer").arg("downloads").spawn();
                    }
                    ui.add_space(6.0);
                    if ui.button("🔄 Escanear").on_hover_text("Buscar archivos .m4a / .mp3 en la carpeta downloads").clicked() {
                        self.library.scan_local_downloads();
                        self.show_toast("💾 Carpeta de descargas actualizada");
                    }
                });
            });

            ui.add_space(10.0);

            if self.library.downloads.is_empty() {
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("No hay canciones descargadas todavía.").size(16.0).color(Color32::GRAY));
                    ui.label(egui::RichText::new("Haz clic en el icono ⬇ en cualquier canción para guardarla en formato M4A/AAC sin DRM.").color(Color32::DARK_GRAY));
                });
            } else {
                self.render_track_table(ui, &self.library.downloads.clone(), accent);
            }
        });
    }

    fn render_history_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("🕒 Historial de Reproducción").size(22.0).strong().color(Color32::WHITE));
            });

            ui.add_space(10.0);

            if self.library.history.is_empty() {
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Historial vacío. ¡Empieza a reproducir música!").size(16.0).color(Color32::GRAY));
                });
            } else {
                self.render_track_table(ui, &self.library.history.clone(), accent);
            }
        });
    }

    fn render_queue_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("📑 Cola de Reproducción").size(22.0).strong().color(Color32::WHITE));
                if ui.button("Vaciar Cola").clicked() {
                    self.queue.clear();
                    self.current_index = None;
                    self.audio.stop();
                }
            });

            ui.add_space(10.0);

            if self.queue.is_empty() {
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("La cola está vacía.").size(16.0).color(Color32::GRAY));
                });
            } else {
                self.render_track_table(ui, &self.queue.clone(), accent);
            }
        });
    }

    fn render_lyrics_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let curr_track = self.current_track().cloned();
        let status = self.audio.get_status();

        ui.add_space(12.0);

        // Header: Song info & Sync status
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            if let Some(ref t) = curr_track {
                ui.vertical(|ui| {
                    ui.heading(egui::RichText::new(&t.title).size(22.0).strong().color(Color32::WHITE));
                    ui.label(egui::RichText::new(&t.artist).size(13.0).color(Color32::from_rgb(180, 180, 180)));
                });
            } else {
                ui.heading(egui::RichText::new("🎤 Letras en Tiempo Real").size(22.0).strong().color(Color32::WHITE));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                if ui.button("🔄 Recargar Letra").on_hover_text("Volver a consultar LRCLIB").clicked() {
                    self.lyrics_data = None;
                    self.fetch_lyrics_for_current();
                }

                ui.add_space(8.0);
                if ui.small_button("⏩ +0.5s").on_hover_text("Adelantar sincronización de letra").clicked() {
                    self.lyrics_offset_secs += 0.5;
                }
                if ui.small_button(format!("{:+.1}s", self.lyrics_offset_secs)).on_hover_text("Reiniciar offset a 0.0s").clicked() {
                    self.lyrics_offset_secs = 0.0;
                }
                if ui.small_button("⏪ -0.5s").on_hover_text("Retrasar sincronización de letra").clicked() {
                    self.lyrics_offset_secs -= 0.5;
                }
            });
        });

        // Live 24-Bar Audio Spectrum Visualizer on Karaoke stage when playing
        if status.state == 3 {
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                let time = ui.ctx().input(|i| i.time);
                let num_bars = 24;
                let bar_w = 6.0_f32;
                let gap = 3.0_f32;
                let total_w = num_bars as f32 * (bar_w + gap);
                let (vis_rect, _) = ui.allocate_exact_size(Vec2::new(total_w, 22.0), egui::Sense::hover());
                for b in 0..num_bars {
                    let freq = 4.5 + (b % 7) as f64 * 1.7;
                    let phase = b as f64 * 0.45;
                    let h = (4.0 + 17.0 * ((time * freq + phase).sin().abs() as f32)).min(22.0);
                    let x = vis_rect.min.x + b as f32 * (bar_w + gap);
                    let y = vis_rect.max.y - h;
                    let r = egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(bar_w, h));
                    ui.painter().rect_filled(r, Rounding::same(2.0), accent);
                }
            });
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(10.0);

        if curr_track.is_none() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(crate::APP_LOGO_URI)
                        .fit_to_exact_size(Vec2::new(110.0, 110.0))
                        .rounding(Rounding::same(16.0)),
                );
                ui.add_space(14.0);
                ui.label(egui::RichText::new("🎧 Elige una canción para ver su letra en modo Karaoke").size(16.0).color(Color32::GRAY));
            });
            return;
        }

        if self.is_loading_lyrics {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.spinner();
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Buscando letra sincronizada en LRCLIB...").size(15.0).color(accent));
            });
            return;
        }

        if let Some(ref lyrics) = self.lyrics_data {
            if lyrics.lines.is_empty() {
                ui.add_space(60.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("No hay letra disponible para esta canción.").size(15.0).color(Color32::GRAY));
                });
                return;
            }

            let cur_pos = (status.position + self.lyrics_offset_secs).max(0.0);
            let mut active_idx = None;
            if lyrics.is_synced {
                for (i, line) in lyrics.lines.iter().enumerate() {
                    if line.time_seconds <= cur_pos {
                        active_idx = Some(i);
                    } else {
                        break;
                    }
                }
            }

            let mut seek_target = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);

                        for (i, line) in lyrics.lines.iter().enumerate() {
                            let is_active = active_idx == Some(i);
                            let text_color = if is_active {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(115, 115, 115)
                            };

                            let font_size = if is_active { 26.0 } else { 18.0 };
                            let mut rich = egui::RichText::new(&line.text)
                                .size(font_size)
                                .color(text_color);

                            if is_active {
                                rich = rich.strong();
                            }

                            let btn = if is_active {
                                egui::Button::new(rich)
                                    .fill(Color32::from_rgba_premultiplied(accent.r() / 7, accent.g() / 7, accent.b() / 7, 35))
                                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 75)))
                                    .rounding(Rounding::same(14.0))
                                    .min_size(Vec2::new((ui.available_width() - 40.0).max(200.0), font_size + 18.0))
                            } else {
                                egui::Button::new(rich)
                                    .frame(false)
                                    .min_size(Vec2::new((ui.available_width() - 40.0).max(200.0), font_size + 14.0))
                            };

                            let resp = ui.add(btn);

                            if is_active && lyrics.is_synced {
                                resp.scroll_to_me(Some(egui::Align::Center));
                            }

                            if resp.clicked() && lyrics.is_synced {
                                seek_target = Some(line.time_seconds);
                            }

                            ui.add_space(6.0);
                        }

                        ui.add_space(80.0);
                    });
                });

            if let Some(seek_time) = seek_target {
                self.audio.seek(seek_time);
            }
        } else {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No encontramos letra sincronizada para esta pista.").size(15.0).color(Color32::GRAY));
                ui.add_space(10.0);
                if ui.button("Intentar buscar de nuevo").clicked() {
                    self.fetch_lyrics_for_current();
                }
            });
        }
    }

    fn render_track_table(&mut self, ui: &mut egui::Ui, tracks: &[Track], accent: Color32) {
        let current_id = self.current_track().map(|t| t.id.clone());

        // Header Labels
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("#").size(12.0).color(Color32::from_rgb(120, 120, 120)));
            ui.add_space(20.0);
            ui.label(egui::RichText::new("PORTADA").size(12.0).color(Color32::from_rgb(120, 120, 120)));
            ui.add_space(24.0);
            ui.label(egui::RichText::new("TÍTULO").size(12.0).color(Color32::from_rgb(120, 120, 120)));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(30.0);
                ui.label(egui::RichText::new("DURACIÓN").size(12.0).color(Color32::from_rgb(120, 120, 120)));
                ui.add_space(50.0);
                ui.label(egui::RichText::new("ACCIONES").size(12.0).color(Color32::from_rgb(120, 120, 120)));
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        for (idx, track) in tracks.iter().enumerate() {
            let is_current = current_id.as_deref() == Some(&track.id);
            let row_bg = if is_current {
                Color32::from_rgb(30, 42, 34)
            } else if idx % 2 == 0 {
                Color32::from_rgb(22, 22, 22)
            } else {
                Color32::from_rgb(18, 18, 18)
            };

            let row_height = 54.0;
            let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 20.0, row_height), egui::Sense::click());

            let bg_color = if response.hovered() {
                Color32::from_rgb(42, 42, 42)
            } else {
                row_bg
            };

            ui.painter().rect_filled(rect, Rounding::same(6.0), bg_color);
            if response.hovered() {
                ui.painter().rect_stroke(rect, Rounding::same(6.0), egui::Stroke::new(1.0_f32, Color32::from_rgb(55, 55, 55)));
            }

            // Hover Prefetching: Start resolving audio stream as soon as cursor hovers!
            if response.hovered() {
                self.streamer.prefetch_stream_url(track);
            }

            let clicked_actions = response.hover_pos().map_or(false, |p| p.x > rect.max.x - 165.0);
            if (response.clicked() && !clicked_actions) || response.double_clicked() {
                self.play_track_now(track.clone());
            }

            // Draw track row content
            let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            row_ui.horizontal(|ui| {
                ui.add_space(10.0);

                // Play / Index Icon with real-time responsive state
                let num_str_buf = format!("{:02}", idx + 1);
                let (num_str, num_color) = if is_current {
                    if self.is_loading_stream {
                        ("⏳", accent)
                    } else if self.audio.get_status().state == 3 {
                        ("⏸", accent)
                    } else {
                        ("▶", accent)
                    }
                } else if response.hovered() {
                    ("▶", Color32::WHITE)
                } else {
                    (num_str_buf.as_str(), Color32::from_rgb(150, 150, 150))
                };
                ui.add_sized([24.0, row_height], egui::Label::new(egui::RichText::new(num_str).color(num_color).strong().size(13.0)));

                ui.add_space(8.0);

                // ALBUM COVER IMAGE THUMBNAIL (Fixed 44x44 container with Melodyfy Rust Logo default)
                let (thumb_rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), egui::Sense::hover());
                ui.painter().rect_filled(thumb_rect, Rounding::same(4.0), Color32::from_rgb(34, 34, 34));
                ui.put(
                    thumb_rect,
                    egui::Image::new(crate::APP_LOGO_URI)
                        .fit_to_exact_size(Vec2::new(44.0, 44.0))
                        .rounding(Rounding::same(4.0)),
                );

                if !track.thumbnail_url.is_empty() {
                    ui.put(
                        thumb_rect,
                        egui::Image::new(&track.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(44.0, 44.0))
                            .rounding(Rounding::same(4.0)),
                    );
                }

                ui.add_space(12.0);

                // Title & Artist
                ui.vertical(|ui| {
                    let title_color = if is_current { accent } else { Color32::WHITE };
                    let title_clipped = if track.title.chars().count() > 50 {
                        format!("{}...", track.title.chars().take(48).collect::<String>())
                    } else {
                        track.title.clone()
                    };
                    ui.label(egui::RichText::new(title_clipped).color(title_color).strong().size(13.5));
                    ui.label(egui::RichText::new(&track.artist).color(Color32::from_rgb(160, 160, 160)).size(11.5));
                });

                // Right aligned items
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(16.0);

                    // Duration
                    ui.label(egui::RichText::new(&track.duration).color(Color32::from_rgb(170, 170, 170)).size(12.5));

                    ui.add_space(16.0);

                    // Add to Queue button
                    if ui.small_button("➕").on_hover_text("Añadir a la cola").clicked() {
                        self.queue.push(track.clone());
                        self.show_toast("¡Añadido a la cola!");
                    }

                    ui.add_space(6.0);

                    // Add to Custom Playlist Menu / Remove if inside Playlists tab
                    let mut playlist_names: Vec<String> = self.library.playlists.keys().cloned().collect();
                    playlist_names.sort();
                    ui.menu_button("📂", |ui| {
                        ui.label(egui::RichText::new("Añadir a Playlist:").strong().size(11.5).color(accent));
                        ui.separator();
                        if playlist_names.is_empty() {
                            if ui.button("➕ Crear 'Mi Mix' y añadir").clicked() {
                                self.library.create_playlist("Mi Mix".to_string());
                                self.library.add_to_playlist("Mi Mix", track.clone());
                                self.show_toast("📂 Guardada en 'Mi Mix'");
                                ui.close_menu();
                            }
                        } else {
                            for pl_name in &playlist_names {
                                if ui.button(format!("📂 {}", pl_name)).clicked() {
                                    self.library.add_to_playlist(pl_name, track.clone());
                                    self.show_toast(format!("📂 Añadida a '{}'", pl_name));
                                    ui.close_menu();
                                }
                            }
                        }
                        if self.current_tab == ViewTab::Playlists {
                            if let Some(sel) = self.selected_playlist.clone() {
                                ui.separator();
                                if ui.button(egui::RichText::new("🗑 Quitar de esta playlist").color(Color32::from_rgb(239, 68, 68))).clicked() {
                                    self.library.remove_from_playlist(&sel, &track.id);
                                    self.show_toast("Quitada de la playlist");
                                    ui.close_menu();
                                }
                            }
                        }
                    });

                    ui.add_space(6.0);

                    // Favorite button
                    let is_fav = self.library.is_favorite(&track.id);
                    let heart = if is_fav { "💚" } else { "🤍" };
                    if ui.small_button(heart).on_hover_text("Marcar como favorita").clicked() {
                        let added = self.library.toggle_favorite(track.clone());
                        if added {
                            self.show_toast("Guardada en Tus Me Gusta");
                        }
                    }

                    ui.add_space(6.0);

                    // Offline Download button (Sin DRM)
                    if ui.small_button("⬇").on_hover_text("Descargar audio M4A/AAC (Sin DRM)").clicked() {
                        self.start_download(track.clone());
                    }
                });
            });

            ui.add_space(4.0);
        }
    }

    fn render_bottom_player_bar(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let status = self.audio.get_status();
        let curr_track = self.current_track().cloned();
        let full_rect = ui.available_rect_before_wrap();

        // 1px sleek Spotify divider line on top of player bar
        ui.painter().line_segment(
            [egui::pos2(full_rect.min.x, full_rect.min.y), egui::pos2(full_rect.max.x, full_rect.min.y)],
            egui::Stroke::new(1.0_f32, Color32::from_rgb(38, 38, 38)),
        );

        // Mathematically balanced 3-column layout anchored across the entire window
        let center_w = (full_rect.width() * 0.44).clamp(380.0, 620.0);
        let center_x = full_rect.center().x;
        let center_rect = egui::Rect::from_center_size(
            egui::pos2(center_x, full_rect.center().y),
            Vec2::new(center_w, full_rect.height()),
        );

        let left_rect = egui::Rect::from_min_max(
            full_rect.min,
            egui::pos2(center_rect.min.x - 8.0, full_rect.max.y),
        );

        let right_rect = egui::Rect::from_min_max(
            egui::pos2(center_rect.max.x + 8.0, full_rect.min.y),
            full_rect.max,
        );

        // ================= 1. LEFT SECTION (Cover, Title, Artist, Visualizer) =================
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(16.0);
                let (art_rect, _) = ui.allocate_exact_size(Vec2::new(56.0, 56.0), egui::Sense::hover());
                ui.painter().rect_filled(art_rect, Rounding::same(6.0), Color32::from_rgb(32, 32, 32));

                if let Some(ref t) = curr_track {
                    ui.put(
                        art_rect,
                        egui::Image::new(crate::APP_LOGO_URI)
                            .fit_to_exact_size(Vec2::new(56.0, 56.0))
                            .rounding(Rounding::same(6.0)),
                    );
                    if !t.thumbnail_url.is_empty() {
                        ui.put(
                            art_rect,
                            egui::Image::new(&t.thumbnail_url)
                                .fit_to_exact_size(Vec2::new(56.0, 56.0))
                                .rounding(Rounding::same(6.0)),
                        );
                    }

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        let title_clipped = if t.title.chars().count() > 24 {
                            format!("{}...", t.title.chars().take(22).collect::<String>())
                        } else {
                            t.title.clone()
                        };

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(title_clipped).strong().size(13.5).color(Color32::WHITE));

                            // Animated Equalizer Wave when playing
                            if status.state == 3 {
                                let time = ui.ctx().input(|i| i.time);
                                let (wave_rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 14.0), egui::Sense::hover());
                                let bar_w = 3.0;
                                let bar_gap = 1.5;
                                for b in 0..4 {
                                    let freq = 7.0 + b as f64 * 2.2;
                                    let h = (3.5 + 9.0 * ((time * freq).sin().abs() as f32)).min(14.0);
                                    let x = wave_rect.min.x + b as f32 * (bar_w + bar_gap);
                                    let y = wave_rect.max.y - h;
                                    let r = egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(bar_w, h));
                                    ui.painter().rect_filled(r, Rounding::same(1.0), accent);
                                }
                            }
                        });

                        let artist_clipped = if t.artist.chars().count() > 26 {
                            format!("{}...", t.artist.chars().take(24).collect::<String>())
                        } else {
                            t.artist.clone()
                        };
                        ui.label(egui::RichText::new(artist_clipped).size(11.0).color(Color32::from_rgb(170, 170, 170)));

                        if self.is_loading_stream {
                            ui.label(egui::RichText::new("⚡ Cargando audio instantáneo...").size(10.0).color(accent));
                        }
                    });

                    ui.add_space(8.0);
                    let is_fav = self.library.is_favorite(&t.id);
                    let heart = if is_fav { "💚" } else { "🤍" };
                    if ui.add(egui::Button::new(egui::RichText::new(heart).size(15.0)).frame(false)).on_hover_text("Me gusta").clicked() {
                        self.library.toggle_favorite(t.clone());
                    }

                    ui.add_space(6.0);
                    if ui.add(egui::Button::new(egui::RichText::new("⬇").size(14.0).color(Color32::from_rgb(180, 180, 180))).frame(false))
                        .on_hover_text("Descargar canción (Sin DRM)").clicked()
                    {
                        self.start_download(t.clone());
                    }
                } else {
                    ui.put(
                        art_rect,
                        egui::Image::new(crate::APP_LOGO_URI)
                            .fit_to_exact_size(Vec2::new(56.0, 56.0))
                            .rounding(Rounding::same(6.0)),
                    );
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Melodyfy Rust").strong().size(14.0).color(Color32::WHITE));
                        ui.label(egui::RichText::new("Elige una pista para comenzar").size(11.0).color(Color32::GRAY));
                    });
                }
            });
        });

        // ================= 2. CENTER SECTION (Balanced Controls & Wide Center Seekbar) =================
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(center_rect), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);

                // Row 1: Buttons (Symmetrically Centered)
                ui.horizontal(|ui| {
                    let btn_group_w = 176.0;
                    let pad = ((center_rect.width() - btn_group_w) / 2.0).max(0.0);
                    ui.add_space(pad);

                    // Shuffle Button
                    let shuffle_color = if self.is_shuffle { accent } else { Color32::from_rgb(160, 160, 160) };
                    if ui.add(egui::Button::new(egui::RichText::new("🔀").size(15.0).color(shuffle_color)).frame(false)).on_hover_text("Modo aleatorio").clicked() {
                        self.is_shuffle = !self.is_shuffle;
                    }

                    ui.add_space(8.0);

                    // Previous Button
                    if ui.add(egui::Button::new(egui::RichText::new("⏮").size(18.0).color(Color32::WHITE)).frame(false)).on_hover_text("Anterior").clicked() {
                        self.play_prev();
                    }

                    ui.add_space(8.0);

                    // Large Circular Hero Play/Pause Button
                    let is_playing = status.state == 3;
                    let play_symbol = if is_playing { "⏸" } else { "▶" };
                    let play_btn = egui::Button::new(egui::RichText::new(play_symbol).size(20.0).strong().color(Color32::BLACK))
                        .fill(accent)
                        .rounding(Rounding::same(21.0))
                        .min_size(Vec2::new(42.0, 42.0));

                    if ui.add(play_btn).clicked() {
                        self.audio.toggle_play_pause();
                    }

                    ui.add_space(8.0);

                    // Next Button
                    if ui.add(egui::Button::new(egui::RichText::new("⏭").size(18.0).color(Color32::WHITE)).frame(false)).on_hover_text("Siguiente").clicked() {
                        self.play_next();
                    }

                    ui.add_space(8.0);

                    // Repeat Button
                    let (repeat_symbol, repeat_color) = match self.repeat_mode {
                        RepeatMode::Off => ("🔁", Color32::from_rgb(150, 150, 150)),
                        RepeatMode::All => ("🔁", accent),
                        RepeatMode::One => ("🔂", accent),
                    };
                    if ui.add(egui::Button::new(egui::RichText::new(repeat_symbol).size(15.0).color(repeat_color)).frame(false)).on_hover_text("Repetir").clicked() {
                        self.repeat_mode = match self.repeat_mode {
                            RepeatMode::Off => RepeatMode::All,
                            RepeatMode::All => RepeatMode::One,
                            RepeatMode::One => RepeatMode::Off,
                        };
                    }
                });

                ui.add_space(4.0);

                // Row 2: Wide Seekbar with Time Labels (Symmetrically Centered)
                ui.horizontal(|ui| {
                    let cur_secs = status.position as u64;
                    let dur_secs = if status.duration > 0.0 {
                        status.duration as u64
                    } else if let Some(ref t) = curr_track {
                        t.duration_seconds
                    } else {
                        0
                    };

                    let max_dur = if dur_secs > 0 { dur_secs as f64 } else { 100.0 };
                    let time_label_w = 34.0;
                    let slider_w = (center_rect.width() - (time_label_w * 2.0 + 36.0)).clamp(220.0, 560.0);
                    let row_total_w = time_label_w + 10.0 + slider_w + 10.0 + time_label_w;
                    let pad = ((center_rect.width() - row_total_w) / 2.0).max(0.0);
                    ui.add_space(pad);

                    ui.add_sized(Vec2::new(time_label_w, 16.0), egui::Label::new(
                        egui::RichText::new(format!("{}:{:02}", cur_secs / 60, cur_secs % 60))
                            .size(11.0)
                            .color(Color32::from_rgb(170, 170, 170))
                    ));

                    if let Some(new_pos) = Self::custom_spotify_seekbar(ui, status.position, max_dur, slider_w, accent) {
                        if self.last_seek_time.elapsed() > std::time::Duration::from_millis(150) {
                            self.audio.seek(new_pos);
                            self.last_seek_time = Instant::now();
                        }
                    }

                    ui.add_sized(Vec2::new(time_label_w, 16.0), egui::Label::new(
                        egui::RichText::new(format!("{}:{:02}", dur_secs / 60, dur_secs % 60))
                            .size(11.0)
                            .color(Color32::from_rgb(170, 170, 170))
                    ));
                });
            });
        });

        // ================= 3. RIGHT SECTION (Volume Slider & Tools Docked to Window Right Edge) =================
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(14.0);

                // --- OBSIDIAN GLASS PILL TOOLBAR ---
                let pill_frame = egui::Frame::none()
                    .fill(Color32::from_rgb(24, 24, 24))
                    .rounding(Rounding::same(16.0))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 40)))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0));

                pill_frame.show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

                    // 1. PiP Video Button (High-End Pill)
                    let is_pip = self.is_pip_video_mode;
                    let pip_bg = if is_pip { accent } else { Color32::from_rgb(34, 34, 34) };
                    let pip_fg = if is_pip { Color32::BLACK } else { Color32::WHITE };
                    let pip_btn = egui::Button::new(
                        egui::RichText::new("📺 PiP Video").size(11.5).strong().color(pip_fg)
                    )
                    .fill(pip_bg)
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(72.0, 26.0));

                    if ui.add(pip_btn).on_hover_text("Modo Visor PiP Video para Juegos (Ventana Anclada 16:9, <30MB RAM)").clicked() {
                        self.toggle_pip_mode(ui.ctx());
                    }

                    // 2. Karaoke Lyrics Button (High-End Pill)
                    let is_lyrics = self.current_tab == ViewTab::Lyrics;
                    let lyrics_bg = if is_lyrics { accent } else { Color32::from_rgb(34, 34, 34) };
                    let lyrics_fg = if is_lyrics { Color32::BLACK } else { Color32::WHITE };
                    let lyrics_btn = egui::Button::new(
                        egui::RichText::new("🎤 Letras").size(11.5).strong().color(lyrics_fg)
                    )
                    .fill(lyrics_bg)
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(64.0, 26.0));

                    if ui.add(lyrics_btn).on_hover_text("Letras sincronizadas (Karaoke)").clicked() {
                        if self.current_tab == ViewTab::Lyrics {
                            self.current_tab = ViewTab::Home;
                        } else {
                            self.current_tab = ViewTab::Lyrics;
                            self.fetch_lyrics_for_current();
                        }
                    }

                    // 3. Mini-Player Button
                    let mini_btn = egui::Button::new(
                        egui::RichText::new("🕹️ Mini").size(11.0).color(Color32::from_rgb(220, 220, 220))
                    )
                    .fill(Color32::from_rgb(30, 30, 30))
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(52.0, 26.0));

                    if ui.add(mini_btn).on_hover_text("Modo Mini / 2º Plano Flotante (440x120)").clicked() {
                        self.is_mini_mode = !self.is_mini_mode;
                        let (new_size, on_top) = if self.is_mini_mode {
                            (Vec2::new(440.0, 120.0), true)
                        } else {
                            (Vec2::new(1150.0, 740.0), false)
                        };
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(new_size));
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(if on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        }));
                    }

                    // 4. DSP Speed Preset Pill
                    let is_custom_speed = (self.playback_rate - 1.0).abs() > 0.05;
                    let speed_bg = if is_custom_speed { Color32::from_rgba_premultiplied(accent.r() / 4, accent.g() / 4, accent.b() / 4, 180) } else { Color32::from_rgb(30, 30, 30) };
                    let speed_fg = if is_custom_speed { accent } else { Color32::from_rgb(200, 200, 200) };
                    let speed_label = format!("⚡ {:.2}x", self.playback_rate).replace(".00x", ".0x").replace("1.50x", "1.5x");
                    let speed_btn = egui::Button::new(
                        egui::RichText::new(speed_label).size(10.5).strong().color(speed_fg)
                    )
                    .fill(speed_bg)
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(52.0, 26.0));

                    if ui.add(speed_btn).on_hover_text("Efecto de Velocidad DSP: 1.0x Normal / 1.25x Nightcore / 1.5x Fast / 0.85x Slowed").clicked() {
                        self.cycle_playback_rate();
                    }

                    // 5. Radio Auto-mix Button
                    let radio_fg = if self.is_auto_radio { accent } else { Color32::from_rgb(150, 150, 150) };
                    if ui.add(egui::Button::new(egui::RichText::new("📻").size(13.0).color(radio_fg)).frame(false))
                        .on_hover_text("Radio Infinita Auto-Mix").clicked()
                    {
                        self.is_auto_radio = !self.is_auto_radio;
                        self.library.auto_radio = Some(self.is_auto_radio);
                        self.library.save();
                        let msg = if self.is_auto_radio { "📻 Radio Infinita ACTIVADA" } else { "📻 Radio Infinita DESACTIVADA" };
                        self.show_toast(msg);
                    }

                    // 6. Sleep Timer Button
                    let timer_active = self.sleep_timer_minutes > 0;
                    let timer_fg = if timer_active { accent } else { Color32::from_rgb(160, 160, 160) };
                    let timer_label = if let Some(end) = self.sleep_timer_end {
                        let rem_mins = (end.saturating_duration_since(Instant::now()).as_secs() / 60) + 1;
                        format!("🌙{}m", rem_mins)
                    } else {
                        "🌙".to_string()
                    };
                    if ui.add(egui::Button::new(egui::RichText::new(timer_label).size(11.5).color(timer_fg)).frame(false))
                        .on_hover_text("Temporizador de Sueño (Off / 15m / 30m / 45m / 60m)").clicked()
                    {
                        self.cycle_sleep_timer();
                    }

                    // 7. Queue Button
                    let q_color = if self.current_tab == ViewTab::Queue { accent } else { Color32::from_rgb(160, 160, 160) };
                    if ui.add(egui::Button::new(egui::RichText::new("📑").size(13.0).color(q_color)).frame(false))
                        .on_hover_text("Ver Cola de Reproducción").clicked()
                    {
                        self.current_tab = ViewTab::Queue;
                    }
                });

                ui.add_space(8.0);

                // --- VOLUME CLUSTER ---
                if let Some(new_vol) = Self::custom_spotify_volume_bar(ui, self.volume, 74.0, accent) {
                    self.volume = new_vol;
                    self.audio.set_volume(new_vol);
                    self.library.last_volume = Some(new_vol);
                    self.library.save();
                }

                ui.add_space(2.0);

                let vol_icon = if self.volume == 0 { "🔇" } else if self.volume < 50 { "🔉" } else { "🔊" };
                if ui.add(egui::Button::new(egui::RichText::new(vol_icon).size(13.5)).frame(false))
                    .on_hover_text(if self.volume == 0 { "Desmutear" } else { "Silenciar" }).clicked()
                {
                    if self.volume > 0 { self.volume = 0; } else { self.volume = 80; }
                    self.audio.set_volume(self.volume);
                }
            });
        });
    }

    fn custom_spotify_seekbar(
        ui: &mut egui::Ui,
        current_pos: f64,
        duration: f64,
        width: f32,
        accent: Color32,
    ) -> Option<f64> {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 16.0), egui::Sense::click_and_drag());
        let is_active = response.hovered() || response.dragged();
        let track_height = if is_active { 6.0_f32 } else { 4.0_f32 };
        let track_rect = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), track_height));

        // Background track rail
        ui.painter().rect_filled(track_rect, Rounding::same(track_height / 2.0), Color32::from_rgb(55, 55, 55));

        let ratio = if duration > 0.0 {
            (current_pos / duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        // Filled progress
        let filled_w = track_rect.width() * ratio;
        let filled_rect = egui::Rect::from_min_size(track_rect.min, Vec2::new(filled_w, track_height));
        let fill_color = if is_active { accent } else { Color32::from_rgb(220, 220, 220) };
        ui.painter().rect_filled(filled_rect, Rounding::same(track_height / 2.0), fill_color);

        // Circular Thumb knob (always visible, expands on hover/drag)
        let knob_radius = if is_active { 6.5 } else { 4.5 };
        let knob_center = egui::pos2(filled_rect.max.x.clamp(track_rect.min.x, track_rect.max.x), track_rect.center().y);
        ui.painter().circle_filled(knob_center, knob_radius, Color32::WHITE);

        // Interactive click / drag seek
        let mut seek_result = None;
        if response.clicked() || response.dragged() {
            if let Some(interact_pos) = response.interact_pointer_pos() {
                let rel_x = (interact_pos.x - track_rect.min.x).clamp(0.0, track_rect.width());
                let seek_fraction = (rel_x / track_rect.width()) as f64;
                seek_result = Some(seek_fraction * duration);
            }
        }

        // Hover timestamp tooltip
        if let Some(hover_pos) = response.hover_pos() {
            if duration > 0.0 {
                let rel_x = (hover_pos.x - track_rect.min.x).clamp(0.0, track_rect.width());
                let hover_secs = ((rel_x / track_rect.width()) as f64 * duration) as u64;
                response.on_hover_ui_at_pointer(|ui| {
                    ui.label(egui::RichText::new(format!("{}:{:02}", hover_secs / 60, hover_secs % 60)).size(11.0));
                });
            }
        }

        seek_result
    }

    fn custom_spotify_volume_bar(
        ui: &mut egui::Ui,
        current_vol: i32,
        width: f32,
        accent: Color32,
    ) -> Option<i32> {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 16.0), egui::Sense::click_and_drag());
        let is_active = response.hovered() || response.dragged();
        let track_height = if is_active { 6.0_f32 } else { 4.0_f32 };
        let track_rect = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), track_height));

        ui.painter().rect_filled(track_rect, Rounding::same(track_height / 2.0), Color32::from_rgb(55, 55, 55));

        let ratio = (current_vol as f32 / 100.0).clamp(0.0, 1.0);
        let filled_w = track_rect.width() * ratio;
        let filled_rect = egui::Rect::from_min_size(track_rect.min, Vec2::new(filled_w, track_height));
        let fill_color = if is_active { accent } else { Color32::from_rgb(220, 220, 220) };
        ui.painter().rect_filled(filled_rect, Rounding::same(track_height / 2.0), fill_color);

        // Circular Thumb knob (always visible, expands on hover/drag)
        let knob_radius = if is_active { 5.5 } else { 4.0 };
        let knob_center = egui::pos2(filled_rect.max.x.clamp(track_rect.min.x, track_rect.max.x), track_rect.center().y);
        ui.painter().circle_filled(knob_center, knob_radius, Color32::WHITE);

        let mut vol_result = None;
        if response.clicked() || response.dragged() {
            if let Some(interact_pos) = response.interact_pointer_pos() {
                let rel_x = (interact_pos.x - track_rect.min.x).clamp(0.0, track_rect.width());
                let vol = ((rel_x / track_rect.width()) * 100.0).round() as i32;
                vol_result = Some(vol.clamp(0, 100));
            }
        }

        if response.hovered() {
            response.on_hover_ui_at_pointer(|ui| {
                ui.label(egui::RichText::new(format!("Volumen: {}%", current_vol)).size(11.0));
            });
        }

        vol_result
    }

    fn render_mini_player_ui(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let status = self.audio.get_status();
        let curr_track = self.current_track().cloned();

        ui.vertical(|ui| {
            // Row 1: Header + Track title + Restore button
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                if let Some(ref t) = curr_track {
                    let (art_rect, _) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                    ui.painter().rect_filled(art_rect, Rounding::same(4.0), Color32::from_rgb(32, 32, 32));
                    ui.put(
                        art_rect,
                        egui::Image::new(crate::APP_LOGO_URI)
                            .fit_to_exact_size(Vec2::new(36.0, 36.0))
                            .rounding(Rounding::same(4.0)),
                    );
                    if !t.thumbnail_url.is_empty() {
                        ui.put(
                            art_rect,
                            egui::Image::new(&t.thumbnail_url)
                                .fit_to_exact_size(Vec2::new(36.0, 36.0))
                                .rounding(Rounding::same(4.0)),
                        );
                    }

                    ui.add_space(6.0);

                    ui.vertical(|ui| {
                        let title_clipped = if t.title.chars().count() > 22 {
                            format!("{}...", t.title.chars().take(20).collect::<String>())
                        } else {
                            t.title.clone()
                        };
                        ui.label(egui::RichText::new(title_clipped).strong().size(12.5).color(Color32::WHITE));
                        ui.label(egui::RichText::new(&t.artist).size(10.0).color(Color32::from_rgb(160, 160, 160)));
                    });
                } else {
                    ui.add(
                        egui::Image::new(crate::APP_LOGO_URI)
                            .fit_to_exact_size(Vec2::new(32.0, 32.0))
                            .rounding(Rounding::same(4.0)),
                    );
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Melodyfy Rust").strong().size(13.0).color(Color32::WHITE));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(6.0);
                    if ui.small_button("📺 PiP Video").on_hover_text("Cambiar a modo visor PiP 16:9 con video").clicked() {
                        self.is_mini_mode = false;
                        self.toggle_pip_mode(ui.ctx());
                    }
                    if ui.small_button("🪟 Maximizar").on_hover_text("Regresar a ventana completa (1150x740)").clicked() {
                        self.is_mini_mode = false;
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1150.0, 740.0)));
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                    }

                    ui.label(egui::RichText::new("🎧 2º Plano").size(10.0).color(accent).strong());
                });
            });

            ui.add_space(2.0);

            // Row 2: Controls & Seekbar
            ui.horizontal(|ui| {
                ui.add_space(6.0);

                if ui.small_button("⏮").clicked() { self.play_prev(); }

                let is_playing = status.state == 3;
                let play_sym = if is_playing { "⏸" } else { "▶" };
                if ui.add(egui::Button::new(egui::RichText::new(play_sym).strong().color(Color32::BLACK)).fill(accent)).clicked() {
                    self.audio.toggle_play_pause();
                }

                if ui.small_button("⏭").clicked() { self.play_next(); }

                ui.add_space(4.0);

                let cur_secs = status.position as u64;
                let dur_secs = if status.duration > 0.0 { status.duration as u64 } else if let Some(ref t) = curr_track { t.duration_seconds } else { 0 };
                let max_dur = if dur_secs > 0 { dur_secs as f64 } else { 100.0 };

                let slider_w = (ui.available_width() - 55.0).max(120.0);
                if let Some(new_pos) = Self::custom_spotify_seekbar(ui, status.position, max_dur, slider_w, accent) {
                    self.audio.seek(new_pos);
                }

                ui.label(egui::RichText::new(format!("{}:{:02}", cur_secs / 60, cur_secs % 60)).size(10.5).color(Color32::from_rgb(170, 170, 170)));
            });
        });
    }

    // =========================================================================================
    // =============================== VIDEO PiP GAMING MODE (16:9) ============================
    // =========================================================================================

    pub fn get_screen_corner_pos(corner: usize, win_w: f32, win_h: f32) -> egui::Pos2 {
        let (screen_w, screen_h) = unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
            let w = GetSystemMetrics(SM_CXSCREEN) as f32;
            let h = GetSystemMetrics(SM_CYSCREEN) as f32;
            (if w > 100.0 { w } else { 1920.0 }, if h > 100.0 { h } else { 1080.0 })
        };
        let margin = 20.0;
        let bottom_margin = 60.0;
        match corner {
            0 => egui::pos2((screen_w - win_w - margin).max(0.0), (screen_h - win_h - bottom_margin).max(0.0)), // Bottom-Right
            1 => egui::pos2((screen_w - win_w - margin).max(0.0), margin + 15.0),                                // Top-Right
            2 => egui::pos2(margin, (screen_h - win_h - bottom_margin).max(0.0)),                                // Bottom-Left
            _ => egui::pos2(margin, margin + 15.0),                                                               // Top-Left
        }
    }

    pub fn move_pip_video_to_corner(&self, corner: usize) {
        let pos = Self::get_screen_corner_pos(corner, 426.0, 240.0);
        let left = pos.x as i32;
        let top = pos.y as i32;

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{FindWindowA, SetWindowPos, HWND_TOPMOST, SWP_NOSIZE, SWP_NOACTIVATE};
            let hwnd = FindWindowA(windows::core::PCSTR::null(), windows::core::s!("Melodyfy Video PiP"));
            if hwnd.0 != 0 {
                let _ = SetWindowPos(hwnd, HWND_TOPMOST, left, top, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }

    pub fn apply_dark_titlebar_to_pip() -> bool {
        unsafe {
            extern "system" {
                fn LoadLibraryA(lpLibFileName: *const u8) -> isize;
                fn GetProcAddress(hModule: isize, lpProcName: *const u8) -> *const std::ffi::c_void;
                fn FindWindowA(lpClassName: *const u8, lpWindowName: *const u8) -> isize;
                fn LoadImageA(hInst: isize, name: *const u8, type_: u32, cx: i32, cy: i32, fuLoad: u32) -> isize;
                fn SendMessageA(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> isize;
            }

            let title = b"Melodyfy Video PiP\0";
            let hwnd = FindWindowA(std::ptr::null(), title.as_ptr());
            if hwnd != 0 {
                // 1. Set Official Melodyfy Icon on Window Titlebar
                let icon_path_buf = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("assets").join("logo.ico")))
                    .unwrap_or_else(|| std::path::PathBuf::from("assets/logo.ico"));
                let icon_str = format!("{}\0", icon_path_buf.to_string_lossy());
                let hicon = LoadImageA(0, icon_str.as_ptr(), 1 /* IMAGE_ICON */, 32, 32, 0x0010 /* LR_LOADFROMFILE */);
                if hicon != 0 {
                    let _ = SendMessageA(hwnd, 0x0080 /* WM_SETICON */, 0 /* ICON_SMALL */, hicon);
                    let _ = SendMessageA(hwnd, 0x0080 /* WM_SETICON */, 1 /* ICON_BIG */, hicon);
                }

                // 2. Set Immersive Dark Mode & Dark Chrome Colors via DWM (DwmSetWindowAttribute)
                let lib_name = b"dwmapi.dll\0";
                let h_dwm = LoadLibraryA(lib_name.as_ptr());
                if h_dwm != 0 {
                    let proc_name = b"DwmSetWindowAttribute\0";
                    let proc = GetProcAddress(h_dwm, proc_name.as_ptr());
                    if !proc.is_null() {
                        type FnDwm = unsafe extern "system" fn(isize, u32, *const std::ffi::c_void, u32) -> i32;
                        let set_attr: FnDwm = std::mem::transmute(proc);

                        let dark: i32 = 1;
                        // DWMWA_USE_IMMERSIVE_DARK_MODE (20 on Win 11 / 10 20H1+, 19 on older Win 10)
                        let _ = set_attr(hwnd, 20, &dark as *const _ as *const _, 4);
                        let _ = set_attr(hwnd, 19, &dark as *const _ as *const _, 4);

                        // DWMWA_CAPTION_COLOR (35) -> 0x00121212 (deep obsidian grey titlebar)
                        let caption: u32 = 0x00121212;
                        let _ = set_attr(hwnd, 35, &caption as *const _ as *const _, 4);

                        // DWMWA_TEXT_COLOR (36) -> 0x00FFFFFF (white title text)
                        let text: u32 = 0x00FFFFFF;
                        let _ = set_attr(hwnd, 36, &text as *const _ as *const _, 4);

                        // DWMWA_BORDER_COLOR (34) -> 0x00282828 (subtle dark border)
                        let border: u32 = 0x00282828;
                        let _ = set_attr(hwnd, 34, &border as *const _ as *const _, 4);
                    }
                }
                return true;
            }
            false
        }
    }

    pub fn toggle_pip_mode(&mut self, ctx: &egui::Context) {
        self.is_pip_video_mode = !self.is_pip_video_mode;
        if self.is_pip_video_mode {
            self.is_mini_mode = false;
            self.pip_last_interaction = Instant::now();
            let corner_pos = Self::get_screen_corner_pos(self.pip_corner, 426.0, 240.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(426.0, 240.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(240.0, 120.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(corner_pos));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("📺 Melodyfy — PiP Video Player".into()));

            self.start_pip_video_stream();
        } else {
            if let Some(mut child) = self.pip_video_child.take() {
                let _ = child.kill();
            }
            self.pip_active_track_id = None;
            self.audio.play();

            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1150.0, 740.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(320.0, 480.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("🎧 Melodyfy Rust — Reproductor Libre, Rápido y Sin Anuncios".into()));
        }
    }

    fn render_pip_video_ui(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let status = self.audio.get_status();
        let curr_track = self.current_track().cloned();
        let rect = ui.available_rect_before_wrap();

        // Detect hover or mouse activity to reset auto-hide timer
        let is_hovered = ui.rect_contains_pointer(rect);
        if is_hovered {
            self.pip_last_interaction = Instant::now();
        }
        let show_hud = self.pip_last_interaction.elapsed().as_secs_f32() < 2.5;

        // Keyboard navigation in PiP mode
        ui.ctx().input(|i| {
            if i.key_pressed(egui::Key::C) {
                self.pip_corner = (self.pip_corner + 1) % 4;
                let pos = Self::get_screen_corner_pos(self.pip_corner, 426.0, 240.0);
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
                self.move_pip_video_to_corner(self.pip_corner);
            }
            if i.key_pressed(egui::Key::O) {
                self.pip_opacity = match self.pip_opacity {
                    o if o > 0.9 => 0.85,
                    o if o > 0.75 => 0.70,
                    _ => 1.0,
                };
            }
        });

        // 1. VIDEO SURFACE & VISUALIZER
        // Dark cinematic background
        ui.painter().rect_filled(rect, Rounding::ZERO, Color32::from_rgb(12, 12, 12));

        if let Some(ref t) = curr_track {
            if !t.thumbnail_url.is_empty() {
                ui.put(
                    rect,
                    egui::Image::new(&t.thumbnail_url)
                        .fit_to_exact_size(rect.size())
                        .rounding(Rounding::ZERO),
                );
            } else {
                ui.put(
                    rect,
                    egui::Image::new(crate::APP_LOGO_URI)
                        .fit_to_exact_size(rect.size())
                        .rounding(Rounding::ZERO),
                );
            }

            // Real-time audio spectrum equalizer bars over bottom of video
            if status.state == 3 {
                let bar_count = 32;
                let bar_spacing = rect.width() / (bar_count as f32);
                let bar_width = (bar_spacing * 0.7).max(2.0);
                let base_y = rect.bottom() - 3.0;
                let time_val = status.position * 8.0;

                for idx in 0..bar_count {
                    let phase = (idx as f64) * 0.45;
                    let wave1 = (time_val + phase).sin().abs();
                    let wave2 = (time_val * 1.5 + phase * 2.0).cos().abs();
                    let h_factor = ((wave1 * 0.6 + wave2 * 0.4) as f32).clamp(0.08, 1.0);
                    let bar_h = h_factor * 26.0;

                    let bar_x = rect.left() + (idx as f32) * bar_spacing + (bar_spacing - bar_width) * 0.5;
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(bar_x, base_y - bar_h),
                        Vec2::new(bar_width, bar_h),
                    );
                    let bar_col = Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 140);
                    ui.painter().rect_filled(bar_rect, Rounding::same(1.5), bar_col);
                }
            }

            // Discreet badge when HUD is hidden
            if !show_hud {
                let is_video_active = self.pip_video_child.is_some();
                let badge_text = if is_video_active { "🔴 LIVE VIDEO" } else { "📺 PiP" };
                let badge_pos = rect.left_top() + Vec2::new(8.0, 8.0);
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(badge_pos, Vec2::new(if is_video_active { 82.0 } else { 48.0 }, 18.0)),
                    Rounding::same(4.0),
                    Color32::from_black_alpha(160),
                );
                ui.painter().text(
                    badge_pos + Vec2::new(if is_video_active { 41.0 } else { 24.0 }, 9.0),
                    egui::Align2::CENTER_CENTER,
                    badge_text,
                    FontId::proportional(10.0),
                    if is_video_active { Color32::from_rgb(255, 80, 80) } else { Color32::LIGHT_GRAY },
                );
            }
        } else {
            // Idle state
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.add(
                    egui::Image::new(crate::APP_LOGO_URI)
                        .fit_to_exact_size(Vec2::new(48.0, 48.0))
                        .rounding(Rounding::same(8.0)),
                );
                ui.add_space(8.0);
                ui.label(egui::RichText::new("📺 Melodyfy Visor PiP").strong().size(13.0).color(Color32::WHITE));
                ui.label(egui::RichText::new("Selecciona una pista para reproducir").size(10.5).color(Color32::GRAY));
            });
        }

        // 2. CINEMATIC OVERLAY HUD (Appears on hover)
        if show_hud {
            // Dark gradient backdrop
            ui.painter().rect_filled(rect, Rounding::ZERO, Color32::from_black_alpha(175));

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.vertical(|ui| {
                    ui.add_space(6.0);

                    // ================= TOP BAR (Draggable) =================
                    let top_bar_rect = egui::Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), 32.0));
                    let drag_resp = ui.interact(top_bar_rect, ui.id().with("pip_win_drag"), egui::Sense::drag());
                    if drag_resp.dragged() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }

                    ui.horizontal(|ui| {
                        ui.add_space(8.0);

                        // Quick corner snaps
                        let corners = [("↖", 3), ("↗", 1), ("↙", 2), ("↘", 0)];
                        for (sym, c_idx) in corners {
                            let is_curr = self.pip_corner == c_idx;
                            let btn_color = if is_curr { accent } else { Color32::WHITE };
                            if ui.add(egui::Button::new(egui::RichText::new(sym).size(11.0).color(btn_color)).frame(false))
                                .on_hover_text(format!("Anclar a esquina {}", sym)).clicked()
                            {
                                self.pip_corner = c_idx;
                                let pos = Self::get_screen_corner_pos(c_idx, 426.0, 240.0);
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
                                self.move_pip_video_to_corner(c_idx);
                            }
                        }

                        ui.add_space(4.0);

                        // Track title clipped
                        if let Some(ref t) = curr_track {
                            let title_clipped = if t.title.chars().count() > 18 {
                                format!("{}...", t.title.chars().take(16).collect::<String>())
                            } else {
                                t.title.clone()
                            };
                            ui.label(egui::RichText::new(title_clipped).strong().size(11.5).color(Color32::WHITE));
                        }

                        if self.pip_video_child.is_none() && curr_track.is_some() {
                            if ui.add(egui::Button::new(egui::RichText::new("▶ Conectar Video").size(10.0).strong().color(Color32::BLACK)).fill(accent).rounding(Rounding::same(6.0)))
                                .on_hover_text("Iniciar reproducción de video en vivo (ffplay)").clicked()
                            {
                                self.start_pip_video_stream();
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);

                            // Restore full window
                            if ui.add(egui::Button::new(egui::RichText::new("🪟 Full").size(11.0).color(Color32::WHITE)).frame(false))
                                .on_hover_text("Restaurar ventana completa (1150x740)").clicked()
                            {
                                self.toggle_pip_mode(ui.ctx());
                            }

                            // Switch to audio HUD mini mode
                            if ui.add(egui::Button::new(egui::RichText::new("🕹️ Mini").size(11.0).color(Color32::from_rgb(180, 180, 180))).frame(false))
                                .on_hover_text("Cambiar a barra de audio compacta (440x120)").clicked()
                            {
                                self.is_pip_video_mode = false;
                                self.is_mini_mode = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(440.0, 120.0)));
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title("🕹️ Melodyfy Mini Player".into()));
                            }

                            // Opacity toggle
                            let op_label = format!("👁️{}%", (self.pip_opacity * 100.0) as u32);
                            if ui.add(egui::Button::new(egui::RichText::new(op_label).size(10.5).color(Color32::from_rgb(190, 190, 190))).frame(false))
                                .on_hover_text("Cambiar opacidad (100% / 85% / 70%)").clicked()
                            {
                                self.pip_opacity = match self.pip_opacity {
                                    o if o > 0.9 => 0.85,
                                    o if o > 0.75 => 0.70,
                                    _ => 1.0,
                                };
                            }
                        });
                    });

                    // ================= CENTER CONTROLS =================
                    ui.add_space(38.0);
                    ui.horizontal_centered(|ui| {
                        let total_w = ui.available_width();
                        ui.add_space((total_w - 140.0) * 0.5);

                        if ui.add(egui::Button::new(egui::RichText::new("⏮").size(16.0).color(Color32::WHITE)).frame(false)).clicked() {
                            self.play_prev();
                        }

                        ui.add_space(8.0);

                        let is_playing = status.state == 3;
                        let play_sym = if is_playing { "⏸" } else { "▶" };
                        let play_btn = egui::Button::new(
                            egui::RichText::new(play_sym).strong().size(18.0).color(Color32::BLACK)
                        )
                        .fill(accent)
                        .rounding(Rounding::same(20.0))
                        .min_size(Vec2::new(40.0, 40.0));

                        if ui.add(play_btn).clicked() {
                            self.audio.toggle_play_pause();
                        }

                        ui.add_space(8.0);

                        if ui.add(egui::Button::new(egui::RichText::new("⏭").size(16.0).color(Color32::WHITE)).frame(false)).clicked() {
                            self.play_next();
                        }
                    });

                    // ================= BOTTOM BAR =================
                    ui.add_space(32.0);
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);

                        let cur_secs = status.position as u64;
                        let dur_secs = if status.duration > 0.0 {
                            status.duration as u64
                        } else if let Some(ref t) = curr_track {
                            t.duration_seconds
                        } else {
                            0
                        };
                        let max_dur = if dur_secs > 0 { dur_secs as f64 } else { 100.0 };

                        let seekbar_w = (ui.available_width() - 120.0).max(100.0);
                        if let Some(new_pos) = Self::custom_spotify_seekbar(ui, status.position, max_dur, seekbar_w, accent) {
                            self.audio.seek(new_pos);
                        }

                        let time_str = format!("{}:{:02}/{}:{:02}", cur_secs / 60, cur_secs % 60, dur_secs / 60, dur_secs % 60);
                        ui.label(egui::RichText::new(time_str).size(9.5).color(Color32::from_rgb(180, 180, 180)));

                        // Volume mute toggle
                        let vol_icon = if status.volume == 0 { "🔇" } else { "🔊" };
                        if ui.add(egui::Button::new(egui::RichText::new(vol_icon).size(11.0)).frame(false)).clicked() {
                            if self.volume > 0 {
                                self.volume = 0;
                            } else {
                                self.volume = 80;
                            }
                            self.audio.set_volume(self.volume);
                        }
                    });
                });
            });
        }
    }

    // =========================================================================================
    // =============================== SPOTIFY MOBILE EDITION (9:16) ============================
    // =========================================================================================

    fn render_mobile_ui(&mut self, ctx: &egui::Context, accent: Color32) {
        // Fullscreen Mobile Player View (BottomSheet / Now Playing Overlay)
        if self.show_mobile_fullscreen_player {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_mobile_fullscreen_player(ui, accent);
            });
            return;
        }

        // 1. Mobile Bottom Navigation Bar (Fixed at the bottom)
        egui::TopBottomPanel::bottom("mobile_bottom_nav")
            .exact_height(56.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let total_w = ui.available_width();
                    let col_w = total_w / 3.0;

                    let nav_tabs = [
                        (ViewTab::Home, "🏠", "Inicio"),
                        (ViewTab::Search, "🔍", "Buscar"),
                        (ViewTab::Favorites, "📚", "Biblioteca"),
                    ];

                    for (tab, icon, label) in nav_tabs {
                        let is_active = self.current_tab == tab;
                        let text_color = if is_active { Color32::WHITE } else { Color32::from_rgb(140, 140, 140) };
                        let icon_color = if is_active { accent } else { Color32::from_rgb(150, 150, 150) };

                        ui.allocate_ui_with_layout(
                            Vec2::new(col_w - 4.0, 48.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                if ui.add(egui::Button::new(
                                    egui::RichText::new(icon).size(18.0).color(icon_color)
                                ).frame(false)).clicked() {
                                    self.current_tab = tab;
                                }
                                if ui.add(egui::Button::new(
                                    egui::RichText::new(label).size(10.5).color(text_color)
                                ).frame(false)).clicked() {
                                    self.current_tab = tab;
                                }
                            },
                        );
                    }
                });
            });

        // 2. Sticky Floating Mini-Player (Right above Bottom Navigation)
        let curr_track = self.current_track().cloned();
        if let Some(ref t) = curr_track {
            egui::TopBottomPanel::bottom("mobile_sticky_mini")
                .exact_height(58.0)
                .show(ctx, |ui| {
                    self.render_mobile_sticky_mini_player(ui, t, accent);
                });
        }

        // 3. Mobile Top Header
        egui::TopBottomPanel::top("mobile_top_header")
            .exact_height(46.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(12.0);
                    ui.add(
                        egui::Image::new(crate::APP_LOGO_URI)
                            .fit_to_exact_size(Vec2::new(28.0, 28.0))
                            .rounding(Rounding::same(6.0)),
                    );
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Melodyfy Rust").strong().size(16.0).color(Color32::WHITE));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);

                        // Button to switch back to PC/Desktop format
                        if ui.add(egui::Button::new(egui::RichText::new("💻 PC").size(11.5).color(Color32::WHITE))
                            .fill(Color32::from_rgb(40, 40, 40))
                            .rounding(Rounding::same(12.0)))
                            .on_hover_text("Cambiar a formato Escritorio (1150x740)")
                            .clicked()
                        {
                            self.is_mobile_mode = false;
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1150.0, 740.0)));
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title("🎧 Melodyfy Rust".into()));
                        }

                        ui.add_space(6.0);

                        let theme_sym = if self.use_spotify_green { "🟢" } else { "🟠" };
                        if ui.small_button(theme_sym).on_hover_text("Cambiar color de acento").clicked() {
                            self.use_spotify_green = !self.use_spotify_green;
                        }
                    });
                });
            });

        // 4. Central Mobile Content
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                match self.current_tab {
                    ViewTab::Home => self.render_mobile_home_view(ui, accent),
                    ViewTab::Search => self.render_mobile_search_view(ui, accent),
                    ViewTab::Favorites | ViewTab::Playlists | ViewTab::Downloads | ViewTab::History | ViewTab::Queue => {
                        self.render_mobile_library_view(ui, accent)
                    }
                    ViewTab::Lyrics => self.render_lyrics_view(ui, accent),
                }
            });
        });
    }

    fn render_mobile_sticky_mini_player(&mut self, ui: &mut egui::Ui, track: &Track, accent: Color32) {
        let status = self.audio.get_status();
        let full_rect = ui.available_rect_before_wrap();

        let card_rect = egui::Rect::from_min_max(
            egui::pos2(full_rect.min.x + 6.0, full_rect.min.y + 2.0),
            egui::pos2(full_rect.max.x - 6.0, full_rect.max.y - 2.0),
        );

        // Card surface with 8px rounding
        ui.painter().rect_filled(card_rect, Rounding::same(8.0), Color32::from_rgb(34, 34, 34));

        // Thin 2px bottom progress bar
        let dur = if status.duration > 0.0 { status.duration } else { track.duration_seconds as f64 };
        let ratio = if dur > 0.0 { (status.position / dur).clamp(0.0, 1.0) as f32 } else { 0.0 };
        let prog_rect = egui::Rect::from_min_size(
            egui::pos2(card_rect.min.x + 4.0, card_rect.max.y - 2.5),
            Vec2::new((card_rect.width() - 8.0) * ratio, 2.0),
        );
        ui.painter().rect_filled(prog_rect, Rounding::same(1.0), accent);

        // Clicking anywhere on card opens full-screen player
        let click_box = ui.allocate_rect(card_rect, egui::Sense::click());
        if click_box.clicked() {
            self.show_mobile_fullscreen_player = true;
        }

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(card_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);

                // Miniature Thumbnail 40x40
                let (thumb_rect, _) = ui.allocate_exact_size(Vec2::new(40.0, 40.0), egui::Sense::hover());
                ui.painter().rect_filled(thumb_rect, Rounding::same(5.0), Color32::from_rgb(20, 20, 20));
                ui.put(
                    thumb_rect,
                    egui::Image::new(crate::APP_LOGO_URI)
                        .fit_to_exact_size(Vec2::new(40.0, 40.0))
                        .rounding(Rounding::same(5.0)),
                );
                if !track.thumbnail_url.is_empty() {
                    ui.put(
                        thumb_rect,
                        egui::Image::new(&track.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(40.0, 40.0))
                            .rounding(Rounding::same(5.0)),
                    );
                }

                ui.add_space(8.0);

                // Title + Artist
                ui.vertical(|ui| {
                    let title_clipped = if track.title.chars().count() > 18 {
                        format!("{}...", track.title.chars().take(16).collect::<String>())
                    } else {
                        track.title.clone()
                    };
                    ui.label(egui::RichText::new(title_clipped).strong().size(12.5).color(Color32::WHITE));

                    let artist_clipped = if track.artist.chars().count() > 20 {
                        format!("{}...", track.artist.chars().take(18).collect::<String>())
                    } else {
                        track.artist.clone()
                    };
                    ui.label(egui::RichText::new(artist_clipped).size(10.5).color(Color32::from_rgb(160, 160, 160)));
                });

                // Right controls: Heart + Play/Pause
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);

                    // Play/Pause button
                    let is_playing = status.state == 3;
                    let play_sym = if is_playing { "⏸" } else { "▶" };
                    if ui.add(egui::Button::new(egui::RichText::new(play_sym).size(16.0).color(Color32::WHITE)).frame(false)).clicked() {
                        self.audio.toggle_play_pause();
                    }

                    ui.add_space(8.0);

                    // Heart button
                    let is_fav = self.library.is_favorite(&track.id);
                    let heart = if is_fav { "💚" } else { "🤍" };
                    if ui.add(egui::Button::new(egui::RichText::new(heart).size(14.0)).frame(false)).clicked() {
                        self.library.toggle_favorite(track.clone());
                    }
                });
            });
        });
    }

    fn render_mobile_fullscreen_player(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let status = self.audio.get_status();
        let curr_track = self.current_track().cloned();

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);

            // Top Header: Down arrow ⌄ + Title
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if ui.add(egui::Button::new(egui::RichText::new("⌄").size(28.0).strong().color(Color32::WHITE)).frame(false))
                    .on_hover_text("Minimizar").clicked()
                {
                    self.show_mobile_fullscreen_player = false;
                }

                let center_w = ui.available_width() - 48.0;
                ui.allocate_ui_with_layout(Vec2::new(center_w, 36.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("REPRODUCIENDO").size(9.0).color(Color32::GRAY));
                    ui.label(egui::RichText::new("Melodyfy Mix").size(12.0).strong().color(Color32::WHITE));
                });
            });

            ui.add_space(16.0);

            // Large Album Artwork 280x280 px centered
            let art_w = (ui.available_width() - 48.0).clamp(240.0, 310.0);
            let (art_rect, _) = ui.allocate_exact_size(Vec2::new(art_w, art_w), egui::Sense::hover());
            ui.painter().rect_filled(art_rect, Rounding::same(12.0), Color32::from_rgb(28, 28, 28));
            ui.put(
                art_rect,
                egui::Image::new(crate::APP_LOGO_URI)
                    .fit_to_exact_size(Vec2::new(art_w, art_w))
                    .rounding(Rounding::same(12.0)),
            );

            if let Some(ref t) = curr_track {
                if !t.thumbnail_url.is_empty() {
                    ui.put(
                        art_rect,
                        egui::Image::new(&t.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(art_w, art_w))
                            .rounding(Rounding::same(12.0)),
                    );
                }
            }

            ui.add_space(20.0);

            // Title & Artist + Favorite
            if let Some(ref t) = curr_track {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        let title_clipped = if t.title.chars().count() > 24 {
                            format!("{}...", t.title.chars().take(22).collect::<String>())
                        } else {
                            t.title.clone()
                        };
                        ui.label(egui::RichText::new(title_clipped).size(17.0).strong().color(Color32::WHITE));

                        let artist_clipped = if t.artist.chars().count() > 28 {
                            format!("{}...", t.artist.chars().take(26).collect::<String>())
                        } else {
                            t.artist.clone()
                        };
                        ui.label(egui::RichText::new(artist_clipped).size(13.0).color(Color32::from_rgb(175, 175, 175)));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(24.0);
                        let is_fav = self.library.is_favorite(&t.id);
                        let heart = if is_fav { "💚" } else { "🤍" };
                        if ui.add(egui::Button::new(egui::RichText::new(heart).size(22.0)).frame(false)).clicked() {
                            self.library.toggle_favorite(t.clone());
                        }
                    });
                });
            }

            ui.add_space(14.0);

            // Full-width Seekbar
            let cur_secs = status.position as u64;
            let dur_secs = if status.duration > 0.0 {
                status.duration as u64
            } else if let Some(ref t) = curr_track {
                t.duration_seconds
            } else {
                0
            };
            let max_dur = if dur_secs > 0 { dur_secs as f64 } else { 100.0 };

            let seek_w = ui.available_width() - 48.0;
            if let Some(new_pos) = Self::custom_spotify_seekbar(ui, status.position, max_dur, seek_w, accent) {
                if self.last_seek_time.elapsed() > std::time::Duration::from_millis(150) {
                    self.audio.seek(new_pos);
                    self.last_seek_time = Instant::now();
                }
            }

            // Timestamps
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(egui::RichText::new(format!("{}:{:02}", cur_secs / 60, cur_secs % 60)).size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(24.0);
                    ui.label(egui::RichText::new(format!("{}:{:02}", dur_secs / 60, dur_secs % 60)).size(11.0).color(Color32::GRAY));
                });
            });

            ui.add_space(18.0);

            // Mobile Controls: Shuffle, Prev, Giant Play/Pause (58x58), Next, Repeat
            ui.horizontal_centered(|ui| {
                let controls_w = 260.0_f32;
                let pad = ((ui.available_width() - controls_w) / 2.0).max(0.0);
                ui.add_space(pad);

                // Shuffle
                let shuffle_col = if self.is_shuffle { accent } else { Color32::GRAY };
                if ui.add(egui::Button::new(egui::RichText::new("🔀").size(20.0).color(shuffle_col)).frame(false)).clicked() {
                    self.is_shuffle = !self.is_shuffle;
                }

                ui.add_space(16.0);

                // Prev
                if ui.add(egui::Button::new(egui::RichText::new("⏮").size(24.0).color(Color32::WHITE)).frame(false)).clicked() {
                    self.play_prev();
                }

                ui.add_space(16.0);

                // Giant Circular Play/Pause (58x58)
                let is_playing = status.state == 3;
                let play_sym = if is_playing { "⏸" } else { "▶" };
                let play_btn = egui::Button::new(egui::RichText::new(play_sym).size(24.0).strong().color(Color32::BLACK))
                    .fill(accent)
                    .rounding(Rounding::same(29.0))
                    .min_size(Vec2::new(58.0, 58.0));

                if ui.add(play_btn).clicked() {
                    self.audio.toggle_play_pause();
                }

                ui.add_space(16.0);

                // Next
                if ui.add(egui::Button::new(egui::RichText::new("⏭").size(24.0).color(Color32::WHITE)).frame(false)).clicked() {
                    self.play_next();
                }

                ui.add_space(16.0);

                // Repeat
                let (rep_sym, rep_col) = match self.repeat_mode {
                    RepeatMode::Off => ("🔁", Color32::GRAY),
                    RepeatMode::All => ("🔁", accent),
                    RepeatMode::One => ("🔂", accent),
                };
                if ui.add(egui::Button::new(egui::RichText::new(rep_sym).size(20.0).color(rep_col)).frame(false)).clicked() {
                    self.repeat_mode = match self.repeat_mode {
                        RepeatMode::Off => RepeatMode::All,
                        RepeatMode::All => RepeatMode::One,
                        RepeatMode::One => RepeatMode::Off,
                    };
                }
            });

            ui.add_space(22.0);

            // Bottom Tools Row: Speaker & Volume + Queue button
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let vol_icon = if self.volume == 0 { "🔇" } else if self.volume < 50 { "🔉" } else { "🔊" };
                if ui.add(egui::Button::new(egui::RichText::new(vol_icon).size(15.0)).frame(false)).clicked() {
                    if self.volume > 0 { self.volume = 0; } else { self.volume = 80; }
                    self.audio.set_volume(self.volume);
                }

                ui.add_space(6.0);
                if let Some(new_vol) = Self::custom_spotify_volume_bar(ui, self.volume, 120.0, accent) {
                    self.volume = new_vol;
                    self.audio.set_volume(new_vol);
                    self.library.last_volume = Some(new_vol);
                    self.library.save();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(24.0);
                    if ui.add(egui::Button::new(egui::RichText::new("📑").size(18.0).color(Color32::WHITE)).frame(false))
                        .on_hover_text("Ver Cola").clicked()
                    {
                        self.show_mobile_fullscreen_player = false;
                        self.current_tab = ViewTab::Queue;
                    }

                    ui.add_space(12.0);

                    if ui.add(egui::Button::new(egui::RichText::new("🎤").size(18.0).color(accent)).frame(false))
                        .on_hover_text("Ver Letras Sincronizadas").clicked()
                    {
                        self.show_mobile_fullscreen_player = false;
                        self.current_tab = ViewTab::Lyrics;
                        self.fetch_lyrics_for_current();
                    }

                    ui.add_space(12.0);

                    if let Some(ref t) = curr_track {
                        if ui.add(egui::Button::new(egui::RichText::new("⬇").size(18.0).color(Color32::WHITE)).frame(false))
                            .on_hover_text("Descargar Pista sin DRM").clicked()
                        {
                            self.start_download(t.clone());
                        }
                    }
                });
            });
        });
    }

    fn render_mobile_home_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.add_space(10.0);

        // Quick Category Chips (Horizontal Scroll)
        let quick_tags = self.library.get_quick_tags();
        ui.horizontal_wrapped(|ui| {
            ui.add_space(12.0);
            for tag in quick_tags.iter().take(6) {
                let is_active = self.search_query == tag.query;
                let bg_color = if is_active { accent } else { Color32::from_rgb(34, 34, 34) };
                let text_color = if is_active { Color32::BLACK } else { Color32::from_rgb(220, 220, 220) };

                let chip_btn = egui::Button::new(egui::RichText::new(&tag.label).size(11.5).strong().color(text_color))
                    .fill(bg_color)
                    .rounding(Rounding::same(14.0));

                if ui.add(chip_btn).clicked() {
                    self.search_query = tag.query.clone();
                    self.current_tab = ViewTab::Search;
                    self.trigger_search(&tag.query);
                }
                ui.add_space(4.0);
            }
        });

        ui.add_space(14.0);

        // 2x2 Curated Mixes (Spotify Mobile Card Style)
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(egui::RichText::new("TUS MIXES").strong().size(13.0).color(Color32::WHITE));
        });

        let mixes = [
            ("🎧 Lo-Fi Chill", "Lo-Fi Gaming Chill Beats"),
            ("⚡ Synthwave", "Synthwave Cyberpunk 80s"),
            ("🎸 Rock Clásico", "Rock Clásico 80s 90s"),
            ("🔥 Top Global", "Top Éxitos Mundiales 2026"),
        ];

        let col_w = (ui.available_width() - 28.0) / 2.0;
        for chunk in mixes.chunks(2) {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                for (title, query) in chunk {
                    let card = egui::Button::new(egui::RichText::new(*title).strong().size(12.0).color(Color32::WHITE))
                        .fill(Color32::from_rgb(38, 38, 38))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(col_w, 42.0));

                    if ui.add(card).clicked() {
                        self.search_query = query.to_string();
                        self.current_tab = ViewTab::Search;
                        self.trigger_search(query);
                    }
                    ui.add_space(4.0);
                }
            });
            ui.add_space(4.0);
        }

        ui.add_space(14.0);

        // Trending Tracks Section
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(egui::RichText::new("RECOMENDADO PARA TI").strong().size(13.0).color(Color32::WHITE));
        });

        let tracks = self.search_results.clone();
        for (idx, track) in tracks.iter().enumerate().take(15) {
            self.render_mobile_track_row(ui, track, idx, accent);
        }
    }

    fn render_mobile_search_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.add_space(10.0);

        // Mobile Search Bar
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            let input_w = ui.available_width() - 70.0;
            let response = ui.add_sized(
                [input_w, 36.0],
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 ¿Qué te apetece escuchar?")
                    .font(FontId::proportional(13.0))
            );

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let q = self.search_query.clone();
                self.trigger_search(&q);
            }

            if ui.add(egui::Button::new(egui::RichText::new("Buscar").strong().size(12.0).color(Color32::BLACK))
                .fill(accent)
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(50.0, 36.0)))
                .clicked()
            {
                let q = self.search_query.clone();
                self.trigger_search(&q);
            }
        });

        ui.add_space(10.0);

        if self.is_searching {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.spinner();
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Buscando en YouTube...").color(accent));
            });
        }

        let tracks = self.search_results.clone();
        for (idx, track) in tracks.iter().enumerate() {
            self.render_mobile_track_row(ui, track, idx, accent);
        }
    }

    fn render_mobile_library_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.add_space(10.0);

        // Subtab pills: Me Gusta / Historial / Cola
        ui.horizontal(|ui| {
            ui.add_space(12.0);

            let subtabs = [
                (0, format!("💚 Me Gusta ({})", self.library.favorites.len())),
                (1, format!("💾 Offline ({})", self.library.downloads.len())),
                (2, "🕒 Historial".to_string()),
                (3, format!("📑 Cola ({})", self.queue.len())),
            ];

            for (idx, label) in subtabs {
                let is_active = self.mobile_library_subtab == idx;
                let bg_col = if is_active { accent } else { Color32::from_rgb(32, 32, 32) };
                let txt_col = if is_active { Color32::BLACK } else { Color32::WHITE };

                let pill = egui::Button::new(egui::RichText::new(label).size(10.5).strong().color(txt_col))
                    .fill(bg_col)
                    .rounding(Rounding::same(12.0));

                if ui.add(pill).clicked() {
                    self.mobile_library_subtab = idx;
                    if idx == 1 {
                        self.library.scan_local_downloads();
                    }
                }
                ui.add_space(4.0);
            }
        });

        ui.add_space(12.0);

        match self.mobile_library_subtab {
            0 => {
                let favs = self.library.favorites.clone();
                if favs.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Aún no tienes canciones favoritas. Pulsa 🤍 para guardar.").color(Color32::GRAY));
                    });
                }
                for (idx, track) in favs.iter().enumerate() {
                    self.render_mobile_track_row(ui, track, idx, accent);
                }
            }
            1 => {
                let downs = self.library.downloads.clone();
                if downs.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("No hay descargas offline todavía. Usa ⬇ para guardar.").color(Color32::GRAY));
                    });
                }
                for (idx, track) in downs.iter().enumerate() {
                    self.render_mobile_track_row(ui, track, idx, accent);
                }
            }
            2 => {
                let hist = self.library.history.clone();
                for (idx, track) in hist.iter().enumerate() {
                    self.render_mobile_track_row(ui, track, idx, accent);
                }
            }
            _ => {
                let q = self.queue.clone();
                for (idx, track) in q.iter().enumerate() {
                    self.render_mobile_track_row(ui, track, idx, accent);
                }
            }
        }
    }

    fn render_mobile_track_row(&mut self, ui: &mut egui::Ui, track: &Track, idx: usize, accent: Color32) {
        let is_current = self.current_track().map_or(false, |t| t.id == track.id);
        let row_height = 54.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_height), egui::Sense::click());

        let bg_color = if is_current {
            Color32::from_rgb(26, 40, 30)
        } else if response.hovered() {
            Color32::from_rgb(34, 34, 34)
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, Rounding::same(6.0), bg_color);

        if response.clicked() {
            self.play_track_now(track.clone());
        }

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);

                // Index or Playing indicator
                let num_str = format!("{:02}", idx + 1);
                let (icon_sym, icon_col) = if is_current {
                    if self.audio.get_status().state == 3 {
                        ("⏸", accent)
                    } else {
                        ("▶", accent)
                    }
                } else {
                    (num_str.as_str(), Color32::GRAY)
                };
                ui.add_sized([18.0, row_height], egui::Label::new(egui::RichText::new(icon_sym).size(11.0).color(icon_col)));

                ui.add_space(6.0);

                // Thumbnail 42x42
                let (thumb_rect, _) = ui.allocate_exact_size(Vec2::new(42.0, 42.0), egui::Sense::hover());
                ui.painter().rect_filled(thumb_rect, Rounding::same(4.0), Color32::from_rgb(24, 24, 24));
                ui.put(
                    thumb_rect,
                    egui::Image::new(crate::APP_LOGO_URI)
                        .fit_to_exact_size(Vec2::new(42.0, 42.0))
                        .rounding(Rounding::same(4.0)),
                );
                if !track.thumbnail_url.is_empty() {
                    ui.put(
                        thumb_rect,
                        egui::Image::new(&track.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(42.0, 42.0))
                            .rounding(Rounding::same(4.0)),
                    );
                }

                ui.add_space(8.0);

                // Title + Artist
                ui.vertical(|ui| {
                    let title_col = if is_current { accent } else { Color32::WHITE };
                    let title_clipped = if track.title.chars().count() > 22 {
                        format!("{}...", track.title.chars().take(20).collect::<String>())
                    } else {
                        track.title.clone()
                    };
                    ui.label(egui::RichText::new(title_clipped).strong().size(12.5).color(title_col));

                    let artist_clipped = if track.artist.chars().count() > 24 {
                        format!("{}...", track.artist.chars().take(22).collect::<String>())
                    } else {
                        track.artist.clone()
                    };
                    ui.label(egui::RichText::new(artist_clipped).size(10.5).color(Color32::from_rgb(160, 160, 160)));
                });

                // Favorite button on right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    let is_fav = self.library.is_favorite(&track.id);
                    let heart = if is_fav { "💚" } else { "🤍" };
                    if ui.add(egui::Button::new(egui::RichText::new(heart).size(15.0)).frame(false)).clicked() {
                        self.library.toggle_favorite(track.clone());
                    }
                });
            });
        });
    }
}
