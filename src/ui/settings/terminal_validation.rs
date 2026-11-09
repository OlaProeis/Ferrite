//! Non-blocking validators for terminal-related settings fields (ISS-19).

/// File extensions accepted for custom terminal notification sounds.
pub const TERMINAL_SOUND_EXTENSIONS: &[&str] = &[
    "wav", "wave", "ogg", "oga", "mp3", "aiff", "aif", "flac", "m4a",
];

/// First invalid regex line in multiline prompt patterns text (`None` = OK).
pub fn validate_prompt_patterns(text: &str) -> Option<PromptPatternError> {
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        if let Err(e) = regex::Regex::new(line) {
            return Some(PromptPatternError {
                line: index + 1,
                message: e.to_string(),
            });
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptPatternError {
    pub line: usize,
    pub message: String,
}

/// Sound path issues when a non-empty path is set (`None` = OK or empty path).
pub fn validate_terminal_sound_path(path: &str) -> Option<SoundPathError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path_obj = std::path::Path::new(trimmed);
    if !path_obj.exists() {
        return Some(SoundPathError::NotFound);
    }
    let ext = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext {
        Some(ext)
            if TERMINAL_SOUND_EXTENSIONS
                .iter()
                .any(|allowed| *allowed == ext) =>
        {
            None
        }
        Some(_) => Some(SoundPathError::UnsupportedExtension),
        None => Some(SoundPathError::UnsupportedExtension),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPathError {
    NotFound,
    UnsupportedExtension,
}

/// Startup command issues (`None` = OK). Empty command is allowed.
pub fn validate_terminal_startup_command(command: &str) -> Option<StartupCommandError> {
    if command.is_empty() {
        return None;
    }
    if command.chars().all(char::is_whitespace) {
        return Some(StartupCommandError::WhitespaceOnly);
    }
    if quotes_unbalanced(command) {
        return Some(StartupCommandError::UnbalancedQuotes);
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupCommandError {
    WhitespaceOnly,
    UnbalancedQuotes,
}

/// Shell-style single/double quote balance (backslash escapes inside double quotes).
fn quotes_unbalanced(s: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    for c in s.chars() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_double {
            escape = true;
            continue;
        }
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ => {}
        }
    }
    in_single || in_double
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_patterns_valid_lines() {
        assert!(validate_prompt_patterns("").is_none());
        assert!(validate_prompt_patterns("^\\w+@\\w+:\n$ ").is_none());
    }

    #[test]
    fn prompt_patterns_invalid_regex() {
        let err = validate_prompt_patterns("valid\n([invalid").expect("expected error");
        assert_eq!(err.line, 2);
        assert!(!err.message.is_empty());
    }

    #[test]
    fn sound_path_missing_file() {
        assert_eq!(
            validate_terminal_sound_path("/nonexistent/path/alert.wav"),
            Some(SoundPathError::NotFound)
        );
    }

    #[test]
    fn sound_path_bad_extension() {
        let dir = std::env::temp_dir();
        let path = dir.join("ferrite_terminal_validation_test.txt");
        std::fs::write(&path, b"x").expect("write temp file");
        let result = validate_terminal_sound_path(path.to_str().expect("utf8 path"));
        std::fs::remove_file(&path).ok();
        assert_eq!(result, Some(SoundPathError::UnsupportedExtension));
    }

    #[test]
    fn sound_path_empty_ok() {
        assert!(validate_terminal_sound_path("").is_none());
        assert!(validate_terminal_sound_path("   ").is_none());
    }

    #[test]
    fn startup_command_whitespace_only() {
        assert_eq!(
            validate_terminal_startup_command("   \t  "),
            Some(StartupCommandError::WhitespaceOnly)
        );
        assert!(validate_terminal_startup_command("").is_none());
    }

    #[test]
    fn startup_command_unbalanced_quotes() {
        assert_eq!(
            validate_terminal_startup_command(r#"echo "hello"#),
            Some(StartupCommandError::UnbalancedQuotes)
        );
        assert_eq!(
            validate_terminal_startup_command("echo 'hi"),
            Some(StartupCommandError::UnbalancedQuotes)
        );
        assert!(validate_terminal_startup_command(r#"echo "ok""#).is_none());
    }
}
