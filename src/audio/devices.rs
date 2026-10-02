//! سرد الأجهزة والتطبيقات
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct AudioDeviceInfo {
    pub name: String,
    #[allow(dead_code)]
    pub is_loopback: bool,
    #[allow(dead_code)]
    pub is_monitor: bool,
}

pub fn list_input_devices() -> Result<Vec<AudioDeviceInfo>> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let mut out = Vec::new();
    for dev in host.input_devices()? {
        if let Ok(name) = dev.name() {
            out.push(AudioDeviceInfo { name, is_loopback: false, is_monitor: false });
        }
    }
    Ok(out)
}

pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
    #[cfg(target_os = "windows")]
    {
        use wasapi::{Direction, DeviceCollection};
        let _ = wasapi::initialize_mta();
        if let Ok(coll) = DeviceCollection::new(&Direction::Render) {
            let mut out = Vec::new();
            if let Ok(n) = coll.get_nbr_devices() {
                for i in 0..n {
                    if let Ok(dev) = coll.get_device_at_index(i) {
                        if let Ok(name) = dev.get_friendlyname() {
                            out.push(AudioDeviceInfo { name, is_loopback: false, is_monitor: false });
                        }
                    }
                }
            }
            if !out.is_empty() { return out; }
        }
    }
    // Fallback to cpal output devices
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let mut out = Vec::new();
    if let Ok(devs) = host.output_devices() {
        for dev in devs {
            if let Ok(name) = dev.name() {
                out.push(AudioDeviceInfo { name, is_loopback: false, is_monitor: false });
            }
        }
    }
    if out.is_empty() { out.push(AudioDeviceInfo { name: "Default Output".into(), is_loopback: true, is_monitor: true }); }
    out
}

#[allow(dead_code)]
pub fn list_loopback_devices() -> Vec<AudioDeviceInfo> {
    // نجمع كل الأجهزة ونميز الـ monitor
    let mut v = list_input_devices().unwrap_or_default();
    // على Windows: نضيف أجهزة الـ loopback الافتراضية
    #[cfg(target_os = "windows")]
    {
        // WASAPI loopback devices تظهر كـ render devices
        
        if let Ok(host) = std::panic::catch_unwind(|| cpal::default_host()) {
            let _ = host;
        }
        // سنضيف عنصر وهمي يمثل System Audio
        v.push(AudioDeviceInfo { name: "System Audio (WASAPI Loopback)".into(), is_loopback: true, is_monitor: true });
    }
    v
}

/// سرد جلسات الصوت للتطبيقات (Windows فقط عبر WASAPI Audio Sessions)
#[cfg(target_os = "windows")]
pub fn list_app_sessions() -> Vec<String> {
    // نستخدم wasapi لسرد العمليات التي تصدر صوتاً
    // هذا يعتمد على WASAPI session enumeration
    // للتبسيط نعيد قائمة فارغة إذا فشل، والتطبيق سيعمل بـ loopback عام
    match try_list_sessions_wasapi() {
        Ok(v) => v,
        Err(_) => vec![],
    }
}

#[cfg(not(target_os = "windows"))]
pub fn list_app_sessions() -> Vec<String> {
    // على Linux: يمكن سرد عملاء PulseAudio عبر `pactl list clients` لكن نتركه فارغ للـ MVP
    vec![]
}

#[cfg(target_os = "windows")]
fn try_list_sessions_wasapi() -> anyhow::Result<Vec<String>> {
    use windows::Win32::Media::Audio::{MMDeviceEnumerator, IMMDeviceEnumerator, eRender, eConsole, IAudioSessionManager2, IAudioSessionControl2, AudioSessionStateActive};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, COINIT_MULTITHREADED, CoInitializeEx};
    use sysinfo::{System, Pid};
    use windows::core::Interface;
    unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }

    let mut out = Vec::new();
    let enumerator: IMMDeviceEnumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };
    let mgr: IAudioSessionManager2 = unsafe { device.Activate(CLSCTX_ALL, None)? };
    let session_enum = unsafe { mgr.GetSessionEnumerator()? };
    let count = unsafe { session_enum.GetCount()? };
    let mut sys = System::new_all();
    sys.refresh_all();

    for i in 0..count {
        let ctrl = unsafe { session_enum.GetSession(i)? };
        let ctrl2: IAudioSessionControl2 = match ctrl.cast() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let pid = unsafe { ctrl2.GetProcessId().unwrap_or(0) };
        if pid == 0 { continue; } // System sounds
        let state = unsafe { ctrl2.GetState().unwrap_or(AudioSessionStateActive) };
        if state != AudioSessionStateActive { continue; } // Only show apps currently playing
        let proc_name = sys.process(Pid::from(pid as usize))
            .map(|p| p.name().to_string_lossy().to_string())
            .unwrap_or_else(|| format!("PID {}", pid));
        let display = unsafe { ctrl2.GetDisplayName().ok().and_then(|pw| pw.to_string().ok()).unwrap_or_default() };
        let label = if display.is_empty() {
            format!("{} (PID {})", proc_name, pid)
        } else {
            format!("{} (PID {}) - {}", proc_name, pid, display)
        };
        if !out.contains(&label) {
            out.push(label);
        }
    }

    if out.is_empty() {
        // Fallback to sysinfo active processes that have audio capability (heuristic)
        for (pid, proc) in sys.processes() {
            let name = proc.name().to_string_lossy().to_lowercase();
            if ["chrome", "firefox", "msedge", "spotify", "vlc", "youtube", "discord", "steam"].iter().any(|k| name.contains(k)) {
                out.push(format!("{} (PID {})", proc.name().to_string_lossy(), pid.as_u32()));
                if out.len() >= 8 { break; }
            }
        }
    }
    if out.is_empty() {
        out = vec!["chrome.exe".into(), "firefox.exe".into(), "spotify.exe".into()];
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
pub fn get_peak_for_pid(target_pid: u32) -> f32 {
    use windows::Win32::Media::Audio::{MMDeviceEnumerator, IMMDeviceEnumerator, eRender, eConsole, IAudioSessionManager2};
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, COINIT_MULTITHREADED, CoInitializeEx};
    use windows::core::Interface;
    unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
    let enumerator: Result<IMMDeviceEnumerator, _> = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) };
    let enumerator = match enumerator { Ok(e) => e, Err(_) => return 0.0 };
    let device = match unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) } { Ok(d) => d, Err(_) => return 0.0 };
    let mgr: Result<IAudioSessionManager2, _> = unsafe { device.Activate(CLSCTX_ALL, None) };
    let mgr = match mgr { Ok(m) => m, Err(_) => return 0.0 };
    let session_enum = match unsafe { mgr.GetSessionEnumerator() } { Ok(s) => s, Err(_) => return 0.0 };
    let count = unsafe { session_enum.GetCount().unwrap_or(0) };
    let mut max_peak: f32 = 0.0;
    let mut found = false;
    for i in 0..count {
        let ctrl = match unsafe { session_enum.GetSession(i) } { Ok(c) => c, Err(_) => continue };
        use windows::Win32::Media::Audio::IAudioSessionControl2;
        let ctrl2: Result<IAudioSessionControl2, _> = ctrl.cast();
        let ctrl2 = match ctrl2 { Ok(c) => c, Err(_) => continue };
        let pid = unsafe { ctrl2.GetProcessId().unwrap_or(0) };
        if pid != target_pid { continue; }
        found = true;
        // Try per-session meter (IAudioMeterInformation on the session)
        let meter: Result<IAudioMeterInformation, _> = ctrl.cast();
        let meter2: Result<IAudioMeterInformation, _> = ctrl2.cast();
        let peak = if let Ok(m) = meter { unsafe { m.GetPeakValue().unwrap_or(0.0) } } else { 0.0 };
        let peak2 = if let Ok(m) = meter2 { unsafe { m.GetPeakValue().unwrap_or(0.0) } } else { 0.0 };
        let p = peak.max(peak2);
        if p > max_peak { max_peak = p; }
    }
    if found { max_peak } else { 0.0 }
}
