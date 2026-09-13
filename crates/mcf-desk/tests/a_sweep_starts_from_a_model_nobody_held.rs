use mcf_desk::{Act, Desk};

fn desk() -> Desk {
    Desk::new(std::path::PathBuf::from("/nowhere"))
}

#[test]
fn nothing_in_the_refusals_asks_for_a_model_to_be_hosted_first() {
    let mut desk = desk();
    for _ in 0..3 {
        desk.act(Act::Sweep);
        let why = desk.optimizing.refused.clone().unwrap_or_default();
        assert!(
            !why.to_lowercase().contains("host one first"),
            "a sweep holds the model itself: {why}"
        );
    }
}

#[test]
fn a_sweep_says_what_is_missing_rather_than_doing_nothing() {
    let mut desk = desk();
    desk.act(Act::Sweep);
    assert!(
        desk.optimizing.refused.is_some(),
        "pressing the button and seeing nothing happen is the worst outcome there is"
    );
    assert!(!desk.optimizing.running);
}
