use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: String,
    pub duration_seconds: u64,
    pub thumbnail_url: String,
    pub webpage_url: String,
}

impl Track {
    #[allow(dead_code)]
    pub fn new(id: impl Into<String>, title: impl Into<String>, artist: impl Into<String>) -> Self {
        let id_str = id.into();
        let webpage_url = format!("https://www.youtube.com/watch?v={}", id_str);
        Self {
            id: id_str,
            title: title.into(),
            artist: artist.into(),
            duration: "0:00".to_string(),
            duration_seconds: 0,
            thumbnail_url: String::new(),
            webpage_url,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickTag {
    pub label: String,
    pub query: String,
}

impl QuickTag {
    pub fn default_tags() -> Vec<Self> {
        vec![
            QuickTag { label: "🔥 Top 50".to_string(), query: "Top Éxitos Mundiales 2026".to_string() },
            QuickTag { label: "🎧 Lo-Fi Chill".to_string(), query: "Lo-Fi Gaming Chill Beats".to_string() },
            QuickTag { label: "⚡ Synthwave".to_string(), query: "Synthwave Cyberpunk 80s".to_string() },
            QuickTag { label: "🎸 Rock Clásico".to_string(), query: "Rock Clásico 80s 90s".to_string() },
            QuickTag { label: "🎤 Bad Bunny".to_string(), query: "Bad Bunny".to_string() },
            QuickTag { label: "🤖 Daft Punk".to_string(), query: "Daft Punk".to_string() },
            QuickTag { label: "🌙 The Weeknd".to_string(), query: "The Weeknd".to_string() },
            QuickTag { label: "🚗 Phonk Drift".to_string(), query: "Phonk Drift".to_string() },
            QuickTag { label: "🌸 Anime".to_string(), query: "Anime Openings HD".to_string() },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLibrary {
    pub favorites: Vec<Track>,
    pub history: Vec<Track>,
    pub playlists: HashMap<String, Vec<Track>>,
    pub last_volume: Option<i32>,
    pub quick_tags: Option<Vec<QuickTag>>,
}

impl AppLibrary {
    const FILE_NAME: &'static str = "library.json";

    pub fn load() -> Self {
        if Path::new(Self::FILE_NAME).exists() {
            if let Ok(data) = fs::read_to_string(Self::FILE_NAME) {
                if let Ok(lib) = serde_json::from_str::<Self>(&data) {
                    return lib;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(Self::FILE_NAME, json);
        }
    }

    pub fn get_quick_tags(&self) -> Vec<QuickTag> {
        self.quick_tags.clone().unwrap_or_else(QuickTag::default_tags)
    }

    pub fn add_quick_tag(&mut self, label: String, query: String) {
        let mut tags = self.get_quick_tags();
        tags.retain(|t| t.label != label);
        tags.push(QuickTag { label, query });
        self.quick_tags = Some(tags);
        self.save();
    }

    pub fn remove_quick_tag(&mut self, index: usize) {
        let mut tags = self.get_quick_tags();
        if index < tags.len() {
            tags.remove(index);
            self.quick_tags = Some(tags);
            self.save();
        }
    }

    pub fn is_favorite(&self, track_id: &str) -> bool {
        self.favorites.iter().any(|t| t.id == track_id)
    }

    pub fn toggle_favorite(&mut self, track: Track) -> bool {
        if let Some(pos) = self.favorites.iter().position(|t| t.id == track.id) {
            self.favorites.remove(pos);
            self.save();
            false
        } else {
            self.favorites.insert(0, track);
            self.save();
            true
        }
    }

    pub fn record_history(&mut self, track: Track) {
        self.history.retain(|t| t.id != track.id);
        self.history.insert(0, track);
        if self.history.len() > 100 {
            self.history.truncate(100);
        }
        self.save();
    }
}
