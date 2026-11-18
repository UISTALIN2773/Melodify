use crate::models::Track;
use serde_json::json;

pub struct InnerTube;

impl InnerTube {
    /// Búsqueda nativa en Rust puro a través de la API InnerTube de YouTube.
    /// Responde en ~300ms sin crear procesos hijos de Python ni depender de binarios externos.
    pub fn search(query: &str) -> Option<Vec<Track>> {
        let payload = json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": "2.20240313.01.00",
                    "hl": "es",
                    "gl": "US"
                }
            },
            "query": query
        });

        let resp = ureq::post("https://www.youtube.com/youtubei/v1/search")
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send_json(&payload)
            .ok()?;

        let json_val: serde_json::Value = resp.into_body().read_json().ok()?;

        // Navegar árbol JSON de resultados de búsqueda
        let contents = json_val
            .get("contents")?
            .get("twoColumnSearchResultsRenderer")?
            .get("primaryContents")?
            .get("sectionListRenderer")?
            .get("contents")?
            .as_array()?;

        let mut tracks = Vec::new();

        for section in contents {
            if let Some(items) = section.get("itemSectionRenderer").and_then(|s| s.get("contents")).and_then(|c| c.as_array()) {
                for item in items {
                    if let Some(vr) = item.get("videoRenderer") {
                        if let Some(track) = Self::parse_video_renderer(vr) {
                            tracks.push(track);
                        }
                    }
                }
            }
        }

        if tracks.is_empty() {
            None
        } else {
            Some(tracks)
        }
    }

    fn parse_video_renderer(vr: &serde_json::Value) -> Option<Track> {
        let id = vr.get("videoId")?.as_str()?.to_string();

        let title = vr.get("title")
            .and_then(|t| t.get("runs"))
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|item| item.get("text"))
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown Title")
            .to_string();

        let artist = vr.get("ownerText")
            .or_else(|| vr.get("shortBylineText"))
            .and_then(|o| o.get("runs"))
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|item| item.get("text"))
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown Artist")
            .to_string();

        let duration = vr.get("lengthText")
            .and_then(|l| l.get("simpleText"))
            .and_then(|s| s.as_str())
            .unwrap_or("0:00")
            .to_string();

        let duration_seconds = Self::parse_duration_string(&duration);
        let thumbnail_url = format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", id);
        let webpage_url = format!("https://www.youtube.com/watch?v={}", id);

        Some(Track {
            id,
            title,
            artist,
            duration,
            duration_seconds,
            thumbnail_url,
            webpage_url,
        })
    }

    fn parse_duration_string(s: &str) -> u64 {
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            2 => {
                let m: u64 = parts[0].parse().unwrap_or(0);
                let sec: u64 = parts[1].parse().unwrap_or(0);
                m * 60 + sec
            }
            3 => {
                let h: u64 = parts[0].parse().unwrap_or(0);
                let m: u64 = parts[1].parse().unwrap_or(0);
                let sec: u64 = parts[2].parse().unwrap_or(0);
                h * 3600 + m * 60 + sec
            }
            _ => 0,
        }
    }

    /// Resuelve la URL directa HTTPS de audio para un ID de video de YouTube
    /// utilizando la API nativa de InnerTube en Rust puro sin invocar subprocesos externos.
    pub fn resolve_stream_url(video_id: &str) -> Option<String> {
        // Intentar primero con cliente Android
        if let Some(url) = Self::try_client(video_id, "ANDROID", "19.09.37", Some(30)) {
            return Some(url);
        }

        // Fallback rápido a cliente iOS
        if let Some(url) = Self::try_client(video_id, "IOS", "19.09.3", None) {
            return Some(url);
        }

        None
    }

    fn try_client(video_id: &str, client_name: &str, client_version: &str, sdk_ver: Option<u32>) -> Option<String> {
        let mut client_obj = json!({
            "clientName": client_name,
            "clientVersion": client_version,
            "hl": "en",
            "gl": "US"
        });

        if let Some(sdk) = sdk_ver {
            if let Some(obj) = client_obj.as_object_mut() {
                obj.insert("androidSdkVersion".to_string(), json!(sdk));
            }
        }

        let payload = json!({
            "context": {
                "client": client_obj
            },
            "videoId": video_id
        });

        let resp = ureq::post("https://www.youtube.com/youtubei/v1/player")
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send_json(&payload)
            .ok()?;

        let json_val: serde_json::Value = resp.into_body().read_json().ok()?;

        // Explorar adaptiveFormats (separados por pista de audio/video)
        let streaming_data = json_val.get("streamingData")?;
        let formats = streaming_data.get("adaptiveFormats")?.as_array()?;

        let mut best_audio_url: Option<String> = None;
        let mut best_bitrate = 0u64;

        for fmt in formats {
            let mime = fmt.get("mimeType").and_then(|m| m.as_str()).unwrap_or("");
            if mime.starts_with("audio/") {
                let bitrate = fmt.get("bitrate").and_then(|b| b.as_u64()).unwrap_or(0);
                if let Some(url) = fmt.get("url").and_then(|u| u.as_str()) {
                    if bitrate >= best_bitrate && !url.is_empty() {
                        best_bitrate = bitrate;
                        best_audio_url = Some(url.to_string());
                    }
                }
            }
        }

        best_audio_url
    }
}
