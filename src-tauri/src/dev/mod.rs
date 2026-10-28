//! Debug-build conveniences for driving the app from scripts/dev without disturbing the user: window
//! placement, background launches, running JavaScript, and the fake Jira (`fake.rs`). Only compiled into debug builds
//! (`#[cfg(debug_assertions)]` on the module and its call sites).

mod fake;

pub use fake::{drift, start_fake_jira};

/// `TASKBOARD_WINDOW_FRAME="x,y,width,height"` (logical points) places the main window.
pub fn window_frame() -> Option<(f64, f64, f64, f64)> {
    parse_frame(&std::env::var("TASKBOARD_WINDOW_FRAME").ok()?)
}

/// `TASKBOARD_BACKGROUND=1` keeps the windows from taking focus.
pub fn background() -> bool {
    std::env::var("TASKBOARD_BACKGROUND").is_ok_and(|v| v == "1")
}

/// Runs JavaScript sent by `scripts/dev/js.sh` in the main window, or in another one named by a first
/// line `@<label>`.
pub fn eval(app: &tauri::AppHandle, js: &str) {
    use tauri::Manager;
    let (label, js) = match js.strip_prefix('@').and_then(|rest| rest.split_once('\n')) {
        Some((label, rest)) => (label.trim(), rest),
        None => (crate::window::MAIN, js),
    };
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    if let Err(e) = window.eval(js) {
        log::warn!("Couldn't evaluate script from scripts/dev: {e}");
    }
}

pub(crate) fn parse_frame(raw: &str) -> Option<(f64, f64, f64, f64)> {
    let v: Vec<f64> = raw
        .split(',')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    if !v.iter().all(|n| n.is_finite()) {
        return None;
    }
    match v[..] {
        [x, y, w, h] if w > 0.0 && h > 0.0 => Some((x, y, w, h)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frames_and_rejects_garbage() {
        assert_eq!(
            parse_frame("-3000, -537, 1400,900"),
            Some((-3000.0, -537.0, 1400.0, 900.0))
        );
        for raw in ["1,2,3", "1,2,0,4", "a,b,c,d", "1,2,3,4,5", ""] {
            assert_eq!(parse_frame(raw), None, "{raw}");
        }
    }

    #[test]
    fn non_finite_or_non_positive_sizes_are_rejected() {
        for raw in ["NaN,0,10,10", "0,0,inf,10", "0,0,-5,10", "0,0,10,0"] {
            assert_eq!(parse_frame(raw), None, "{raw}");
        }
    }

    #[test]
    fn whitespace_around_numbers_is_ignored() {
        assert_eq!(parse_frame("  1 , 2 ,  3,4 "), Some((1.0, 2.0, 3.0, 4.0)));
    }
}
