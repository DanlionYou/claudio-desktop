use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

use rodio::{Decoder, OutputStream, Sink, Source};

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
    playback_started_at: Option<Instant>,
    paused_elapsed: f64,
    current_duration: f64,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, String> {
        let (stream, stream_handle) =
            OutputStream::try_default().map_err(|e| format!("音频输出创建失败: {}", e))?;
        let sink =
            Sink::try_new(&stream_handle).map_err(|e| format!("音频播放器创建失败: {}", e))?;
        Ok(Self {
            _stream: stream,
            sink,
            playback_started_at: None,
            paused_elapsed: 0.0,
            current_duration: 0.0,
        })
    }

    pub fn play(&mut self, path: &str) -> Result<f64, String> {
        let file =
            File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("无法解码音频: {}", e))?;

        let duration = source
            .total_duration()
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        self.sink.stop();
        self.sink.append(source);
        self.sink.play();

        self.playback_started_at = Some(Instant::now());
        self.paused_elapsed = 0.0;
        self.current_duration = duration;

        Ok(duration)
    }

    pub fn pause(&mut self) {
        self.sink.pause();
        if let Some(start) = self.playback_started_at {
            self.paused_elapsed += start.elapsed().as_secs_f64();
        }
        self.playback_started_at = None;
    }

    pub fn resume(&mut self) {
        self.sink.play();
        self.playback_started_at = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.playback_started_at = None;
        self.paused_elapsed = 0.0;
    }

    pub fn set_volume(&self, vol: f64) {
        self.sink.set_volume(vol as f32);
    }

    pub fn get_elapsed(&self) -> f64 {
        if let Some(start) = self.playback_started_at {
            self.paused_elapsed + start.elapsed().as_secs_f64()
        } else {
            self.paused_elapsed
        }
    }

    pub fn current_duration(&self) -> f64 {
        self.current_duration
    }

    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }
}

// AudioPlayer fields are safe to send between threads:
// - OutputStream: wraps cpal, which is thread-safe
// - Sink: uses Arc internally
// - Instant, f64: trivially Send
unsafe impl Send for AudioPlayer {}

static AUDIO_PLAYER: OnceLock<Mutex<Option<AudioPlayer>>> = OnceLock::new();

pub fn get_audio_player() -> &'static Mutex<Option<AudioPlayer>> {
    AUDIO_PLAYER.get_or_init(|| Mutex::new(None))
}
