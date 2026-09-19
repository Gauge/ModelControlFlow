//! What this machine reports about itself. Ignored: it is a reading of whatever is here,
//! not a claim about what should be.

#[test]
#[ignore = "a reading of this machine"]
fn what_the_cards_say() {
    let mut sampler = mcf_tui::machine::Sampler::new();
    for card in sampler.read().cards {
        #[expect(clippy::integer_division, reason = "whole mebibytes is the unit shown")]
        let gb = |held: Option<u64>| {
            held.map_or_else(
                || "?".to_owned(),
                |held| format!("{} MiB", held / (1024 * 1024)),
            )
        };
        println!("{}: {} of {}", card.name, gb(card.used), gb(card.total));
    }
}
