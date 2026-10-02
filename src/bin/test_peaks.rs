use m_ust::audio::{list_app_sessions, session_peaks, parse_pid};

fn main() {
    let apps = list_app_sessions();
    let peaks = session_peaks();
    println!("Found {} apps", apps.len());
    for app in &apps {
        if let Some(pid) = parse_pid(app) {
            let peak = peaks.get(&pid).copied().unwrap_or(0.0);
            println!("{} -> peak {:.3} {}", app, peak, if peak>0.02 {"WAVE"} else {""});
        } else {
            println!("{} -> no pid", app);
        }
    }
}
