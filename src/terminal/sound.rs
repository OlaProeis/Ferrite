//! Terminal sound notification module.
//!
//! This module provides simple sound notification functionality
//! for terminal prompt detection.

/// Play a notification sound.
///
/// If `custom_file` is Some and the file exists, attempts to play that file.
/// Otherwise, falls back to a system beep.
pub fn play_notification(custom_file: Option<&str>) {
    // Try custom sound file first
    if let Some(path) = custom_file {
        if std::path::Path::new(path).exists() {
            // Try to play the custom sound file
            if play_sound_file(path) {
                return;
            }
        }
    }

    // Fall back to system beep
    play_system_beep();
}

/// Play a sound file (best effort, cross-platform).
fn play_sound_file(path: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // Use Windows Media Player or PowerShell to play sound
        let result = std::process::Command::new("powershell.exe")
            .args(&[
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &format!(
                    "(New-Object Media.SoundPlayer '{}').PlaySync()",
                    path.replace("'", "''")
                ),
            ])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
        return result.is_ok();
    }

    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("afplay").arg(path).spawn();
        return result.is_ok();
    }

    #[cfg(target_os = "linux")]
    {
        // Try aplay (ALSA) first, then paplay (PulseAudio)
        if std::process::Command::new("aplay")
            .arg(path)
            .spawn()
            .is_ok()
        {
            return true;
        }
        return std::process::Command::new("paplay")
            .arg(path)
            .spawn()
            .is_ok();
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

/// Play a system beep sound.
fn play_system_beep() {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // Use PowerShell to play system beep
        let _ = std::process::Command::new("powershell.exe")
            .args(&[
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "[console]::beep(800, 200)",
            ])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
    }

    #[cfg(target_os = "macos")]
    {
        // macOS system sound
        let _ = std::process::Command::new("afplay")
            .arg("/System/Library/Sounds/Tink.aiff")
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        // Try multiple methods for Linux
        // First try the console bell
        let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x07");
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // Also try paplay with a system sound
        let _ = std::process::Command::new("paplay")
            .arg("/usr/share/sounds/freedesktop/stereo/complete.oga")
            .spawn();
    }
}

/// Sound notification manager that prevents rapid-fire sounds.
pub struct SoundNotifier {
    /// Minimum time between sounds in milliseconds
    cooldown_ms: u64,
    /// Last sound played time
    last_sound_time: std::time::Instant,
    /// Custom sound file path
    custom_sound_file: Option<String>,
    /// Whether notifications are enabled
    enabled: bool,
    /// Flag to track if we're waiting (to only play once per prompt)
    was_waiting: bool,
}

impl SoundNotifier {
    /// Create a new sound notifier.
    pub fn new() -> Self {
        Self {
            cooldown_ms: 500, // Minimum 500ms between sounds
            last_sound_time: std::time::Instant::now() - std::time::Duration::from_secs(10),
            custom_sound_file: None,
            enabled: false,
            was_waiting: false,
        }
    }

    /// Update settings.
    pub fn update_settings(&mut self, enabled: bool, custom_file: Option<String>) {
        self.enabled = enabled;
        self.custom_sound_file = custom_file;
    }

    /// Check if terminal transitioned to waiting state and play sound if appropriate.
    ///
    /// Returns true if a sound was played.
    pub fn check_and_notify(&mut self, is_waiting: bool) -> bool {
        if !self.enabled {
            self.was_waiting = is_waiting;
            return false;
        }

        // Only play sound on transition from not-waiting to waiting
        let should_play = is_waiting && !self.was_waiting;
        self.was_waiting = is_waiting;

        if !should_play {
            return false;
        }

        // Check cooldown
        if self.last_sound_time.elapsed().as_millis() < self.cooldown_ms as u128 {
            return false;
        }

        // Play sound
        self.last_sound_time = std::time::Instant::now();
        play_notification(self.custom_sound_file.as_deref());
        true
    }
}

impl Default for SoundNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_notifier_creation() {
        let notifier = SoundNotifier::new();
        assert!(!notifier.enabled);
        assert!(notifier.custom_sound_file.is_none());
    }

    #[test]
    fn test_sound_notifier_settings() {
        let mut notifier = SoundNotifier::new();
        notifier.update_settings(true, Some("test.wav".to_string()));
        assert!(notifier.enabled);
        assert_eq!(notifier.custom_sound_file, Some("test.wav".to_string()));
    }

    #[test]
    fn test_sound_notifier_disabled() {
        let mut notifier = SoundNotifier::new();
        notifier.update_settings(false, None);

        // Should not play when disabled
        assert!(!notifier.check_and_notify(true));
    }

    #[test]
    fn test_sound_notifier_transition() {
        let mut notifier = SoundNotifier::new();
        notifier.update_settings(true, None);

        // First transition to waiting should trigger
        // (We can't actually test the sound plays, but we can test the logic)
        let result = notifier.check_and_notify(true);
        assert!(result);

        // Staying in waiting should not trigger again
        let result = notifier.check_and_notify(true);
        assert!(!result);

        // Transition to not waiting
        let result = notifier.check_and_notify(false);
        assert!(!result);
    }
}
