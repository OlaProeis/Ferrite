//! Configuration module for Ferrite
//!
//! This module handles user preferences and application settings,
//! including serialization/deserialization to/from JSON and
//! persistent storage to platform-specific directories.
//!
//! The session submodule provides crash-safe session state persistence
//! for restoring tabs and editor state after crashes or restarts.
//!
//! The snippets submodule provides user-defined text expansions
//! with built-in date/time snippets.

mod persistence;
mod session;
mod settings;
mod snippets;

pub use persistence::*;
pub use session::*;
pub use settings::*;
pub use snippets::*;

/// Run `body` with [`DATA_DIR_ENV_VAR`] (`FERRITE_DATA_DIR`) pointed at a fresh
/// temporary directory so config and session tests do not touch the real user
/// config directory.
///
/// A process-global mutex serializes callers. The previous value of the
/// variable (or its absence) is restored when `body` returns, including if it
/// panics. The temporary directory is removed after that restore.
///
/// Do not call this helper reentrantly from inside `body`; the mutex is not
/// reentrant.
///
/// # Examples
///
/// ```ignore
/// use ferrite::config::{get_config_dir, with_temp_data_dir};
///
/// with_temp_data_dir(|dir| {
///     let config_dir = get_config_dir().unwrap();
///     assert!(config_dir.starts_with(dir));
/// });
/// ```
#[cfg(test)]
pub(crate) fn with_temp_data_dir<R>(body: impl FnOnce(&std::path::Path) -> R) -> R {
    let _guard = TEMP_DATA_DIR_LOCK
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    with_temp_data_dir_locked(body)
}

/// Serializes every mutation of `FERRITE_DATA_DIR` across test threads.
#[cfg(test)]
static TEMP_DATA_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Body of [`with_temp_data_dir`]; the caller must already hold
/// [`TEMP_DATA_DIR_LOCK`].
#[cfg(test)]
fn with_temp_data_dir_locked<R>(body: impl FnOnce(&std::path::Path) -> R) -> R {
    struct RestoreEnv(Option<String>);

    impl Drop for RestoreEnv {
        fn drop(&mut self) {
            // SAFETY: `LOCK` is held for the whole helper, so this test thread
            // is the only one mutating `FERRITE_DATA_DIR`.
            match self.0.take() {
                Some(value) => unsafe { std::env::set_var(DATA_DIR_ENV_VAR, value) },
                None => unsafe { std::env::remove_var(DATA_DIR_ENV_VAR) },
            }
        }
    }

    let temp = tempfile::TempDir::new().expect("create temp data dir");
    let previous = std::env::var(DATA_DIR_ENV_VAR).ok();
    // SAFETY: same lock as `RestoreEnv`; no other test mutates this var until drop.
    unsafe { std::env::set_var(DATA_DIR_ENV_VAR, temp.path()) };
    let _restore = RestoreEnv(previous);
    body(temp.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_temp_data_dir_points_under_temp() {
        with_temp_data_dir(|dir| {
            let config_dir = get_config_dir().expect("config dir under temp");
            assert!(
                config_dir.starts_with(dir),
                "config dir {} is not under {}",
                config_dir.display(),
                dir.display()
            );
        });
    }

    #[test]
    fn with_temp_data_dir_sets_and_restores_env() {
        // Hold the lock ourselves so `before`/`after` cannot observe another
        // test's temporary value; call the unlocked body directly.
        let _guard = TEMP_DATA_DIR_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let before = std::env::var(DATA_DIR_ENV_VAR).ok();
        let mut seen = None;
        with_temp_data_dir_locked(|dir| {
            assert_eq!(
                std::env::var(DATA_DIR_ENV_VAR).ok().as_deref(),
                Some(dir.to_string_lossy().as_ref())
            );
            seen = Some(dir.to_path_buf());
        });
        assert!(seen.is_some());
        assert_eq!(std::env::var(DATA_DIR_ENV_VAR).ok(), before);
    }

    /// CI guard: every `get_config_dir()` call inside `#[cfg(test)]` modules
    /// and `tests/` must sit inside `with_temp_data_dir(...)`.
    #[test]
    fn audit_test_get_config_dir_uses_temp_data_dir() {
        let call = format!("{}{}", "get_config_dir", "(");
        let helper = format!("{}{}", "with_temp_data_dir", "(");
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut offenders = Vec::new();
        for rel in ["src", "tests"] {
            let dir = root.join(rel);
            if dir.is_dir() {
                collect_config_dir_offenders(&dir, &call, &helper, &mut offenders);
            }
        }
        assert!(
            offenders.is_empty(),
            "tests call get_config_dir() outside with_temp_data_dir:\n{}",
            offenders.join("\n")
        );
    }
}

#[cfg(test)]
fn collect_config_dir_offenders(
    dir: &std::path::Path,
    call: &str,
    helper: &str,
    offenders: &mut Vec<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_config_dir_offenders(&path, call, helper, offenders);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let stripped = strip_rust_comments_and_strings(&source);
        let in_tests_dir = path
            .components()
            .any(|component| component.as_os_str() == std::ffi::OsStr::new("tests"));
        let regions = test_code_regions(&stripped, in_tests_dir);
        for region in regions {
            for (name, body) in rust_functions(&region) {
                if function_calls_config_dir_outside_helper(&body, call, helper) {
                    offenders.push(format!("{}::{name}", path.display()));
                }
            }
        }
    }
}

#[cfg(test)]
fn test_code_regions(source: &str, entire_file: bool) -> Vec<String> {
    if entire_file {
        return vec![source.to_string()];
    }
    let mut regions = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !source.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if bytes[i] == b'#' && source[i..].starts_with("#[cfg(test)]") {
            let after = i + "#[cfg(test)]".len();
            let rest = source[after..].trim_start();
            if rest.starts_with("mod ") {
                if let Some(rel_brace) = rest.find('{') {
                    let abs = after + (source[after..].len() - rest.len()) + rel_brace;
                    // `abs` points at `{`; the loop will increment depth when it sees it.
                    let start = abs + 1;
                    // Record region once the module closes.
                    let mut d = 0i32;
                    let mut j = abs;
                    while j < bytes.len() {
                        if !source.is_char_boundary(j) {
                            j += 1;
                            continue;
                        }
                        if bytes[j] == b'{' {
                            d += 1;
                        } else if bytes[j] == b'}' {
                            d -= 1;
                            if d == 0 {
                                regions.push(source[start..j].to_string());
                                break;
                            }
                        }
                        j += 1;
                    }
                }
            }
        }
        i += 1;
    }
    regions
}

#[cfg(test)]
fn rust_functions(region: &str) -> Vec<(String, String)> {
    let mut fns = Vec::new();
    let bytes = region.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if !region.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let at_fn = region[i..].starts_with("fn ")
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
        if !at_fn {
            i += 1;
            continue;
        }
        let name_start = i + 3;
        let name_end = region[name_start..]
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .map(|end| name_start + end)
            .unwrap_or(region.len());
        let name = region[name_start..name_end].to_string();
        if let Some(brace_rel) = region[name_end..].find('{') {
            let brace = name_end + brace_rel;
            let mut d = 0i32;
            let mut j = brace;
            while j < bytes.len() {
                if !region.is_char_boundary(j) {
                    j += 1;
                    continue;
                }
                if bytes[j] == b'{' {
                    d += 1;
                } else if bytes[j] == b'}' {
                    d -= 1;
                    if d == 0 {
                        fns.push((name, region[brace..=j].to_string()));
                        i = j + 1;
                        break;
                    }
                }
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
        } else {
            break;
        }
    }
    fns
}

#[cfg(test)]
fn function_calls_config_dir_outside_helper(body: &str, call: &str, helper: &str) -> bool {
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut frames: Vec<bool> = Vec::new();
    while i < bytes.len() {
        if !body.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if body[i..].starts_with(helper) {
            let after = i + helper.len() - 1;
            if after < bytes.len() && bytes[after] == b'(' {
                let prev_ok =
                    i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
                if prev_ok {
                    frames.push(true);
                    i = after + 1;
                    continue;
                }
            }
        }
        if body[i..].starts_with(call) {
            let prev_ok = i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            if prev_ok && !frames.iter().any(|is_helper| *is_helper) {
                return true;
            }
        }
        match bytes[i] {
            b'(' => frames.push(false),
            b')' => {
                frames.pop();
            }
            _ => {}
        }
        i += 1;
    }
    false
}

#[cfg(test)]
fn strip_rust_comments_and_strings(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(chars.len());
            continue;
        }
        if chars[i] == 'r' && i + 1 < chars.len() && (chars[i + 1] == '#' || chars[i + 1] == '"') {
            let mut hashes = 0;
            let mut j = i + 1;
            while j < chars.len() && chars[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if j < chars.len() && chars[j] == '"' {
                j += 1;
                let end = "#".repeat(hashes) + "\"";
                let rest: String = chars[j..].iter().collect();
                if let Some(pos) = rest.find(&end) {
                    let consumed = chars[j..j + pos].iter().filter(|c| **c == '\n').count();
                    for _ in 0..consumed {
                        out.push('\n');
                    }
                    i = j + pos + end.len();
                    continue;
                }
            }
        }
        if chars[i] == '"' {
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    i += 1;
                    break;
                }
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}
