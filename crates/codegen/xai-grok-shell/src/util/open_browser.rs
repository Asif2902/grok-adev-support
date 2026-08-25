//! Best-effort "open this URL in a browser" without panicking on Android.
//!
//! `webbrowser::open` on `target_os = "android"` goes through `ndk-context`
//! JNI (`android.content.Context`). This CLI is a bare shell process
//! (Termux / Mobile IDE), so that context is never initialized and
//! `ndk_context::android_context()` panics. Our release profile uses
//! `panic = "abort"`, so device-code login used to print the URL and then
//! abort:
//!
//! ```text
//! To sign in, open this URL in your browser:
//!   https://accounts.x.ai/oauth2/device?user_code=...
//! thread 'tokio-rt-worker' panicked at ndk-context-...:
//! android context was not initialized
//! Aborted
//! ```
//!
//! On Android we never call `webbrowser::open`. We try `termux-open-url`,
//! `xdg-open`, then `am start` (Android activity manager) and treat any
//! missing-binary / non-zero exit as "could not open automatically".

#[cfg(target_os = "android")]
use std::process::Command;

/// Try to open `url` in a browser. Returns `true` only when a launcher
/// reported success. Never panics on Android.
pub(crate) fn try_open_browser(url: &str) -> bool {
    #[cfg(target_os = "android")]
    {
        try_open_browser_android(url)
    }
    #[cfg(not(target_os = "android"))]
    {
        webbrowser::open(url).is_ok()
    }
}

#[cfg(target_os = "android")]
fn try_open_browser_android(url: &str) -> bool {
    for argv in android_open_argv(url) {
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..]);
        match cmd.status() {
            Ok(status) if status.success() => return true,
            _ => continue,
        }
    }
    false
}

/// argv lists tried in order on Android. Extracted so tests can lock the
/// sequence without spawning processes.
#[cfg(any(target_os = "android", test))]
fn android_open_argv(url: &str) -> Vec<Vec<String>> {
    vec![
        vec!["termux-open-url".to_string(), url.to_string()],
        vec!["xdg-open".to_string(), url.to_string()],
        vec![
            "am".to_string(),
            "start".to_string(),
            "-a".to_string(),
            "android.intent.action.VIEW".to_string(),
            "-d".to_string(),
            url.to_string(),
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::android_open_argv;

    #[test]
    fn android_open_argv_never_uses_webbrowser_and_passes_the_url() {
        let url = "https://accounts.x.ai/oauth2/device?user_code=ABCD-EFGH";
        let argv = android_open_argv(url);
        assert_eq!(argv[0], ["termux-open-url", url]);
        assert_eq!(argv[1], ["xdg-open", url]);
        assert_eq!(
            argv[2],
            ["am", "start", "-a", "android.intent.action.VIEW", "-d", url]
        );
        for cmd in &argv {
            assert_ne!(cmd[0], "webbrowser");
            assert_eq!(cmd.last().map(String::as_str), Some(url));
        }
    }
}
