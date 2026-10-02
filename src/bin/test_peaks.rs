use m_ust::audio::{list_app_sessions, get_peak_for_pid};

fn parse_pid(s: &str) -> Option<u32> {
    if let Some(start) = s.find("(PID ") {
        let rest = &s[start+5..];
        if let Some(end) = rest.find(')') {
            return rest[..end].trim().parse().ok();
        }
    }
    None
}

fn main() {
    let apps = list_app_sessions();
    println!("Found {} apps", apps.len());
    for app in &apps {
        if let Some(pid) = parse_pid(app) {
            let peak = get_peak_for_pid(pid);
            println!("{} -> peak {:.3} {}", app, peak, if peak>0.02 {"WAVE"} else {""});
        } else {
            println!("{} -> no pid", app);
        }
    }
}
