#![allow(clippy::expect_used, clippy::unwrap_used)]
use std::time::Duration;

use questline_core::{DomainError, Game, GameId, RunMode, RunSpec, SpoofMethod};

#[test]
fn game_ids_must_be_numeric() {
    assert!(GameId::new("123").is_ok());
    assert!(GameId::new(" 456 ").is_ok());
    assert_eq!(GameId::new(""), Err(DomainError::EmptyGameId));
    assert_eq!(
        GameId::new("12ab"),
        Err(DomainError::NonNumericGameId("12ab".to_owned()))
    );
    assert!(matches!(
        GameId::new("1); DROP TABLE"),
        Err(DomainError::NonNumericGameId(_))
    ));
}

#[test]
fn game_id_round_trips_through_json() {
    let id = GameId::new("1084537").expect("id");
    let json = serde_json::to_string(&id).expect("serialise");
    assert_eq!(json, "\"1084537\"");
    let back: GameId = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, id);
}

#[test]
fn deserialising_a_bad_game_id_fails() {
    let bad: Result<GameId, _> = serde_json::from_str("\"not-a-number\"");
    assert!(bad.is_err(), "catalogue data must not slip past validation");
}

#[test]
fn a_game_needs_a_name() {
    let id = GameId::new("1").expect("id");
    assert_eq!(
        Game::new(id, "   ", Some("a.exe".to_owned()), SpoofMethod::Process),
        Err(DomainError::EmptyGameName)
    );
}

#[test]
fn executable_basename_is_lowercased_and_platform_agnostic() {
    let id = GameId::new("1").expect("id");
    let game = Game::new(
        id,
        "Terraria",
        Some("bin/win64/Terraria.EXE".to_owned()),
        SpoofMethod::Process,
    )
    .expect("game");
    assert_eq!(game.exe_basename().as_deref(), Some("terraria.exe"));

    let windows_style = Game::new(
        GameId::new("2").expect("id"),
        "CS2",
        Some("bin\\win64\\cs2.exe".to_owned()),
        SpoofMethod::Process,
    )
    .expect("game");
    assert_eq!(windows_style.exe_basename().as_deref(), Some("cs2.exe"));
}

#[test]
fn run_specs_are_validated_at_construction() {
    assert!(RunSpec::new(RunMode::Batch, 8, Duration::from_secs(15)).is_ok());
    assert!(RunSpec::new(RunMode::Batch, 0, Duration::from_secs(15)).is_err());
    assert!(RunSpec::new(RunMode::Batch, 21, Duration::from_secs(15)).is_err());
    assert!(RunSpec::new(RunMode::Fixed, 1, Duration::from_secs(1)).is_err());
    assert!(RunSpec::new(RunMode::Fixed, 1, Duration::from_secs(9 * 3600)).is_err());
}

#[test]
fn a_target_is_always_at_least_one() {
    let spec = RunSpec::new(RunMode::Batch, 4, Duration::from_secs(15))
        .expect("spec")
        .with_target(0);
    assert_eq!(spec.target, Some(1));
}
