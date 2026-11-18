use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::core::HSTRING;
use windows::Foundation::{TimeSpan, Uri};
use windows::Media::Core::MediaSource;
use windows::Media::Playback::{MediaPlayer, MediaPlaybackState};

pub enum AudioCommand {
    PlayUrl(String),
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(i32),
    SetPlaybackRate(f64),
}

#[derive(Debug, Clone, Default)]
pub struct AudioStatus {
    pub state: i32,
    pub position: f64,
    pub duration: f64,
    pub volume: i32,
    pub playback_rate: f64,
}

pub struct AudioEngine {
    cmd_tx: Sender<AudioCommand>,
    status: Arc<Mutex<AudioStatus>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = channel::<AudioCommand>();
        let status = Arc::new(Mutex::new(AudioStatus {
            state: 1,
            position: 0.0,
            duration: 0.0,
            volume: 100,
            playback_rate: 1.0,
        }));
        let status_clone = Arc::clone(&status);

        thread::spawn(move || {
            let player = match MediaPlayer::new() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[CoreKernel Audio] Fallo al inicializar Windows MediaPlayer: {:?}", e);
                    return;
                }
            };

            let _ = player.SetVolume(1.0);
            let mut desired_rate: f64 = 1.0;

            loop {
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        AudioCommand::PlayUrl(raw_url) => {
                            let final_uri_str = if let Some(local_rel) = raw_url.strip_prefix("local:") {
                                if let Ok(abs) = std::fs::canonicalize(local_rel) {
                                    let s = abs.to_string_lossy().replace('\\', "/");
                                    let clean = s.strip_prefix("//?/").unwrap_or(&s);
                                    format!("file:///{}", clean)
                                } else {
                                    raw_url
                                }
                            } else if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") && !raw_url.starts_with("file://") {
                                if let Ok(abs) = std::fs::canonicalize(&raw_url) {
                                    let s = abs.to_string_lossy().replace('\\', "/");
                                    let clean = s.strip_prefix("//?/").unwrap_or(&s);
                                    format!("file:///{}", clean)
                                } else {
                                    raw_url
                                }
                            } else {
                                raw_url
                            };

                            if let Ok(uri) = Uri::CreateUri(&HSTRING::from(final_uri_str)) {
                                if let Ok(source) = MediaSource::CreateFromUri(&uri) {
                                    let _ = player.SetSource(&source);
                                    if let Ok(session) = player.PlaybackSession() {
                                        let _ = session.SetPlaybackRate(desired_rate);
                                    }
                                    let _ = player.Play();
                                }
                            }
                        }
                        AudioCommand::Play => {
                            if let Ok(session) = player.PlaybackSession() {
                                let _ = session.SetPlaybackRate(desired_rate);
                            }
                            let _ = player.Play();
                        }
                        AudioCommand::Pause => {
                            let _ = player.Pause();
                        }
                        AudioCommand::Stop => {
                            let _ = player.Pause();
                            if let Ok(session) = player.PlaybackSession() {
                                let _ = session.SetPosition(TimeSpan { Duration: 0 });
                            }
                        }
                        AudioCommand::Seek(pos_secs) => {
                            if let Ok(session) = player.PlaybackSession() {
                                let ticks = (pos_secs * 10_000_000.0) as i64;
                                let _ = session.SetPosition(TimeSpan { Duration: ticks });
                            }
                        }
                        AudioCommand::SetVolume(vol) => {
                            let vol_f64 = (vol.clamp(0, 100) as f64) / 100.0;
                            let _ = player.SetVolume(vol_f64);
                        }
                        AudioCommand::SetPlaybackRate(rate) => {
                            desired_rate = rate.clamp(0.5, 2.0);
                            if let Ok(session) = player.PlaybackSession() {
                                let _ = session.SetPlaybackRate(desired_rate);
                            }
                        }
                    }
                }

                // Poll status from Windows Core MediaPlayer
                let mut current_state = 1;
                let mut pos = 0.0;
                let mut dur = 0.0;

                if let Ok(session) = player.PlaybackSession() {
                    if let Ok(state) = session.PlaybackState() {
                        current_state = match state {
                            MediaPlaybackState::Playing => 3,
                            MediaPlaybackState::Paused => 2,
                            MediaPlaybackState::Buffering => 6,
                            MediaPlaybackState::Opening => 9,
                            _ => 1,
                        };
                    }

                    if let Ok(ts_pos) = session.Position() {
                        pos = (ts_pos.Duration as f64) / 10_000_000.0;
                    }

                    if let Ok(ts_dur) = session.NaturalDuration() {
                        dur = (ts_dur.Duration as f64) / 10_000_000.0;
                    }

                    // Keep desired playback rate applied when playing
                    if current_state == 3 {
                        if let Ok(cur_rate) = session.PlaybackRate() {
                            if (cur_rate - desired_rate).abs() > 0.01 {
                                let _ = session.SetPlaybackRate(desired_rate);
                            }
                        }
                    }

                    // Auto-advance trigger: if track reaches the end
                    if dur > 0.0 && pos >= (dur - 0.4) {
                        current_state = 8; // MediaEnded
                    }
                }

                let vol = player.Volume().map(|v| (v * 100.0).round() as i32).unwrap_or(100);

                if let Ok(mut lock) = status_clone.lock() {
                    lock.state = current_state;
                    lock.position = pos;
                    if dur > 0.0 {
                        lock.duration = dur;
                    }
                    lock.volume = vol;
                    lock.playback_rate = desired_rate;
                }

                thread::sleep(Duration::from_millis(50));
            }
        });

        Self { cmd_tx, status }
    }

    pub fn play_url(&self, url: impl Into<String>) {
        let _ = self.cmd_tx.send(AudioCommand::PlayUrl(url.into()));
    }

    pub fn play(&self) {
        let _ = self.cmd_tx.send(AudioCommand::Play);
    }

    pub fn pause(&self) {
        let _ = self.cmd_tx.send(AudioCommand::Pause);
    }

    pub fn toggle_play_pause(&self) {
        let status = self.get_status();
        if status.state == 3 {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn stop(&self) {
        let _ = self.cmd_tx.send(AudioCommand::Stop);
    }

    pub fn seek(&self, seconds: f64) {
        let _ = self.cmd_tx.send(AudioCommand::Seek(seconds));
    }

    pub fn set_volume(&self, volume: i32) {
        let _ = self.cmd_tx.send(AudioCommand::SetVolume(volume));
    }

    pub fn set_playback_rate(&self, rate: f64) {
        let _ = self.cmd_tx.send(AudioCommand::SetPlaybackRate(rate));
    }

    pub fn get_status(&self) -> AudioStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}
