//! Reading Discord's detectable-applications payload without trusting it.
//!
//! The shape mirrors `GET /api/v9/applications/detectable`. Everything is
//! optional except id and name, because a catalogue entry with a missing or
//! renamed field must degrade to "no executable for this platform" rather than
//! make the whole load fail.

use questline_core::{sanitize_executable_rel_path, Game, GameId, Platform, SpoofMethod};
use serde::{Deserialize, Serialize};

/// One `executables[]` element.
#[derive(Debug, Clone, Deserialize)]
pub struct RawExecutable {
    pub name: String,
    pub os: String,
    #[serde(default)]
    pub arguments: Option<String>,
    #[serde(default)]
    pub is_launcher: Option<bool>,
}

/// One application from the catalogue.
#[derive(Debug, Clone, Deserialize)]
pub struct RawEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub executables: Vec<RawExecutable>,
}

/// A catalogue entry we refused, with the reason shown to the user.
#[derive(Debug, Clone, Serialize)]
pub struct Rejected {
    pub name: String,
    pub reason: String,
}

/// Result of parsing a catalogue for one platform.
#[derive(Debug, Default)]
pub struct Catalogue {
    pub games: Vec<Game>,
    pub rejected: Vec<Rejected>,
    /// Entries with no executable for this platform (RPC-only titles).
    pub without_executable: usize,
}

/// Picks the executable for `platform` and turns the entry into a [`Game`].
///
/// Preference order: a non-launcher executable the catalogue lists first, then
/// any executable for the platform. A launcher shortcut is what upstream
/// avoided too — Discord does not count the launcher process as the game.
#[must_use]
pub fn parse_entries(entries: &[RawEntry], platform: Platform) -> Catalogue {
    let os = match platform {
        Platform::Windows => "win32",
        Platform::Unix => "linux",
    };

    let mut catalogue = Catalogue::default();

    for entry in entries {
        let Some(candidate) = pick_executable(&entry.executables, os) else {
            catalogue.without_executable += 1;
            continue;
        };

        let Ok(id) = GameId::new(entry.id.clone()) else {
            catalogue.rejected.push(Rejected {
                name: entry.name.clone(),
                reason: format!("invalid application id {:?}", entry.id),
            });
            continue;
        };

        let path = match sanitize_executable_rel_path(&candidate.name, platform) {
            Ok(path) => path,
            Err(err) => {
                catalogue.rejected.push(Rejected {
                    name: entry.name.clone(),
                    reason: err.to_string(),
                });
                continue;
            }
        };

        let display = path.to_string_lossy().replace('\\', "/");
        match Game::new(id, entry.name.clone(), Some(display), SpoofMethod::Process) {
            Ok(game) => catalogue.games.push(Game {
                arguments: candidate.arguments.clone(),
                ..game
            }),
            Err(err) => catalogue.rejected.push(Rejected {
                name: entry.name.clone(),
                reason: err.to_string(),
            }),
        }
    }

    catalogue
}

fn pick_executable<'a>(executables: &'a [RawExecutable], os: &str) -> Option<&'a RawExecutable> {
    let mut fallback = None;
    for executable in executables.iter().filter(|e| e.os.eq_ignore_ascii_case(os)) {
        if executable.is_launcher == Some(true) {
            fallback.get_or_insert(executable);
        } else {
            return Some(executable);
        }
    }
    fallback
}

/// Reads and parses a catalogue file, keeping parse errors explicit.
pub fn load_file(path: &std::path::Path) -> Result<Vec<RawEntry>, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    serde_json::from_str(&data).map_err(|err| format!("cannot parse {}: {err}", path.display()))
}
