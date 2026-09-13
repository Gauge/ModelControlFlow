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
