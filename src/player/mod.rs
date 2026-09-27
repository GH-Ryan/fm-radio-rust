use crate::radio::Station;
use rodio::{OutputStream, Sink};
use std::io::{BufReader, Read};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlayerError {
    #[error("音频输出不可用：{0}")]
    OutputUnavailable(String),
    #[error("无法连接该电台：{0}")]
    ConnectionFailed(String),
    #[error("{0}")]
    UnsupportedFormat(String),
    #[error("解码失败：{0}")]
    DecodeError(String),
    #[error("电台已停止传输")]
    StreamEnded,
    #[error("操作被取消")]
    Cancelled,
    #[error("没有正在播放的电台")]
    NothingPlaying,
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("请求错误: {0}")]
    Request(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Default)]
pub struct PlayerState {
    pub playing: bool,
    pub station: Option<Station>,
    pub title: String,
    pub backend: String,
    pub error: Option<String>,
    pub volume: f32,
    pub paused: bool,
    pub playing_key: Option<String>,
}

type StateCallback = Box<dyn Fn(PlayerState) + Send + Sync>;

// #[derive(Debug)]
pub struct Player {
    state: Arc<Mutex<PlayerState>>,
    sink: Arc<Mutex<Option<Sink>>>,
    stream_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    cancel_flag: Arc<Mutex<bool>>,
    // Bumped on every play/stop. A thread whose generation is stale belongs to a
    // station the user has already moved on from.
    generation: Arc<Mutex<u64>>,
    on_change: Arc<Mutex<Option<StateCallback>>>,
    volume: Arc<Mutex<f32>>,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PlayerState {
                volume: 0.8,
                ..Default::default()
            })),
            sink: Arc::new(Mutex::new(None)),
            stream_handle: Arc::new(Mutex::new(None)),
            cancel_flag: Arc::new(Mutex::new(false)),
            generation: Arc::new(Mutex::new(0)),
            on_change: Arc::new(Mutex::new(None)),
            volume: Arc::new(Mutex::new(0.8)),
        }
    }

    pub fn set_on_change<F>(&self, callback: F)
    where
        F: Fn(PlayerState) + Send + Sync + 'static,
    {
        *self.on_change.lock().unwrap() = Some(Box::new(callback));
    }

    fn emit(&self) {
        let state = self.state.lock().unwrap().clone();
        if let Some(cb) = &*self.on_change.lock().unwrap() {
            cb(state);
        }
    }

    pub fn volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    pub fn set_volume(&self, v: f32) {
        let clamped = v.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped;
        if let Some(sink) = &*self.sink.lock().unwrap() {
            sink.set_volume(clamped);
        }
        self.emit();
    }

    pub fn state(&self) -> PlayerState {
        let mut state = self.state.lock().unwrap().clone();
        state.volume = self.volume();
        state
    }

    pub fn play(&self, station: Station) -> Result<(), PlayerError> {
        self.stop();
        *self.generation.lock().unwrap() += 1;
        *self.cancel_flag.lock().unwrap() = false;

        let stream_url = station.stream_url.clone();
        if stream_url.trim().is_empty() {
            return Err(PlayerError::UnsupportedFormat(
                "该电台缺少播放地址".to_string(),
            ));
        }

        {
            let mut state = self.state.lock().unwrap();
            state.station = Some(station.clone());
            state.title = String::new();
            state.backend = "连接中".to_string();
            state.error = None;
            state.paused = false;
            state.playing = true;
            state.playing_key = Some(station.key());
        }
        self.emit();

        let state = self.state.clone();
        let sink = self.sink.clone();
        let stream_handle = self.stream_handle.clone();
        let cancel_flag = self.cancel_flag.clone();
        let generation = self.generation.clone();
        let volume = self.volume.clone();
        let on_change = self.on_change.clone();

        let mine = *generation.lock().unwrap();
        let handle = thread::spawn(move || {
            let result = Self::run_playback(
                stream_url,
                state.clone(),
                sink.clone(),
                cancel_flag.clone(),
                volume.clone(),
                on_change.clone(),
                generation.clone(),
                mine,
            );
            if *generation.lock().unwrap() != mine {
                // A newer station took over while this one was still running.
                return;
            }
            if let Err(e) = result {
                let mut state = state.lock().unwrap();
                if *cancel_flag.lock().unwrap() {
                    state.error = None;
                } else {
                    state.error = Some(e.to_string());
                    state.playing = false;
                    state.playing_key = None;
                }
                state.paused = false;
                if let Some(cb) = &*on_change.lock().unwrap() {
                    cb(state.clone());
                }
            }
            let mut slot = stream_handle.lock().unwrap();
            if slot
                .as_ref()
                .is_some_and(|h| h.thread().id() == std::thread::current().id())
            {
                *slot = None;
            }
        });

        *self.stream_handle.lock().unwrap() = Some(handle);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn run_playback(
        stream_url: String,
        state: Arc<Mutex<PlayerState>>,
        sink_arc: Arc<Mutex<Option<Sink>>>,
        cancel_flag: Arc<Mutex<bool>>,
        volume: Arc<Mutex<f32>>,
        on_change: Arc<Mutex<Option<StateCallback>>>,
        generation: Arc<Mutex<u64>>,
        mine: u64,
    ) -> Result<(), PlayerError> {
        let client = reqwest::blocking::Client::builder()
            // Only the wait for a connection or for the response head is bounded;
            // the body has no timeout, because a station that goes quiet for a
            // while is normal and must not be treated as a dead stream.
            .connect_timeout(Duration::from_secs(12))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15 wails.io/605.1.15")
            .build()?;

        let mut request = client.get(&stream_url);
        request = request.header("Icy-MetaData", "1");
        let response = request.send()?;

        if !response.status().is_success() {
            return Err(PlayerError::ConnectionFailed(format!(
                "服务器返回 HTTP {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // ICY servers interleave metadata blocks into the audio; left in place
        // they reach the decoder as garbage.
        let icy_metaint = response
            .headers()
            .get("icy-metaint")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|n| *n > 0);

        let title_sink = state.clone();
        let on_meta = on_change.clone();
        let body: Box<dyn std::io::Read + Send + Sync> = match icy_metaint {
            Some(interval) => Box::new(IcyReader::new(response, interval, move |title| {
                if title.is_empty() {
                    return;
                }
                title_sink.lock().unwrap().title = title;
                let snapshot = title_sink.lock().unwrap().clone();
                let guard = on_meta.lock().unwrap();
                if let Some(cb) = guard.as_ref() {
                    cb(snapshot);
                }
            })),
            None => Box::new(response),
        };
        let reader = BufReader::with_capacity(64 * 1024, body);

        // A live stream has no file header to be recognised by -- an MP3
        // broadcast is a bare run of frames -- so the container is named from
        // the response headers and handed to the probe as a hint. Without it
        // the probe has nothing to go on and reports an unrecognised format.
        let mut hint = Hint::new();
        match sniff(&content_type) {
            Some((mime, ext)) => {
                hint.mime_type(mime);
                hint.with_extension(ext);
            }
            None => {
                return Err(PlayerError::UnsupportedFormat(
                    "无法识别该电台的音频格式".to_string(),
                ))
            }
        };

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                MediaSourceStream::new(
                    Box::new(symphonia::core::io::ReadOnlySource::new(reader)),
                    Default::default(),
                ),
                &symphonia::core::formats::FormatOptions::default(),
                &symphonia::core::meta::MetadataOptions::default(),
            )
            .map_err(|_| {
                PlayerError::UnsupportedFormat(
                    "该电台的音频格式无法识别，本应用无法解码".to_string(),
                )
            })?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| PlayerError::UnsupportedFormat("该电台没有可播放的音轨".to_string()))?;
        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44_100).max(8_000);
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &Default::default())
            .map_err(|e| PlayerError::DecodeError(format!("无法创建解码器：{}", e)))?;

        let (_out, out_handle) = OutputStream::try_default()
            .map_err(|e| PlayerError::OutputUnavailable(e.to_string()))?;
        let sink = Sink::try_new(&out_handle)
            .map_err(|e| PlayerError::OutputUnavailable(e.to_string()))?;
        sink.set_volume(*volume.lock().unwrap());
        *sink_arc.lock().unwrap() = Some(sink);

        {
            let mut st = state.lock().unwrap();
            st.backend = format!("内置解码 · {} Hz", sample_rate);
        }
        if let Some(cb) = &*on_change.lock().unwrap() {
            cb(state.lock().unwrap().clone());
        }

        let mut pcm: Vec<f32> = Vec::with_capacity(16_384);
        loop {
            if *cancel_flag.lock().unwrap() || *generation.lock().unwrap() != mine {
                return Ok(());
            }

            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Err(PlayerError::StreamEnded)
                }
                Err(symphonia::core::errors::Error::ResetRequired) => continue,
                // A read that timed out means the station sent nothing for a
                // moment. That is ordinary for a live stream, so it is not an
                // error -- but it is also the only point where a blocking read
                // returns, which is what lets a stop take effect.
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    continue
                }
                Err(e) => return Err(PlayerError::DecodeError(format!("读取音频数据失败：{}", e))),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let channels = spec.channels.count().clamp(1, 2);
                    let mut sb = symphonia::core::audio::SampleBuffer::<f32>::new(
                        decoded.capacity() as u64,
                        spec,
                    );
                    sb.copy_interleaved_ref(decoded);
                    let samples = sb.samples();
                    pcm.reserve(samples.len());
                    for frame in samples.chunks(channels) {
                        pcm.push(frame[0]);
                        pcm.push(if channels > 1 { frame[1] } else { frame[0] });
                    }
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                Err(e) => return Err(PlayerError::DecodeError(format!("解码失败：{}", e))),
            }

            // Hand finished audio to the device in blocks, so the decoder stays
            // slightly ahead of playback rather than buffering without bound.
            if pcm.len() >= 8192 {
                let vol = *volume.lock().unwrap();
                let block: Vec<f32> = pcm.drain(..).map(|s| s * vol).collect();
                if let Some(sink) = &*sink_arc.lock().unwrap() {
                    sink.append(rodio::buffer::SamplesBuffer::new(
                        2,
                        sample_rate as u32,
                        block,
                    ));
                }
            }
        }
    }

    pub fn pause(&self) -> Result<(), PlayerError> {
        {
            let mut state = self.state.lock().unwrap();
            if !(state.playing && !state.paused) {
                return Ok(());
            }
            state.paused = true;
        }
        if let Some(sink) = &*self.sink.lock().unwrap() {
            sink.pause();
        }
        self.emit();
        Ok(())
    }

    pub fn resume(&self) -> Result<(), PlayerError> {
        {
            let mut state = self.state.lock().unwrap();
            if !(state.playing && state.paused) {
                return Ok(());
            }
            state.paused = false;
        }
        if let Some(sink) = &*self.sink.lock().unwrap() {
            sink.play();
        }
        self.emit();
        Ok(())
    }

    /// Toggles playback, and starts the station already in the panel when
    /// nothing is running.
    ///
    /// A station restored from the last session sits in the player without being
    /// started, so the panel has something to show on launch. Treating that as
    /// "nothing playing" left the play button reporting an error while the
    /// panel plainly named a station, so pressing it now starts that station.
    pub fn play_pause(&self) -> Result<(), PlayerError> {
        let state = self.state();
        if !state.playing {
            return match state.station {
                Some(station) if !station.stream_url.trim().is_empty() => self.play(station),
                _ => Err(PlayerError::NothingPlaying),
            };
        }
        if state.paused {
            self.resume()
        } else {
            self.pause()
        }
    }

    pub fn stop(&self) {
        *self.cancel_flag.lock().unwrap() = true;
        // Retires the running thread's claim on the state and the sink.
        *self.generation.lock().unwrap() += 1;

        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }

        // Closing the response is what unblocks the decode thread: it is parked
        // in a socket read and only returns to look at the cancel flag once that
        // read completes. A station that has gone quiet would otherwise hold the
        // join below for as long as it likes, and with it the next station.
        // The decode thread is parked in a socket read and only reaches its
        // cancel check once that read returns, so joining it here would block
        // for as long as the station stays quiet -- and a station that never
        // buffers again is exactly the case a stop has to survive. The stream is
        // therefore not joined: it is left to notice the flag and unwind, and
        // the next session simply supersedes its result.
        self.stream_handle.lock().unwrap().take();

        {
            let mut state = self.state.lock().unwrap();
            state.playing = false;
            state.paused = false;
            state.playing_key = None;
            state.backend = String::new();
            state.error = None;
        }
        self.emit();
    }

    pub fn restore(&self, station: Station) {
        {
            let mut state = self.state.lock().unwrap();
            if state.playing {
                return;
            }
            state.station = Some(station);
            state.title = String::new();
            state.backend = String::new();
            state.error = None;
            state.paused = false;
        }
        self.emit();
    }
}

/// Names the container a stream is likely to be, from what the server said.
///
/// A live stream cannot be identified from a file header the way a file can: an
/// MP3 broadcast is a bare run of frames with no leading ID3 tag. The declared
/// content type is therefore the only evidence available, and it is passed to the
/// probe as a hint rather than a guarantee.
fn sniff(content_type: &str) -> Option<(&'static str, &'static str)> {
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let found = match ct.as_str() {
        "audio/mpeg" | "audio/mp3" | "audio/mpeg3" | "audio/x-mpeg" => ("audio/mpeg", "mp3"),
        "audio/aac" | "audio/aacp" | "audio/x-aac" => ("audio/aac", "aac"),
        "application/ogg" | "audio/ogg" | "audio/vorbis" | "application/vnd.ogg" => {
            ("audio/ogg", "ogg")
        }
        "audio/opus" | "audio/ogg; codecs=opus" => ("audio/ogg", "opus"),
        "audio/flac" | "audio/x-flac" => ("audio/flac", "flac"),
        "audio/wav" | "audio/x-wav" | "audio/wave" | "audio/vnd.wave" => ("audio/wav", "wav"),
        // Some servers send a generic type for anything they serve.
        "audio" | "application/octet-stream" | "application/x-ogg" => ("audio/mpeg", "mp3"),
        _ => return None,
    };
    Some(found)
}

/// Strips the metadata blocks an ICY server interleaves into the audio stream.
///
/// The server announces the spacing with `icy-metaint`; every interval bytes of
/// audio are followed by one length byte and that many 16-byte blocks of
/// metadata. Left in place the block reaches the decoder as audio and is heard
/// as a click, so it is consumed here instead.
struct IcyReader<R, F> {
    source: R,
    interval: usize,
    audio_left: usize,
    on_meta: F,
}

impl<R, F> IcyReader<R, F>
where
    R: Read,
    F: FnMut(String),
{
    fn new(source: R, interval: usize, on_meta: F) -> Self {
        Self {
            source,
            interval,
            audio_left: interval,
            on_meta,
        }
    }

    /// Consumes one metadata block, handing the title it carries to `on_meta`.
    fn skip_metadata(&mut self) -> std::io::Result<()> {
        let mut header = [0u8; 1];
        self.source.read_exact(&mut header)?;
        let size = header[0] as usize * 16;
        if size == 0 {
            return Ok(());
        }
        let mut block = vec![0u8; size];
        self.source.read_exact(&mut block)?;
        (self.on_meta)(icy_title(&block));
        Ok(())
    }
}

/// Pulls `StreamTitle` out of a raw ICY metadata block.
fn icy_title(block: &[u8]) -> String {
    let text = String::from_utf8_lossy(block);
    for part in text.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case("streamtitle") {
            continue;
        }
        let mut value = value.trim();
        // Stations quote the value and escape embedded quotes as \'.
        // An opening quote with nothing after it means the block was cut off
        // before the title arrived, which is an empty title -- not a title
        // made of the one quote that survived.
        if let Some(rest) = value.strip_prefix('\'') {
            value = rest.strip_suffix('\'').unwrap_or("");
        }
        // \' is just ' in a Rust literal, so only the \\' escape needs stripping.
        return value.replace("\\'", "'").trim().to_string();
    }
    String::new()
}

impl<R: Read, F: FnMut(String)> Read for IcyReader<R, F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.audio_left == 0 {
            self.skip_metadata()?;
            self.audio_left = self.interval;
        }
        // Never read past the interval, or audio would be counted as metadata.
        let limit = buf.len().min(self.audio_left);
        let n = self.source.read(&mut buf[..limit])?;
        self.audio_left -= n;
        Ok(n)
    }
}

#[cfg(test)]
#[path = "format_test.rs"]
mod format_test;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::time::Instant;

    fn station(name: &str, url: &str) -> Station {
        Station {
            uuid: format!("u-{name}"),
            name: name.to_string(),
            stream_url: url.to_string(),
            ..Default::default()
        }
    }

    /// A local server that sends headers and then trickles bytes for as long as
    /// the test needs. Nothing here reaches the network, and the body never ends,
    /// the way a station's does not.
    struct SlowStream {
        url: String,
        stop: Arc<AtomicBool>,
    }

    impl SlowStream {
        fn start(name: &str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/{name}", listener.local_addr().unwrap());
            let stop = Arc::new(AtomicBool::new(false));
            let flag = stop.clone();

            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(mut conn) = stream else { continue };
                    let stop_for_conn = flag.clone();
                    std::thread::spawn(move || {
                        let _ =
                            conn.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\n\r\n");
                        let _ = conn.flush();
                        while !stop_for_conn.load(Ordering::Relaxed) {
                            if conn.write_all(&[0u8; 64]).is_err() {
                                break;
                            }
                            let _ = conn.flush();
                            std::thread::sleep(Duration::from_millis(5));
                        }
                    });
                }
            });

            SlowStream { url, stop }
        }
    }

    impl Drop for SlowStream {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    fn eventually(check: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if check() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        check()
    }

    #[test]
    fn switching_stations_leaves_the_new_one_playing() {
        let a = SlowStream::start("a");
        let b = SlowStream::start("b");
        let p = Player::new();

        p.play(station("A", &a.url)).unwrap();
        p.play(station("B", &b.url)).unwrap();

        // Whichever order the two decode threads finish in, the panel must name
        // the station that was asked for last.
        let ok = eventually(|| p.state().station.as_ref().map(|s| s.name.as_str()) == Some("B"));
        assert!(
            ok,
            "player ended up on {:?}",
            p.state().station.map(|s| s.name)
        );
    }

    #[test]
    fn a_superseded_thread_cannot_report_its_own_failure() {
        // The first stream is refused outright, so its thread fails at once and
        // would normally stamp its error over whatever is playing now.
        let refused = "http://127.0.0.1:1/nothing.mp3";
        let live = SlowStream::start("live");
        let (tx, rx) = mpsc::channel();
        let p = Player::new();
        p.set_on_change(move |s| {
            let _ = tx.send(s.station.map(|st| st.name));
        });

        p.play(station("DEAD", refused)).unwrap();
        p.play(station("LIVE", &live.url)).unwrap();

        // Both play() calls report synchronously, and naming the station the
        // user just picked is correct. Only what arrives afterwards is the
        // retired thread talking, and it has nothing to say.
        while rx.try_recv().is_ok() {}

        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Ok(Some(name)) = rx.try_recv() {
                assert_ne!(
                    name, "DEAD",
                    "a retired thread reported state for the station it was replaced for"
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// A thread left in the handle slot cannot be cancelled.
    ///
    /// This passes with or without the id check, because `stop` already takes
    /// the slot; the check guards the case where an older thread outlives a newer
    /// one and would otherwise clear the newer one's entry.
    #[test]
    fn every_started_stream_is_eventually_retired() {
        let a = SlowStream::start("a");
        let p = Player::new();
        p.play(station("A", &a.url)).unwrap();
        assert!(eventually(|| p.state().playing));
        p.stop();
        let empty = eventually(|| p.stream_handle.lock().unwrap().is_none());
        assert!(empty, "a decode thread was left tracked after a stop");
    }

    #[test]
    fn stopping_ends_playback_and_the_panel_stays_readable() {
        let s = SlowStream::start("s");
        let p = Player::new();
        p.play(station("S", &s.url)).unwrap();
        assert!(eventually(|| p.state().playing));

        p.stop();
        let after = p.state();
        assert!(!after.playing, "still playing after a stop");
        assert!(!after.paused);
        assert!(after.playing_key.is_none());
        // The panel keeps naming the station, so the play button can restart it.
        assert_eq!(after.station.map(|x| x.name), Some("S".to_string()));
        assert!(after.error.is_none());
    }

    #[test]
    fn a_stopped_player_can_be_started_again() {
        let a = SlowStream::start("a");
        let p = Player::new();
        p.play(station("A", &a.url)).unwrap();
        assert!(eventually(|| p.state().playing));
        p.stop();
        p.play(station("A", &a.url)).unwrap();
        assert!(
            eventually(|| p.state().playing),
            "restarting after a stop did not take"
        );
    }

    #[test]
    fn pausing_and_resuming_do_not_lose_the_station() {
        let s = SlowStream::start("s");
        let p = Player::new();
        p.play(station("S", &s.url)).unwrap();
        assert!(eventually(|| p.state().playing));

        p.pause().unwrap();
        assert!(p.state().paused);
        assert_eq!(p.state().station.map(|x| x.name), Some("S".to_string()));

        p.resume().unwrap();
        assert!(!p.state().paused);
        assert!(p.state().playing);
    }

    #[test]
    fn toggling_with_nothing_loaded_reports_it() {
        let p = Player::new();
        assert!(matches!(p.play_pause(), Err(PlayerError::NothingPlaying)));
    }

    #[test]
    fn toggling_starts_the_restored_station() {
        let s = SlowStream::start("r");
        let p = Player::new();
        p.restore(station("R", &s.url));
        let _ = p.play_pause();
        assert!(
            eventually(|| p.state().playing),
            "the restored station was not started"
        );
    }

    #[test]
    fn a_restored_station_without_a_stream_cannot_be_started() {
        let p = Player::new();
        p.restore(station("R", "   "));
        assert!(matches!(p.play_pause(), Err(PlayerError::NothingPlaying)));
    }

    #[test]
    fn volume_is_clamped_to_the_usable_range() {
        let p = Player::new();
        p.set_volume(2.0);
        assert_eq!(p.volume(), 1.0);
        p.set_volume(-1.0);
        assert_eq!(p.volume(), 0.0);
        p.set_volume(0.5);
        assert_eq!(p.volume(), 0.5);
    }

    #[test]
    fn state_reports_the_volume_actually_in_effect() {
        let p = Player::new();
        p.set_volume(0.25);
        assert_eq!(p.state().volume, 0.25);
    }
}
