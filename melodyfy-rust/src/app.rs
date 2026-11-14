use crate::audio::AudioEngine;
use crate::models::{AppLibrary, RepeatMode, Track};
use crate::streamer::Streamer;
use eframe::egui::{self, Color32, FontId, Rounding, Vec2};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewTab {
    Home,
    Search,
    Favorites,
    History,
    Queue,
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

    // Notification toast
    toast_message: Option<(String, Instant)>,

    // Mini Player
    is_mini_mode: bool,

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
}

impl MelodyfyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, start_in_mobile: bool) -> Self {
        let audio = Arc::new(AudioEngine::new());
        let streamer = Arc::new(Streamer::new());
        let library = AppLibrary::load();
        let volume = library.last_volume.unwrap_or(100);
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
            toast_message: None,
            is_mini_mode: false,
            use_spotify_green: true,
            is_editing_tags: false,
            show_add_tag_dialog: false,
            new_tag_label: String::new(),
            new_tag_query: String::new(),
            is_mobile_mode: start_in_mobile,
            show_mobile_fullscreen_player: false,
            mobile_library_subtab: 0,
            popular_category_filter: 0,
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
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|i| self.queue.get(i))
    }

    pub fn play_next(&mut self) {
        if self.queue.is_empty() {
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
                            self.audio.play_url(stream_url);
                        } else {
                            self.show_toast("Error al cargar stream de audio");
                        }
                    }
                }
            }
        }

        self.check_auto_advance();
        self.handle_shortcuts(ctx);

        let accent = self.accent_color();

        // High fidelity Spotify Dark theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(18, 18, 18);
        visuals.window_fill = Color32::from_rgb(18, 18, 18);
        visuals.selection.bg_fill = accent;
        visuals.hyperlink_color = accent;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 40, 40);
        visuals.widgets.active.bg_fill = Color32::from_rgb(50, 50, 50);
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
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::new(12.0, 4.0);
                self.render_bottom_player_bar(ui, accent);
            });

        // LEFT SIDEBAR
        egui::SidePanel::left("left_sidebar")
            .exact_width(230.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::new(0.0, 8.0);
                self.render_sidebar(ui, accent);
            });

        // CENTRAL MAIN AREA
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_main_content(ui, accent);
        });
    }
}

impl MelodyfyApp {
    fn render_sidebar(&mut self, ui: &mut egui::Ui, accent: Color32) {
        ui.add_space(16.0);

        // App Branding
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(egui::RichText::new("🟢").size(22.0).color(accent));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Melodyfy").strong().size(20.0).color(Color32::WHITE));
                ui.label(egui::RichText::new("SPOTIFY KILLER • RUST").size(9.0).color(accent).strong());
            });
        });

        ui.add_space(16.0);

        // Navigation Tabs (Spotify Pill Style)
        let tabs = [
            (ViewTab::Home, "🏠", "Inicio"),
            (ViewTab::Search, "🔍", "Buscar"),
            (ViewTab::Favorites, "💚", "Tus Me Gusta"),
            (ViewTab::History, "🕒", "Historial"),
            (ViewTab::Queue, "📑", "Cola Actual"),
        ];

        for (tab, icon, label) in tabs {
            let is_active = self.current_tab == tab;
            let bg = if is_active { Color32::from_rgb(40, 40, 40) } else { Color32::TRANSPARENT };
            let text_color = if is_active { Color32::WHITE } else { Color32::from_rgb(179, 179, 179) };

            let content = egui::RichText::new(format!("  {}   {}", icon, label))
                .size(14.0)
                .strong()
                .color(text_color);

            let btn = egui::Button::new(content)
                .fill(bg)
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(200.0, 40.0));

            ui.horizontal(|ui| {
                ui.add_space(12.0);
                if ui.add(btn).clicked() {
                    self.current_tab = tab;
                }
            });
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        // Quick Curated Playlists
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(egui::RichText::new("MIXES & GÉNEROS").size(10.5).color(Color32::from_rgb(120, 120, 120)).strong());
        });

        let genres = [
            ("🎧 Lo-Fi Chill Gaming", "Lo-Fi Gaming Chill Beats"),
            ("⚡ Synthwave & Cyberpunk", "Synthwave Cyberpunk 80s"),
            ("🎸 Rock Legendario", "Rock Clásico 80s 90s"),
            ("🔥 Top Global 2026", "Top Éxitos Mundiales 2026"),
            ("🌸 Anime Openings", "Anime Openings HD"),
            ("🚗 Phonk Drift Music", "Phonk Drift Bass boosted"),
        ];

        for (display, query) in genres {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if ui.link(egui::RichText::new(display).size(12.5).color(Color32::from_rgb(190, 190, 190))).clicked() {
                    self.search_query = query.to_string();
                    self.current_tab = ViewTab::Search;
                    self.trigger_search(query);
                }
            });
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(12.0);

            // Switch to Mobile Format (390x820)
            if ui.add(egui::Button::new(egui::RichText::new("📱 Modo Móvil (9:16)").size(11.5).color(accent))
                .fill(Color32::from_rgb(32, 32, 32))
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(190.0, 30.0)))
                .on_hover_text("Cambiar a interfaz táctil de smartphone (390x820)")
                .clicked()
            {
                self.is_mobile_mode = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(390.0, 820.0)));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title("📱 Melodyfy Mobile".into()));
            }

            ui.add_space(6.0);

            let theme_label = if self.use_spotify_green { "🟢 Verde Spotify" } else { "🟠 Ámbar Editorial" };
            if ui.small_button(theme_label).on_hover_text("Cambiar color de acento").clicked() {
                self.use_spotify_green = !self.use_spotify_green;
            }

            ui.add_space(6.0);
            ui.label(egui::RichText::new("🚀 0% Publicidad • 100% Libre").size(10.0).color(Color32::from_rgb(110, 110, 110)));
        });
    }

    fn render_main_content(&mut self, ui: &mut egui::Ui, accent: Color32) {
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
            ViewTab::History => self.render_history_view(ui, accent),
            ViewTab::Queue => self.render_queue_view(ui, accent),
        }
    }

    fn render_home_view(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.heading(egui::RichText::new("Buenas tardes").size(28.0).strong().color(Color32::WHITE));
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

            let clicked_actions = response.hover_pos().map_or(false, |p| p.x > rect.max.x - 120.0);
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

                // ALBUM COVER IMAGE THUMBNAIL (Fixed 44x44 container to prevent visual jumping)
                let (thumb_rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), egui::Sense::hover());
                ui.painter().rect_filled(thumb_rect, Rounding::same(4.0), Color32::from_rgb(34, 34, 34));
                ui.painter().text(thumb_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(18.0), Color32::from_rgb(90, 90, 90));

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

                    ui.add_space(8.0);

                    // Favorite button
                    let is_fav = self.library.is_favorite(&track.id);
                    let heart = if is_fav { "💚" } else { "🤍" };
                    if ui.small_button(heart).on_hover_text("Marcar como favorita").clicked() {
                        let added = self.library.toggle_favorite(track.clone());
                        if added {
                            self.show_toast("Guardada en Tus Me Gusta");
                        }
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
                    if !t.thumbnail_url.is_empty() {
                        ui.put(
                            art_rect,
                            egui::Image::new(&t.thumbnail_url)
                                .fit_to_exact_size(Vec2::new(56.0, 56.0))
                                .rounding(Rounding::same(6.0)),
                        );
                    } else {
                        ui.painter().text(art_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(26.0), Color32::WHITE);
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
                } else {
                    ui.painter().text(art_rect.center(), egui::Align2::CENTER_CENTER, "🎧", FontId::proportional(26.0), Color32::GRAY);
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Melodyfy").strong().size(14.0).color(Color32::WHITE));
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

                    // Large Circular Play/Pause Button
                    let is_playing = status.state == 3;
                    let play_symbol = if is_playing { "⏸" } else { "▶" };
                    let play_btn = egui::Button::new(egui::RichText::new(play_symbol).size(18.0).strong().color(Color32::BLACK))
                        .fill(accent)
                        .rounding(Rounding::same(20.0))
                        .min_size(Vec2::new(38.0, 38.0));

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
                ui.add_space(20.0);

                // Mini-Player / Background Mode HUD (440x120 floating)
                let mini_icon = if self.is_mini_mode { "🪟" } else { "🕹️" };
                if ui.add(egui::Button::new(egui::RichText::new(mini_icon).size(15.0)).frame(false))
                    .on_hover_text("Modo Mini / 2º Plano Flotante (440x120)").clicked()
                {
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

                ui.add_space(10.0);

                // Queue button
                let q_color = if self.current_tab == ViewTab::Queue { accent } else { Color32::from_rgb(170, 170, 170) };
                if ui.add(egui::Button::new(egui::RichText::new("📑").size(15.0).color(q_color)).frame(false))
                    .on_hover_text("Ver Cola de Reproducción").clicked()
                {
                    self.current_tab = ViewTab::Queue;
                }

                ui.add_space(10.0);

                // Custom Spotify Volume Bar (90px)
                if let Some(new_vol) = Self::custom_spotify_volume_bar(ui, self.volume, 90.0, accent) {
                    self.volume = new_vol;
                    self.audio.set_volume(new_vol);
                    self.library.last_volume = Some(new_vol);
                    self.library.save();
                }

                ui.add_space(6.0);

                // Volume icon toggle
                let vol_icon = if self.volume == 0 { "🔇" } else if self.volume < 50 { "🔉" } else { "🔊" };
                if ui.add(egui::Button::new(egui::RichText::new(vol_icon).size(14.0)).frame(false))
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
                    ui.painter().text(art_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(16.0), Color32::WHITE);
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
                    ui.label(egui::RichText::new("Melodyfy").strong().size(13.0).color(Color32::WHITE));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(6.0);
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
                    ui.label(egui::RichText::new("🟢 Melodyfy").strong().size(16.0).color(Color32::WHITE));

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
                    ViewTab::Favorites | ViewTab::History | ViewTab::Queue => self.render_mobile_library_view(ui, accent),
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
                if !track.thumbnail_url.is_empty() {
                    ui.put(
                        thumb_rect,
                        egui::Image::new(&track.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(40.0, 40.0))
                            .rounding(Rounding::same(5.0)),
                    );
                } else {
                    ui.painter().text(thumb_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(18.0), Color32::WHITE);
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

            if let Some(ref t) = curr_track {
                if !t.thumbnail_url.is_empty() {
                    ui.put(
                        art_rect,
                        egui::Image::new(&t.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(art_w, art_w))
                            .rounding(Rounding::same(12.0)),
                    );
                } else {
                    ui.painter().text(art_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(64.0), Color32::WHITE);
                }
            } else {
                ui.painter().text(art_rect.center(), egui::Align2::CENTER_CENTER, "🎧", FontId::proportional(64.0), Color32::GRAY);
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
                (1, "🕒 Historial".to_string()),
                (2, format!("📑 Cola ({})", self.queue.len())),
            ];

            for (idx, label) in subtabs {
                let is_active = self.mobile_library_subtab == idx;
                let bg_col = if is_active { accent } else { Color32::from_rgb(32, 32, 32) };
                let txt_col = if is_active { Color32::BLACK } else { Color32::WHITE };

                let pill = egui::Button::new(egui::RichText::new(label).size(11.0).strong().color(txt_col))
                    .fill(bg_col)
                    .rounding(Rounding::same(12.0));

                if ui.add(pill).clicked() {
                    self.mobile_library_subtab = idx;
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
                if !track.thumbnail_url.is_empty() {
                    ui.put(
                        thumb_rect,
                        egui::Image::new(&track.thumbnail_url)
                            .fit_to_exact_size(Vec2::new(42.0, 42.0))
                            .rounding(Rounding::same(4.0)),
                    );
                } else {
                    ui.painter().text(thumb_rect.center(), egui::Align2::CENTER_CENTER, "🎵", FontId::proportional(16.0), Color32::WHITE);
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
