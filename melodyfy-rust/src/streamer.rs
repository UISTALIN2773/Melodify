use crate::models::Track;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const CACHE_FILE: &str = "cache/stream_cache.json";
const CACHE_TTL_SECS: u64 = 18000; // 5 hours (googlevideo streams remain valid for 6 hours)

pub struct Streamer {
    yt_dlp_path: PathBuf,
    stream_cache: Arc<Mutex<HashMap<String, (String, Instant)>>>,
    in_flight: Arc<Mutex<HashSet<String>>>,
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

        let _ = fs::create_dir_all("cache/thumbs");
        let _ = fs::create_dir_all("cache/ytdlp");

        // Load persistent disk cache from previous sessions
        let initial_cache = Self::load_disk_cache();

        Self {
            yt_dlp_path,
            stream_cache: Arc::new(Mutex::new(initial_cache)),
            in_flight: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn load_disk_cache() -> HashMap<String, (String, Instant)> {
        let mut map = HashMap::new();
        let path = Path::new(CACHE_FILE);
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(raw_map) = serde_json::from_str::<HashMap<String, (String, u64)>>(&data) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                for (id, (url, created_at)) in raw_map {
                    if now >= created_at && (now - created_at) < CACHE_TTL_SECS {
                        let age = now - created_at;
                        let inst = Instant::now()
                            .checked_sub(Duration::from_secs(age))
                            .unwrap_or_else(Instant::now);
                        map.insert(id, (url, inst));
                    }
                }
            }
        }
        map
    }

    fn persist_to_disk(cache_arc: &Arc<Mutex<HashMap<String, (String, Instant)>>>) {
        if let Ok(cache) = cache_arc.lock() {
            let now_sys = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let now_inst = Instant::now();
            let mut raw_map = HashMap::new();
            for (id, (url, inst)) in cache.iter() {
                let elapsed = now_inst.saturating_duration_since(*inst).as_secs();
                if elapsed < CACHE_TTL_SECS {
                    let created_at = now_sys.saturating_sub(elapsed);
                    raw_map.insert(id.clone(), (url.clone(), created_at));
                }
            }
            let _ = fs::create_dir_all("cache");
            if let Ok(json) = serde_json::to_string(&raw_map) {
                let _ = fs::write(CACHE_FILE, json);
            }
        }
    }

    pub fn search_async(&self, query: String, tx: Sender<Vec<Track>>) {
        let yt_path = self.yt_dlp_path.clone();
        let cache_clone = self.stream_cache.clone();
        let in_flight_clone = self.in_flight.clone();

        std::thread::spawn(move || {
            let is_direct_url = query.starts_with("http://") || query.starts_with("https://");
            let is_playlist = is_direct_url && (query.contains("list=") || query.contains("playlist"));

            // Priority 1: Instant Native InnerTube Search in Pure Rust (~250ms, 0 child processes)
            if !is_direct_url {
                if let Some(native_tracks) = crate::innertube::InnerTube::search(&query) {
                    if !native_tracks.is_empty() {
                        if let Some(first) = native_tracks.first().cloned() {
                            Self::prefetch_internal(yt_path.clone(), cache_clone.clone(), in_flight_clone.clone(), first);
                        }
                        let _ = tx.send(native_tracks);
                        return;
                    }
                }
            }

            let mut tracks = Vec::new();

            let mut args = vec![
                "--dump-json".to_string(),
                "--flat-playlist".to_string(),
                "--no-warnings".to_string(),
                "--skip-download".to_string(),
                "--no-check-certificates".to_string(),
                "--force-ipv4".to_string(),
                "--socket-timeout".to_string(),
                "5".to_string(),
                "--cache-dir".to_string(),
                "cache/ytdlp".to_string(),
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

                                    // Auto-prefetch top result immediately for instant 0ms first click
                                    if tracks.len() == 1 {
                                        Self::prefetch_internal(yt_path.clone(), cache_clone.clone(), in_flight_clone.clone(), to_prefetch);
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
        Self::prefetch_internal(
            self.yt_dlp_path.clone(),
            self.stream_cache.clone(),
            self.in_flight.clone(),
            track.clone(),
        );
    }

    fn prefetch_internal(
        yt_path: PathBuf,
        cache_clone: Arc<Mutex<HashMap<String, (String, Instant)>>>,
        in_flight_clone: Arc<Mutex<HashSet<String>>>,
        track_clone: Track,
    ) {
        // Fast path: if already in cache with valid TTL, return immediately
        if let Ok(cache) = cache_clone.lock() {
            if let Some((_, ts)) = cache.get(&track_clone.id) {
                if ts.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
                    return;
                }
            }
        }

        // Avoid duplicate concurrent workers for the same track
        if let Ok(mut in_flight) = in_flight_clone.lock() {
            if in_flight.contains(&track_clone.id) {
                return;
            }
            in_flight.insert(track_clone.id.clone());
        }

        std::thread::spawn(move || {
            // Priority 1: Fast Pure-Rust InnerTube resolution (~120ms, no child process)
            if let Some(stream_url) = crate::innertube::InnerTube::resolve_stream_url(&track_clone.id) {
                if !stream_url.is_empty() {
                    if let Ok(mut cache) = cache_clone.lock() {
                        cache.insert(track_clone.id.clone(), (stream_url, Instant::now()));
                    }
                    Self::persist_to_disk(&cache_clone);
                    if let Ok(mut in_flight) = in_flight_clone.lock() {
                        in_flight.remove(&track_clone.id);
                    }
                    return;
                }
            }

            // Priority 2: Fallback to yt-dlp binary if available
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
                "--no-playlist",
                "--no-check-certificates",
                "--force-ipv4",
                "--socket-timeout",
                "5",
                "--cache-dir",
                "cache/ytdlp",
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
                        if let Ok(mut cache) = cache_clone.lock() {
                            cache.insert(track_clone.id.clone(), (stream_url, Instant::now()));
                        }
                        Self::persist_to_disk(&cache_clone);
                    }
                }
            }

            if let Ok(mut in_flight) = in_flight_clone.lock() {
                in_flight.remove(&track_clone.id);
            }
        });
    }

    pub fn resolve_stream_url(&self, track: &Track) -> Option<String> {
        // Offline local track fast path
        if track.webpage_url.starts_with("local:") {
            return Some(track.webpage_url.clone());
        }

        // Instant check in persistent cache (0.0001 ms)
        {
            let cache = self.stream_cache.lock().ok()?;
            if let Some((url, timestamp)) = cache.get(&track.id) {
                if timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
                    return Some(url.clone());
                }
            }
        }

        // Fast pure-Rust InnerTube resolution (~120 ms, no subprocess)
        if let Some(stream_url) = crate::innertube::InnerTube::resolve_stream_url(&track.id) {
            if !stream_url.is_empty() {
                if let Ok(mut cache) = self.stream_cache.lock() {
                    cache.insert(track.id.clone(), (stream_url.clone(), Instant::now()));
                }
                Self::persist_to_disk(&self.stream_cache);
                return Some(stream_url);
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
            "--no-playlist",
            "--no-check-certificates",
            "--force-ipv4",
            "--socket-timeout",
            "5",
            "--cache-dir",
            "cache/ytdlp",
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
                Self::persist_to_disk(&self.stream_cache);
                return Some(stream_url);
            }
        }

        None
    }

    /// Descarga directa de la pista en formato audio M4A/AAC a la carpeta local 'downloads/'
    /// sin DRM propietario para reproducción libre y offline permanente.
    pub fn download_track_async(&self, track: Track, tx: Sender<(String, bool, Option<(Track, String)>)>) {
        let yt_path = self.yt_dlp_path.clone();
        let cache_clone = self.stream_cache.clone();

        std::thread::spawn(move || {
            let _ = fs::create_dir_all("downloads");
            let clean_title: String = track.title.chars()
                .map(|c| match c {
                    '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                    _ => c,
                })
                .collect();
            let clean_artist: String = track.artist.chars()
                .map(|c| match c {
                    '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                    _ => c,
                })
                .collect();
            let filename = format!("downloads/{} - {}.m4a", clean_artist, clean_title);

            // 1. Obtener URL de stream (desde caché, InnerTube o yt-dlp)
            let mut stream_url: Option<String> = None;
            if let Ok(cache) = cache_clone.lock() {
                if let Some((url, ts)) = cache.get(&track.id) {
                    if ts.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
                        stream_url = Some(url.clone());
                    }
                }
            }

            if stream_url.is_none() {
                stream_url = crate::innertube::InnerTube::resolve_stream_url(&track.id);
            }

            if stream_url.is_none() {
                let mut cmd = Command::new(&yt_path);
                cmd.args(&[
                    "-g",
                    "-f", "bestaudio[ext=m4a]/bestaudio/best",
                    "--no-warnings",
                    "--no-playlist",
                    &format!("https://www.youtube.com/watch?v={}", track.id)
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW);

                if let Ok(output) = cmd.output() {
                    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !out.is_empty() {
                        stream_url = Some(out);
                    }
                }
            }

            if let Some(url) = stream_url {
                if let Ok(resp) = ureq::get(&url).call() {
                    if let Ok(mut file) = fs::File::create(&filename) {
                        let mut reader = resp.into_body().into_reader();
                        if std::io::copy(&mut reader, &mut file).is_ok() {
                            let msg = format!("✅ Descargado: {}", track.title);
                            let _ = tx.send((msg, true, Some((track, filename))));
                            return;
                        }
                    }
                }
            }

            let _ = tx.send((format!("❌ Error al descargar: {}", track.title), false, None));
        });
    }
}
