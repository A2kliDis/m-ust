use anyhow::Result;
use crossterm::{
    event::{Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{time::{Duration, Instant}, io};
use crate::{config::AppConfig, audio::{CaptureMode, AudioCapture, list_input_devices, list_output_devices, list_app_sessions}, fingerprint::generate_shazam_signature, api::{recognize_with_shazam, recognize_with_acoustid}};
#[cfg(target_os = "windows")]
use crate::audio::get_peak_for_pid;

#[derive(Clone, Debug, PartialEq)]
pub enum Status { Idle, Recording, Identifying, Done, Error }

#[derive(Clone, Debug)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub url: Option<String>,
}

pub struct App {
    pub config: AppConfig,
    pub status: Status,
    pub mode: CaptureMode,
    pub devices: Vec<String>,
    pub output_devices: Vec<String>,
    pub apps: Vec<String>,
    pub app_peaks: Vec<f32>,
    pub selected_device: usize,
    pub selected_output: usize,
    pub selected_app: usize,
    pub capture_tab: usize,
    pub log: Vec<String>,
    pub result: Option<SongInfo>,
    pub error: Option<String>,
    pub progress: f32,
    pub elapsed: u64,
    pub record_start: Option<Instant>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let devices_raw = list_input_devices().unwrap_or_default().into_iter().map(|d| d.name).collect::<Vec<_>>();
        let output_raw = list_output_devices().into_iter().map(|d| d.name).collect::<Vec<_>>();
        let apps_raw = list_app_sessions();
        let devices = if devices_raw.is_empty() { vec!["Default".into()] } else { devices_raw };
        let output_devices = if output_raw.is_empty() { vec!["Default".into()] } else { output_raw };
        let apps = if apps_raw.is_empty() { vec!["No active audio apps - play music then press g".into()] } else { apps_raw };
        let peaks = vec![0.0; apps.len().max(1)];
        let selected_device = config.default_input.as_ref().and_then(|d| devices.iter().position(|x| x==d)).unwrap_or(0);
        let selected_output = config.default_output.as_ref().and_then(|d| output_devices.iter().position(|x| x==d)).unwrap_or(0);
        let selected_app = 0; // Apps are dynamic, never saved
        let capture_tab = config.default_tab.unwrap_or(1).min(2);
        let mut log = vec!["Ready".into()];
        if let Some(def) = config.default_output.as_ref().or(config.default_input.as_ref()) {
            log.push(format!("Default: {}", def));
        }
        Self {
            config,
            status: Status::Idle,
            mode: CaptureMode::SystemLoopback("Default".into()),
            devices,
            output_devices,
            apps,
            app_peaks: peaks,
            selected_device,
            selected_output,
            selected_app,
            capture_tab,
            log,
            result: None,
            error: None,
            progress: 0.0,
            elapsed: 0,
            record_start: None,
        }
    }
    fn current_mode(&self) -> CaptureMode {
        match self.capture_tab {
            0 => CaptureMode::Microphone(self.devices.get(self.selected_device).cloned().unwrap_or_default()),
            1 => CaptureMode::SystemLoopback(self.output_devices.get(self.selected_output).cloned().unwrap_or_else(|| "Default".into())),
            2 => CaptureMode::AppLoopback(self.apps.get(self.selected_app).cloned().unwrap_or_default()),
            _ => CaptureMode::SystemLoopback("Default".into()),
        }
    }
    pub fn push_log(&mut self, s: impl Into<String>) { self.log.push(s.into()); if self.log.len()>100 { self.log.remove(0); } }
    #[cfg(target_os = "windows")]
    fn update_peaks(&mut self) {
        if self.capture_tab != 2 { return; }
        if self.apps.is_empty() { return; }
        if self.app_peaks.len() != self.apps.len() {
            self.app_peaks = vec![0.0; self.apps.len()];
        }
        for (i, app_str) in self.apps.iter().enumerate() {
            if let Some(pid) = parse_pid(app_str) {
                let peak = get_peak_for_pid(pid);
                self.app_peaks[i] = peak;
            } else {
                self.app_peaks[i] = 0.0;
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    fn update_peaks(&mut self) {}
}

fn parse_pid(s: &str) -> Option<u32> {
    if let Some(start) = s.find("(PID ") {
        let rest = &s[start+5..];
        if let Some(end) = rest.find(')') {
            return rest[..end].trim().parse().ok();
        }
    }
    s.trim().parse().ok()
}

pub async fn run(config: AppConfig) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(config);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Result<SongInfo, String>>();
    let mut events = crossterm::event::EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(100));
    // Draw immediately
    terminal.draw(|f| super::ui::draw(f, &app))?;
    loop {
        tokio::select! {
            maybe_event = events.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    if key.kind != KeyEventKind::Press { continue; }
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') if app.status==Status::Idle || app.status==Status::Done || app.status==Status::Error => {
                            let mode = app.current_mode();
                            let secs = app.config.record_duration_secs;
                            let cfg = app.config.clone();
                            let friendly = match &mode {
                                CaptureMode::Microphone(n) => if n.is_empty() {"Microphone".into()} else {n.clone()},
                                CaptureMode::SystemLoopback(n) => if n=="Default" {"System".into()} else {n.clone()},
                                CaptureMode::AppLoopback(n) => n.clone(),
                            };
                            app.mode = mode.clone();
                            app.status = Status::Recording;
                            app.progress = 0.0;
                            app.elapsed = 0;
                            app.result = None;
                            app.error = None;
                            app.record_start = Some(Instant::now());
                            app.push_log(format!("Recording {} • {}s", friendly, secs));
                            let tx2 = tx.clone();
                            tokio::task::spawn_local(async move {
                                let capture = AudioCapture::new(mode, secs);
                                let res: Result<SongInfo, String> = match capture.record().await {
                                    Ok(audio) => {
                                        let sig = generate_shazam_signature(&audio.samples);
                                        match recognize_with_shazam(&sig).await {
                                            Ok(r) => Ok(SongInfo{title:r.title, artist:r.artist, album:r.album, url:r.url}),
                                            Err(e) => {
                                                let msg = e.to_string();
                                                if let Some(key) = cfg.acoustid_api_key.clone() {
                                                    match recognize_with_acoustid(&audio.samples, 16000, &key).await {
                                                        Ok(ac) => Ok(SongInfo{title:ac.title, artist:ac.artist, album:ac.album, url:None}),
                                                        Err(ae) => Err(if msg.contains("matches") { format!("Song not recognized (Shazam+AcoustID). {} — install fpcalc or try louder volume", ae) } else { format!("Shazam: {} | AcoustID: {}", e, ae)}),
                                                    }
                                                } else {
                                                    Err(if msg.contains("matches") { "Song not recognized. Try louder volume or popular song".into() } else { e.to_string() })
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => Err(format!("Capture failed: {}", e)),
                                };
                                let _ = tx2.send(res);
                            });
                        },
                        KeyCode::Tab => {
                            if app.status == Status::Done || app.status == Status::Error {
                                app.status = Status::Idle;
                                app.error = None;
                                app.result = None;
                                app.progress = 0.0;
                                app.elapsed = 0;
                                app.record_start = None;
                            }
                            app.capture_tab = (app.capture_tab + 1) % 3;
                        },
                        KeyCode::Up => {
                            if app.capture_tab==0 && app.selected_device>0 { app.selected_device-=1; }
                            if app.capture_tab==1 && app.selected_output>0 { app.selected_output-=1; }
                            if app.capture_tab==2 && app.selected_app>0 { app.selected_app-=1; }
                        },
                        KeyCode::Down => {
                            if app.capture_tab==0 && app.selected_device+1 < app.devices.len() { app.selected_device+=1; }
                            if app.capture_tab==1 && app.selected_output+1 < app.output_devices.len() { app.selected_output+=1; }
                            if app.capture_tab==2 && app.selected_app+1 < app.apps.len() { app.selected_app+=1; }
                        },
                        KeyCode::Char('c') => { app.log.clear(); app.push_log("Log cleared".to_string()); },
                        KeyCode::Char('g') | KeyCode::Char('G') | KeyCode::F(5) => {
                            app.apps = list_app_sessions();
                            if app.apps.is_empty() { app.apps = vec!["No active audio apps - play music then press g".into()]; }
                            app.app_peaks = vec![0.0; app.apps.len()];
                            app.output_devices = list_output_devices().into_iter().map(|d| d.name).collect();
                            if app.output_devices.is_empty() { app.output_devices = vec!["Default".into()]; }
                            app.devices = list_input_devices().unwrap_or_default().into_iter().map(|d| d.name).collect();
                            if app.devices.is_empty() { app.devices = vec!["Default".into()]; }
                            app.push_log(format!("Refreshed: {} outputs, {} inputs, {} apps", app.output_devices.len(), app.devices.len(), app.apps.len()));
                        },
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            if app.capture_tab == 2 {
                                app.push_log("Apps are dynamic — not saved (input/output only)".to_string());
                                continue;
                            }
                            let cur_input = app.devices.get(app.selected_device).cloned();
                            let cur_output = app.output_devices.get(app.selected_output).cloned();
                            app.config.default_input = cur_input.clone();
                            app.config.default_output = cur_output.clone();
                            app.config.default_tab = Some(app.capture_tab);
                            match app.config.save() {
                                Ok(_) => app.push_log(format!("Saved defaults: input={:?} output={:?} tab={} (press d to change, g to refresh)", cur_input, cur_output, app.capture_tab)),
                                Err(e) => app.push_log(format!("Failed to save: {}", e)),
                            }
                        },
                        KeyCode::Esc => { app.status = Status::Idle; app.error=None; app.result=None; app.record_start=None; app.progress=0.0; app.elapsed=0; },
                        _ => {}
                    }
                }
            },
            _ = ticker.tick() => {
                if app.status == Status::Recording {
                    if let Some(start) = app.record_start {
                        let elapsed = start.elapsed().as_secs();
                        let total = app.config.record_duration_secs;
                        app.elapsed = elapsed.min(total);
                        app.progress = (elapsed as f32 / total as f32).clamp(0.0, 1.0);
                        if elapsed >= total {
                            app.status = Status::Identifying;
                        }
                    }
                }
                // Live wave for apps tab — updates every 100ms
                app.update_peaks();
                while let Ok(res) = rx.try_recv() {
                    match res {
                        Ok(song) => {
                            app.push_log(format!("{} - {}", song.artist, song.title));
                            app.result = Some(song);
                            app.status = Status::Done;
                            app.progress = 1.0;
                            app.record_start = None;
                            app.elapsed = app.config.record_duration_secs;
                        },
                        Err(e) => {
                            app.push_log(format!("Failed: {}", e));
                            app.error = Some(e);
                            app.status = Status::Error;
                            app.progress = 1.0;
                            app.record_start = None;
                            app.elapsed = app.config.record_duration_secs;
                        }
                    }
                }
                terminal.draw(|f| super::ui::draw(f, &app))?;
            }
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
