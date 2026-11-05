#![allow(clippy::expect_used, clippy::unwrap_used)]
use questline_core::{batch_count, BatchPlanner, Game, GameId, SpoofMethod};

fn game(id: &str, name: &str, exe: Option<&str>) -> Game {
    Game::new(
        GameId::new(id).expect("valid id"),
        name,
        exe.map(str::to_owned),
        SpoofMethod::Process,
    )
    .expect("valid game")
}

#[test]
fn a_batch_never_exceeds_its_size() {
    let mut pool = vec![
        game("1", "A", Some("a.exe")),
        game("2", "B", Some("b.exe")),
        game("3", "C", Some("c.exe")),
        game("4", "D", Some("d.exe")),
    ];
    let planner = BatchPlanner::new(2).expect("planner");

    let first = planner.plan(&mut pool, None);
    assert_eq!(first.len(), 2);
    assert_eq!(pool.len(), 2, "chosen games leave the pool");
    assert_eq!(first[0].name, "A");
    assert_eq!(first[1].name, "B");
}

#[test]
fn games_sharing_an_executable_never_share_a_batch() {
    let mut pool = vec![
        game("1", "First", Some("shared.exe")),
        game("2", "Second", Some("SHARED.EXE")),
        game("3", "Third", Some("other.exe")),
    ];
    let planner = BatchPlanner::new(8).expect("planner");

    let batch = planner.plan(&mut pool, None);
    assert_eq!(
        batch.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
        vec!["First", "Third"]
    );
    assert_eq!(pool.len(), 1, "the duplicate stays for a later batch");
    assert_eq!(pool[0].name, "Second");
}

#[test]
fn rpc_games_are_not_deduplicated_by_executable() {
    let mut pool = vec![game("1", "Rpc one", None), game("2", "Rpc two", None)];
    let planner = BatchPlanner::new(8).expect("planner");

    let batch = planner.plan(&mut pool, None);
    assert_eq!(batch.len(), 2);
    assert_eq!(pool.len(), 0, "the pool is drained");
}

#[test]
fn the_remaining_target_trims_the_last_batch() {
    let mut pool = (1..=5)
        .map(|i| game(&i.to_string(), "game", Some(&format!("g{i}.exe"))))
        .collect::<Vec<_>>();
    let planner = BatchPlanner::new(8).expect("planner");

    let batch = planner.plan(&mut pool, Some(3));
    assert_eq!(batch.len(), 3);
    assert_eq!(pool.len(), 2);
}

#[test]
fn a_zero_target_plans_nothing() {
    let mut pool = vec![game("1", "A", Some("a.exe"))];
    let planner = BatchPlanner::new(4).expect("planner");
    assert_eq!(planner.plan(&mut pool, Some(0)).len(), 0);
    assert_eq!(pool.len(), 1);
}

#[test]
fn an_empty_pool_produces_an_empty_batch() {
    let mut pool: Vec<Game> = Vec::new();
    let planner = BatchPlanner::new(4).expect("planner");
    assert_eq!(planner.plan(&mut pool, None).len(), 0);
}

#[test]
fn planner_rejects_an_impossible_batch_size() {
    assert!(BatchPlanner::new(0).is_err());
    assert!(BatchPlanner::new(21).is_err());
    assert!(BatchPlanner::new(20).is_ok());
}

#[test]
fn batch_count_rounds_up() {
    assert_eq!(batch_count(0, 8), 0);
    assert_eq!(batch_count(1, 8), 1);
    assert_eq!(batch_count(8, 8), 1);
    assert_eq!(batch_count(9, 8), 2);
    assert_eq!(batch_count(17, 8), 3);
    assert_eq!(batch_count(5, 0), 0, "a zero batch size plans nothing");
}
