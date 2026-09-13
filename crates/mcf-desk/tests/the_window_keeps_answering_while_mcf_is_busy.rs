use mcf_desk::Desk;

fn desk() -> Desk {
    Desk::new(std::path::PathBuf::from(
        "/run/user/nowhere/mcf/control.sock",
    ))
}

#[test]
fn a_window_with_nothing_going_on_polls_as_usual() {
    assert!(
        !desk().busy_elsewhere(),
        "an idle window has every reason to ask the daemon how things are"
    );
}

#[test]
fn a_window_does_not_poll_a_daemon_that_is_loading_a_model() {
    let mut desk = desk();
    desk.doing = mcf_desk::Doing::Hosting(mcf_desk::job::Job::already(
        "holding".to_owned(),
        Vec::new(),
    ));
    assert!(
        desk.busy_elsewhere(),
        "the daemon cannot answer while it loads, so asking only stalls the window"
    );
}

#[test]
fn a_window_does_not_poll_a_daemon_that_is_building_an_engine() {
    let mut desk = desk();
    desk.doing = mcf_desk::Doing::Provisioning(mcf_desk::job::Job::already(
        "building".to_owned(),
        Vec::new(),
    ));
    assert!(desk.busy_elsewhere());
}

#[test]
fn a_poll_of_a_daemon_that_never_answers_costs_a_moment_rather_than_seconds() {
    let mut desk = desk();
    let began = std::time::Instant::now();
    desk.read_hosted();
    assert!(
        began.elapsed() < std::time::Duration::from_secs(2),
        "a window that waits five seconds on every frame is a window the desktop calls dead: \
         it waited {:?}",
        began.elapsed()
    );
}

#[test]
fn a_window_with_a_sweep_going_has_something_to_redraw_every_second() {
    let mut desk = desk();
    assert!(
        desk.optimizing.run.is_none(),
        "nothing is running to begin with"
    );
    desk.optimizing.running = true;
    assert!(
        !desk.busy_elsewhere(),
        "the running flag alone is not a daemon call in flight"
    );
}

#[test]
fn a_clock_that_only_moves_when_a_message_arrives_would_sit_still_for_a_minute() {
    use mcf_optimize::running::as_a_clock;
    use std::time::Duration;
    assert_ne!(
        as_a_clock(Duration::from_secs(10)),
        as_a_clock(Duration::from_secs(11)),
        "a trial takes the best part of a minute, so a label that only redraws when one \
         finishes looks stopped"
    );
}
