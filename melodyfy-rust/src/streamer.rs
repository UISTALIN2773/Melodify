use crate::models::Track;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct Streamer {
    yt_dlp_path: PathBuf,
    stream_cache: Arc<Mutex<HashMap<String, (String, Instant)>>>,
}

impl Streamer {
    pub fn new() -> Self {
        let candidates = [
            PathBuf::from("yt-dlp.exe"),
            PathBuf::from("../yt-dlp.exe"),
            PathBuf::from("C:\\Users\\HP\\Desktop\\For GIt\\lite-music-player\\yt-dlp.exe"),
        ];

        let yt_dlp_path = candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| PathBuf::from("yt-dlp.exe"));

        let _ = std::fs::create_dir_all("cache/thumbs");

        Self {
            yt_dlp_path,
            stream_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn search_async(&self, query: String, tx: Sender<Vec<Track>>) {
        let yt_path = self.yt_dlp_path.clone();
        let cache_clone = self.stream_cache.clone();

        std::thread::spawn(move || {
            let mut tracks = Vec::new();
            let is_direct_url = query.starts_with("http://") || query.starts_with("https://");
            let is_playlist = is_direct_url && (query.contains("list=") || query.contains("playlist"));

            let mut args = vec![
                "--dump-json".to_string(),
                "--flat-playlist".to_string(),
                "--no-warnings".to_string(),
                "--skip-download".to_string(),
                "--extractor-args".to_string(),
                "youtube:player_client=android".to_string(),
            ];

            if is_direct_url {
                if !is_playlist {
                    args.push("--no-playlist".to_string());
                }
                args.push(query);
            } else {
                args.push(format!("ytsearch25:{}", query));
            }

            let mut cmd = Command::new(&yt_path);
            cmd.args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW);

            if let Ok(mut child) = cmd.spawn() {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line_result in reader.lines() {
                        if let Ok(line) = line_result {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(id) = json["id"].as_str() {
                                    let title = json["title"].as_str().unwrap_or("Unknown Track").to_string();
                                    let artist = json["uploader"]
                                        .as_str()
                                        .or_else(|| json["channel"].as_str())
                                        .unwrap_or("Unknown Artist")
                                        .to_string();

                                    let mut webpage_url = json["webpage_url"]
                                        .as_str()
                                        .or_else(|| json["url"].as_str())
                                        .map(|s| s.to_string())
                                        .unwrap_or_default();
                                    if webpage_url.is_empty() {
                                        webpage_url = format!("https://www.youtube.com/watch?v={}", id);
                                    }

                                    // Fast, guaranteed high-definition thumbnail
                                    let mut thumbnail_url = json["thumbnail"].as_str().map(|s| s.to_string()).unwrap_or_default();
                                    if thumbnail_url.is_empty() || thumbnail_url.contains("storyboard") {
                                        thumbnail_url = format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", id);
                                    }

                                    let duration_seconds = json["duration"].as_u64().unwrap_or(0);
                                    let duration = if let Some(ds) = json["duration_string"].as_str() {
                                        ds.to_string()
                                    } else {
                                        format!("{}:{:02}", duration_seconds / 60, duration_seconds % 60)
                                    };

                                    let track = Track {
                                        id: id.to_string(),
                                        title,
                                        artist,
                                        duration,
                                        duration_seconds,
                                        thumbnail_url,
                                        webpage_url,
                                    };

                                    let to_prefetch = track.clone();
                                    tracks.push(track);

                                    // Auto-prefetch first 3 tracks in background for instant 0ms click
                                    if tracks.len() <= 3 {
                                        Self::prefetch_internal(yt_path.clone(), cache_clone.clone(), to_prefetch);
                                    }

                                    // Incremental updates for instant response (< 0.5s)
                                    if tracks.len() == 1 || tracks.len() % 3 == 0 {
                                        let _ = tx.send(tracks.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                let _ = child.wait();
            }

            let _ = tx.send(tracks);
        });
    }

    pub fn prefetch_stream_url(&self, track: &Track) {
        Self::prefetch_internal(self.yt_dlp_path.clone(), self.stream_cache.clone(), track.clone());
    }

    fn prefetch_internal(yt_path: PathBuf, streamer_clone: Arc<Mutex<HashMap<String, (String, Instant)>>>, track_clone: Track) {
        std::thread::spawn(move || {
            // If already in cache, do nothing
            if let Ok(cache) = streamer_clone.lock() {
                if let Some((_, ts)) = cache.get(&track_clone.id) {
                    if ts.elapsed() < Duration::from_secs(4 * 3600) {
                        return;
                    }
                }
            }

            let url_to_resolve = if track_clone.webpage_url.is_empty() {
                format!("https://www.youtube.com/watch?v={}", track_clone.id)
            } else {
                track_clone.webpage_url.clone()
            };

            let mut cmd = Command::new(&yt_path);
            cmd.args(&[
                "-g",
                "-f",
                "bestaudio[ext=m4a]/bestaudio/best",
                "--no-warnings",
                "--extractor-args",
                "youtube:player_client=android",
                &url_to_resolve,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let stream_url = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    if !stream_url.is_empty() {
                        if let Ok(mut cache) = streamer_clone.lock() {
                            cache.insert(track_clone.id, (stream_url, Instant::now()));
                        }
                    }
                }
            }
        });
    }

    pub fn resolve_stream_url(&self, track: &Track) -> Option<String> {
        // Check cache (TTL 4 hours)
        {
            let cache = self.stream_cache.lock().ok()?;
            if let Some((url, timestamp)) = cache.get(&track.id) {
                if timestamp.elapsed() < Duration::from_secs(4 * 3600) {
                    return Some(url.clone());
                }
            }
        }

        let url_to_resolve = if track.webpage_url.is_empty() {
            format!("https://www.youtube.com/watch?v={}", track.id)
        } else {
            track.webpage_url.clone()
        };

        let mut cmd = Command::new(&self.yt_dlp_path);
        cmd.args(&[
            "-g",
            "-f",
            "bestaudio[ext=m4a]/bestaudio/best",
            "--no-warnings",
            "--extractor-args",
            "youtube:player_client=android",
            &url_to_resolve,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().ok()?;
        if output.status.success() {
            let stream_url = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()?
                .trim()
                .to_string();

            if !stream_url.is_empty() {
                if let Ok(mut cache) = self.stream_cache.lock() {
                    cache.insert(track.id.clone(), (stream_url.clone(), Instant::now()));
                }
                return Some(stream_url);
            }
        }

        None
    }
}
