//! Turning a catalogue executable name into a path we are willing to create.
//!
//! The input is remote data: Discord's detectable-applications payload, and any
//! proxy in front of it. The upstream implementation *dropped* `..` components
//! and invalid characters, which silently turns `a/../../b.exe` into `b.exe`.
//! Here a suspicious name is rejected outright, so a malformed entry is skipped
//! and logged instead of being rewritten into something plausible.

use std::path::{Component, Path, PathBuf};

use crate::error::PathError;

/// Target platform rules; Windows also rejects device names and its own
/// forbidden characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Unix,
}

impl Platform {
    /// The platform this binary was built for.
    #[must_use]
    pub const fn host() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }
}

/// Names Windows refuses no matter the extension (`CON.exe` is still `CON`).
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Characters Windows forbids in a file name.
fn is_forbidden_on_windows(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control()
}

/// Returns a relative path that is safe to join onto our own temporary root.
///
/// Rules, in order: trim the leading `>` Discord uses as a marker, normalise
/// separators, reject absolute paths, reject `.`/`..`, reject reserved Windows
/// device names, strip characters the target platform forbids, and reject a
/// name that ends up empty.
pub fn sanitize_executable_rel_path(raw: &str, platform: Platform) -> Result<PathBuf, PathError> {
    let trimmed = raw.trim().trim_start_matches('>').trim();
    if trimmed.is_empty() {
        return Err(PathError::Empty);
    }

    // Any drive letter, UNC prefix or leading separator means absolute.
    let normalized = trimmed.replace('\\', "/");
    if normalized.starts_with('/') || has_drive_prefix(&normalized) {
        return Err(PathError::Absolute(trimmed.to_owned()));
    }

    let mut cleaned = PathBuf::new();
    for component in Path::new(&normalized).components() {
        let Component::Normal(part) = component else {
            // CurDir / ParentDir / RootDir / Prefix: refuse instead of rewriting.
            return Err(match component {
                Component::CurDir | Component::ParentDir => {
                    PathError::Traversal(trimmed.to_owned())
                }
                _ => PathError::Absolute(trimmed.to_owned()),
            });
        };

        let mut part = part.to_string_lossy().into_owned();
        if platform == Platform::Windows {
            part = part
                .chars()
                .filter(|c| !is_forbidden_on_windows(*c))
                .collect();
            let trimmed_len = part.trim_end_matches(['.', ' ']).len();
            part.truncate(trimmed_len);
        }

        if part.is_empty() {
            return Err(PathError::NoUsableComponent(trimmed.to_owned()));
        }
        // A leading dot is a hidden file, not a game executable; it is also what
        // a name made only of stripped characters collapses into (e.g. "?.exe").
        if part.starts_with('.') {
            return Err(PathError::NoUsableComponent(trimmed.to_owned()));
        }
        if platform == Platform::Windows && is_reserved_device(&part) {
            return Err(PathError::ReservedDeviceName(part));
        }

        cleaned.push(part);
    }

    if cleaned.as_os_str().is_empty() {
        return Err(PathError::NoUsableComponent(trimmed.to_owned()));
    }

    // Windows cannot detect a game without `.exe`, and a bare name is what the
    // catalogue usually gives us.
    if platform == Platform::Windows && cleaned.extension().is_none() {
        cleaned.set_extension("exe");
    }

    Ok(cleaned)
}

fn has_drive_prefix(normalized: &str) -> bool {
    let bytes = normalized.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

fn is_reserved_device(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part).trim();
    RESERVED
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(stem))
}
