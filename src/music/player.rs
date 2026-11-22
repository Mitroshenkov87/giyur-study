//! Background thread that plays the playlist through rodio, with a
//! crossfade between tracks. The audio device is opened only while music
//! is on, so tests and a silent machine never touch it.

use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

use super::playlist::{ShuffleBag, Track};
use super::{catalog, Catalog};

/// Automatic overlap between the outgoing and incoming tracks.
pub const CROSSFADE: Duration = Duration::from_millis(5000);
/// Shorter overlap when the user presses «Следующий трек».
const SKIP_FADE: Duration = Duration::from_millis(2000);
/// Fade when music is turned off.
const STOP_FADE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub enum Cmd {
    SetEnabled(bool),
    SetVolume(f32),
    Next,
    Shutdown,
}

/// Snapshot the UI reads on a timer.
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub enabled: bool,
    pub title: Option<String>,
    pub error: Option<String>,
    pub available: bool,
    pub track_count: usize,
}

/// Handle held by the UI thread.
pub struct Controller {
    tx: Sender<Cmd>,
    status: Arc<Mutex<Status>>,
    running: Arc<AtomicBool>,
}

impl Controller {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let catalog = catalog().clone();
        let status = Arc::new(Mutex::new(Status {
            enabled: false,
            title: None,
            error: None,
            available: !catalog.tracks.is_empty(),
            track_count: catalog.tracks.len(),
        }));
        let running = Arc::new(AtomicBool::new(true));
        let status_t = Arc::clone(&status);
        let running_t = Arc::clone(&running);
        thread::Builder::new()
            .name("shorashim-music".into())
            .spawn(move || {
                let mut engine = Engine::new(catalog, status_t);
                loop {
                    if !running_t.load(Ordering::SeqCst) {
                        engine.shutdown();
                        break;
                    }
                    match rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(Cmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                            engine.shutdown();
                            break;
                        }
                        Ok(Cmd::SetEnabled(on)) => engine.set_enabled(on),
                        Ok(Cmd::SetVolume(v)) => engine.set_volume(v.clamp(0.0, 1.0)),
                        Ok(Cmd::Next) => engine.skip(),
                        Err(RecvTimeoutError::Timeout) => engine.tick(),
                    }
                }
                running_t.store(false, Ordering::SeqCst);
            })
            .expect("spawn music thread");
        Controller {
            tx,
            status,
            running,
        }
    }

    pub fn status(&self) -> Status {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn set_enabled(&self, on: bool) {
        let _ = self.tx.send(Cmd::SetEnabled(on));
    }

    pub fn set_volume(&self, percent: i64) {
        let v = (percent.clamp(0, 100) as f32) / 100.0;
        let _ = self.tx.send(Cmd::SetVolume(v));
    }

    pub fn next(&self) {
        let _ = self.tx.send(Cmd::Next);
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = self.tx.send(Cmd::Shutdown);
    }
}

struct Slot {
    player: Player,
    track: Track,
    started: Instant,
    duration: Duration,
    fade_out_from: Option<Instant>,
    fade_out_len: Duration,
    fade_in_len: Duration,
}

struct Engine {
    bag: ShuffleBag,
    rng: StdRng,
    status: Arc<Mutex<Status>>,
    enabled: bool,
    volume: f32,
    stream: Option<MixerDeviceSink>,
    current: Option<Slot>,
    incoming: Option<Slot>,
    /// After «off»: wait for the short fade, then drop the device.
    stopping_from: Option<Instant>,
}

impl Engine {
    fn new(catalog: Catalog, status: Arc<Mutex<Status>>) -> Self {
        Engine {
            bag: ShuffleBag::new(catalog.tracks),
            rng: StdRng::from_entropy(),
            status,
            enabled: false,
            volume: 0.3,
            stream: None,
            current: None,
            incoming: None,
            stopping_from: None,
        }
    }

    fn set_enabled(&mut self, on: bool) {
        if on == self.enabled && self.stopping_from.is_none() {
            return;
        }
        self.enabled = on;
        if on {
            self.stopping_from = None;
            if self.bag.is_empty() {
                self.publish(Some("Файлы музыки не найдены.".to_string()));
                return;
            }
            if self.current.is_none() && self.incoming.is_none() {
                if let Err(e) = self.start_track(Duration::from_millis(800), true) {
                    self.publish(Some(e));
                    return;
                }
            }
            self.publish(None);
        } else {
            self.begin_fade_out(STOP_FADE);
            self.stopping_from = Some(Instant::now());
            self.apply_volumes();
            self.publish(None);
        }
    }

    fn set_volume(&mut self, v: f32) {
        self.volume = v;
        self.apply_volumes();
    }

    fn skip(&mut self) {
        if !self.enabled || self.bag.is_empty() {
            return;
        }
        // Finish any in-flight fade immediately, then start the next track.
        if let Some(incoming) = self.incoming.take() {
            drop(self.current.take());
            self.current = Some(incoming);
        }
        if let Err(e) = self.start_track(SKIP_FADE, false) {
            self.publish(Some(e));
            return;
        }
        self.publish(None);
    }

    fn tick(&mut self) {
        if let Some(from) = self.stopping_from {
            self.apply_volumes();
            if from.elapsed() >= STOP_FADE {
                self.drop_playback();
                self.stopping_from = None;
                self.publish(None);
            }
            return;
        }
        if !self.enabled {
            return;
        }
        if self.current.is_none() && self.incoming.is_none() {
            if let Err(e) = self.start_track(Duration::from_millis(800), true) {
                self.publish(Some(e));
            } else {
                self.publish(None);
            }
            return;
        }
        // Promote a finished incoming fade to current.
        if let (Some(cur), Some(_)) = (self.current.as_ref(), self.incoming.as_ref()) {
            if let Some(from) = cur.fade_out_from {
                if from.elapsed() >= cur.fade_out_len {
                    drop(self.current.take());
                    self.current = self.incoming.take();
                    self.publish(None);
                }
            }
        }
        // Time to start the automatic crossfade.
        if self.incoming.is_none() {
            if let Some(cur) = self.current.as_ref() {
                if cur.fade_out_from.is_none() && should_crossfade(cur, CROSSFADE) {
                    if let Err(e) = self.start_track(CROSSFADE, false) {
                        self.publish(Some(e));
                    } else {
                        self.publish(None);
                    }
                }
            }
        }
        self.apply_volumes();
    }

    fn start_track(&mut self, fade_in: Duration, as_current: bool) -> Result<(), String> {
        let track = self
            .bag
            .next(&mut self.rng)
            .cloned()
            .ok_or_else(|| "Файлы музыки не найдены.".to_string())?;
        if self.stream.is_none() {
            let sink = DeviceSinkBuilder::open_default_sink()
                .map_err(|e| format!("Нет звукового устройства: {e}"))?;
            self.stream = Some(sink);
        }
        let mixer = self.stream.as_ref().expect("stream just opened").mixer();
        let file = File::open(&track.path)
            .map_err(|e| format!("Не удалось открыть «{}»: {e}", track.title))?;
        let decoder = Decoder::try_from(file)
            .map_err(|e| format!("Не удалось прочитать «{}»: {e}", track.title))?;
        let duration = track
            .duration
            .or_else(|| decoder.total_duration())
            .unwrap_or(Duration::from_secs(180));
        let player = Player::connect_new(mixer);
        player.append(decoder.fade_in(fade_in));
        player.set_volume(self.volume);
        let slot = Slot {
            player,
            track,
            started: Instant::now(),
            duration,
            fade_out_from: None,
            fade_out_len: Duration::ZERO,
            fade_in_len: fade_in,
        };
        if as_current || self.current.is_none() {
            drop(self.current.take());
            self.current = Some(slot);
        } else {
            if let Some(cur) = self.current.as_mut() {
                cur.fade_out_from = Some(Instant::now());
                cur.fade_out_len = fade_in;
            }
            drop(self.incoming.take());
            self.incoming = Some(slot);
        }
        Ok(())
    }

    fn begin_fade_out(&mut self, len: Duration) {
        let now = Instant::now();
        for slot in [&mut self.current, &mut self.incoming]
            .into_iter()
            .flatten()
        {
            if slot.fade_out_from.is_none() {
                slot.fade_out_from = Some(now);
                slot.fade_out_len = len;
            }
        }
    }

    fn apply_volumes(&self) {
        let now = Instant::now();
        for slot in [&self.current, &self.incoming].into_iter().flatten() {
            slot.player.set_volume(slot_volume(slot, self.volume, now));
        }
    }

    fn drop_playback(&mut self) {
        self.current = None;
        self.incoming = None;
        self.stream = None;
    }

    fn shutdown(&mut self) {
        self.drop_playback();
    }

    fn publish(&self, error: Option<String>) {
        if let Ok(mut s) = self.status.lock() {
            s.enabled = self.enabled && self.stopping_from.is_none();
            s.title = self
                .incoming
                .as_ref()
                .or(self.current.as_ref())
                .map(|t| t.track.title.clone());
            s.error = error;
            s.available = !self.bag.is_empty();
            s.track_count = self.bag.len();
        }
    }
}

fn should_crossfade(slot: &Slot, fade: Duration) -> bool {
    let played = slot.started.elapsed();
    if slot.duration > fade && played + fade >= slot.duration {
        return true;
    }
    // Unknown or very short duration: wait until the player runs dry.
    played > Duration::from_millis(400) && slot.player.empty()
}

fn slot_volume(slot: &Slot, volume: f32, now: Instant) -> f32 {
    let mut v = volume;
    if let Some(from) = slot.fade_out_from {
        let t = if slot.fade_out_len.is_zero() {
            1.0
        } else {
            now.saturating_duration_since(from).as_secs_f32() / slot.fade_out_len.as_secs_f32()
        };
        v *= 1.0 - t.clamp(0.0, 1.0);
    }
    let _ = slot.fade_in_len; // applied on the source itself
    v
}
