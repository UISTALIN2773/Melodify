use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricLine {
    pub time_seconds: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsData {
    pub track_id: String,
    pub is_synced: bool,
    pub lines: Vec<LyricLine>,
    pub plain_lyrics: Option<String>,
}

pub struct LyricsFetcher;

impl LyricsFetcher {
    /// Consulta la API abierta de LRCLIB (https://lrclib.net/) para obtener letras sincronizadas (LRC)
    pub fn fetch_lyrics(title: &str, artist: &str, duration_secs: u64, track_id: &str) -> Option<LyricsData> {
        // Limpiar título de sufijos comunes de YouTube (Official Video, HD, 4K, Audio, etc.)
        let clean_title = Self::clean_title(title);
        let clean_artist = Self::clean_artist(artist);

        // 1. Intento exacto con título, artista y duración
        if let Some(data) = Self::query_lrclib_get(&clean_title, &clean_artist, duration_secs, track_id) {
            return Some(data);
        }

        // 2. Intento de búsqueda abierta en lrclib
        if let Some(data) = Self::query_lrclib_search(&clean_title, &clean_artist, track_id) {
            return Some(data);
        }

        None
    }

    fn clean_title(title: &str) -> String {
        let mut t = title.to_string();
        let patterns = [
            "(Official Video)", "(Official Audio)", "(Official Music Video)",
            "[Official Video]", "[Official Audio]", "[Official Music Video]",
            "(Audio)", "[Audio]", "(Video)", "[Video]",
            "(Lyric Video)", "[Lyric Video]", "(Lyrics)", "[Lyrics]",
            "(HD)", "[HD]", "(4K)", "[4K]", "MV", "(MV)", "[MV]",
            "Video Oficial", "Audio Oficial", "Letra",
        ];
        for p in &patterns {
            t = t.replace(p, "");
        }
        // Si tiene formato "Artista - Titulo", extraer el título
        if let Some((_, right)) = t.split_once(" - ") {
            t = right.to_string();
        }
        t.trim().to_string()
    }

    fn clean_artist(artist: &str) -> String {
        let mut a = artist.to_string();
        let suffixes = [" - Topic", "VEVO", "Official"];
        for s in &suffixes {
            a = a.replace(s, "");
        }
        a.trim().to_string()
    }

    fn query_lrclib_get(title: &str, artist: &str, duration_secs: u64, track_id: &str) -> Option<LyricsData> {
        let mut url = format!(
            "https://lrclib.net/api/get?track_name={}&artist_name={}",
            urlencoding(title),
            urlencoding(artist)
        );
        if duration_secs > 0 {
            url.push_str(&format!("&duration={}", duration_secs));
        }

        let resp = ureq::get(&url)
            .header("User-Agent", "Melodyfy-Rust-Client/1.0 (https://github.com)")
            .call()
            .ok()?;

        let json: serde_json::Value = resp.into_body().read_json().ok()?;
        Self::parse_lrclib_response(&json, track_id)
    }

    fn query_lrclib_search(title: &str, artist: &str, track_id: &str) -> Option<LyricsData> {
        let query = format!("{} {}", title, artist);
        let url = format!(
            "https://lrclib.net/api/search?q={}",
            urlencoding(&query)
        );

        let resp = ureq::get(&url)
            .header("User-Agent", "Melodyfy-Rust-Client/1.0 (https://github.com)")
            .call()
            .ok()?;

        let json_arr: serde_json::Value = resp.into_body().read_json().ok()?;
        if let Some(arr) = json_arr.as_array() {
            for item in arr {
                if let Some(lyrics) = Self::parse_lrclib_response(item, track_id) {
                    return Some(lyrics);
                }
            }
        }

        None
    }

    fn parse_lrclib_response(json: &serde_json::Value, track_id: &str) -> Option<LyricsData> {
        // Verificar si tiene syncedLyrics
        if let Some(synced) = json.get("syncedLyrics").and_then(|s| s.as_str()) {
            if !synced.trim().is_empty() {
                let lines = Self::parse_lrc(synced);
                if !lines.is_empty() {
                    return Some(LyricsData {
                        track_id: track_id.to_string(),
                        is_synced: true,
                        lines,
                        plain_lyrics: json.get("plainLyrics").and_then(|p| p.as_str()).map(|s| s.to_string()),
                    });
                }
            }
        }

        // Fallback a plainLyrics si no hay sincronizadas
        if let Some(plain) = json.get("plainLyrics").and_then(|p| p.as_str()) {
            if !plain.trim().is_empty() {
                let lines: Vec<LyricLine> = plain
                    .lines()
                    .map(|l| LyricLine {
                        time_seconds: 0.0,
                        text: l.trim().to_string(),
                    })
                    .collect();

                return Some(LyricsData {
                    track_id: track_id.to_string(),
                    is_synced: false,
                    lines,
                    plain_lyrics: Some(plain.to_string()),
                });
            }
        }

        None
    }

    /// Parsea texto en formato standard LRC: `[mm:ss.xx] Texto de la letra`
    pub fn parse_lrc(lrc_text: &str) -> Vec<LyricLine> {
        let mut lines = Vec::new();
        for line in lrc_text.lines() {
            let line = line.trim();
            if !line.starts_with('[') {
                continue;
            }

            if let Some(close_bracket) = line.find(']') {
                let time_str = &line[1..close_bracket];
                let text = line[close_bracket + 1..].trim().to_string();

                if let Some(seconds) = Self::parse_timestamp(time_str) {
                    lines.push(LyricLine {
                        time_seconds: seconds,
                        text,
                    });
                }
            }
        }
        lines.sort_by(|a, b| a.time_seconds.partial_cmp(&b.time_seconds).unwrap_or(std::cmp::Ordering::Equal));
        lines
    }

    fn parse_timestamp(time_str: &str) -> Option<f64> {
        // Formatos habituales: "01:23.45" o "01:23" o "00:01:23.45"
        let parts: Vec<&str> = time_str.split(':').collect();
        match parts.len() {
            2 => {
                let mins: f64 = parts[0].parse().ok()?;
                let secs: f64 = parts[1].parse().ok()?;
                Some(mins * 60.0 + secs)
            }
            3 => {
                let hours: f64 = parts[0].parse().ok()?;
                let mins: f64 = parts[1].parse().ok()?;
                let secs: f64 = parts[2].parse().ok()?;
                Some(hours * 3600.0 + mins * 60.0 + secs)
            }
            _ => None,
        }
    }
}

fn urlencoding(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
