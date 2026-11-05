#![allow(clippy::expect_used, clippy::unwrap_used)]
use questline_core::{RunMachine, RunState};

fn running(batch: u32) -> RunMachine {
    let mut machine = RunMachine::new();
    machine.start().expect("start");
    machine.batch_started(batch).expect("batch");
    machine
}

#[test]
fn a_new_machine_is_idle() {
    assert_eq!(RunMachine::new().state(), RunState::Idle);
}

#[test]
fn happy_path_start_run_finish_reset() {
    let mut machine = RunMachine::new();
    assert_eq!(machine.start(), Ok(RunState::Preparing));
    assert_eq!(machine.batch_started(1), Ok(RunState::Running { batch: 1 }));
    assert_eq!(machine.complete(), Ok(RunState::Finished { batches: 1 }));
    assert_eq!(machine.reset(), Ok(RunState::Idle));
}

#[test]
fn pause_and_resume_keep_the_batch() {
    let mut machine = running(3);
    assert_eq!(machine.pause(), Ok(RunState::Paused { batch: 3 }));
    assert_eq!(machine.resume(), Ok(RunState::Running { batch: 3 }));
}

#[test]
fn stop_from_running_and_from_paused() {
    let mut machine = running(2);
    assert_eq!(machine.request_stop(), Ok(RunState::Stopping { batch: 2 }));
    assert_eq!(machine.complete(), Ok(RunState::Finished { batches: 2 }));

    let mut paused = running(5);
    paused.pause().expect("pause");
    assert_eq!(paused.request_stop(), Ok(RunState::Stopping { batch: 5 }));
}

#[test]
fn stop_while_preparing_has_no_batch_yet() {
    let mut machine = RunMachine::new();
    machine.start().expect("start");
    assert_eq!(machine.request_stop(), Ok(RunState::Stopping { batch: 0 }));
}

#[test]
fn stopping_is_not_cancellable() {
    let mut machine = running(1);
    machine.request_stop().expect("stop");
    assert!(machine.start().is_err(), "cannot restart a stopping run");
    assert!(machine.pause().is_err(), "cannot pause a stopping run");
    assert!(
        machine.resume().is_err(),
        "nothing to resume while stopping"
    );
}

#[test]
fn a_failed_run_can_be_started_again() {
    let mut machine = running(4);
    assert_eq!(machine.fail(), Ok(RunState::Failed { batch: 4 }));
    assert_eq!(machine.start(), Ok(RunState::Preparing));
    assert_eq!(
        machine.state(),
        RunState::Preparing,
        "start clears the failure"
    );
}

#[test]
fn failing_twice_is_refused() {
    let mut machine = running(1);
    machine.fail().expect("first failure");
    let second = machine.fail();
    assert!(second.is_err());
    assert_eq!(machine.state(), RunState::Failed { batch: 1 });
}

#[test]
fn illegal_transitions_are_named_in_the_error() {
    let mut machine = RunMachine::new();

    let err = machine.pause().expect_err("cannot pause an idle run");
    assert_eq!(err.action, "pause");
    assert_eq!(err.from, RunState::Idle);
    assert_eq!(err.to_string(), "cannot pause while idle");

    let err = machine.complete().expect_err("cannot complete idle");
    assert_eq!(err.action, "complete");

    let err = machine.reset().expect_err("idle needs no reset");
    assert_eq!(err.action, "reset");
}

#[test]
fn resume_requires_a_pause() {
    let mut machine = running(1);
    assert!(machine.resume().is_err());
}

#[test]
fn active_states_are_the_ones_that_own_processes() {
    let mut machine = RunMachine::new();
    assert!(!machine.state().is_active());

    machine.start().expect("start");
    assert!(machine.state().is_active());

    machine.batch_started(1).expect("batch");
    assert!(machine.state().is_active());

    machine.complete().expect("complete");
    assert!(
        !machine.state().is_active(),
        "finished must not own processes"
    );
}

#[test]
fn batch_numbers_are_reported_per_state() {
    let machine = running(7);
    assert_eq!(machine.state().batch(), Some(7));
    assert_eq!(RunState::Idle.batch(), None);
    assert_eq!(RunState::Preparing.batch(), None);
}
