//! The simulated hub does what it says it does.
//!
//! A hub written to misbehave is only useful if its misbehaviour is exact, so
//! these check the simulator itself: each declared behaviour produces the
//! failure it names, and the well-formed case serves what it listed.

use super::{Behaviour, FakeHub, Repository};
use crate::world::World;
use mcf_core::failure::Category;
use mcf_hub::reference::parse;
use mcf_hub::source::Source;

fn hub() -> FakeHub {
    FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"the weights, notionally"),
    )
}

#[test]
fn a_well_formed_repository_lists_what_it_holds() {
    let listing = hub()
        .list(&parse("owner/model").expect("a reference"))
        .expect("it lists");
    assert_eq!(listing.entries.len(), 1);
    assert_eq!(
        listing.entry("model.gguf").map(|entry| entry.size),
        Some(23)
    );
    assert_eq!(listing.declared_licence.as_deref(), Some("apache-2.0"));
    assert_eq!(listing.revision.as_deref(), Some("main"));
    assert_eq!(listing.total_bytes(), Some(23));
}

#[test]
fn a_repository_that_is_not_there_is_not_found() {
    let failure = hub()
        .list(&parse("owner/absent").expect("a reference"))
        .expect_err("no such repository");
    assert_eq!(failure.category(), Category::HubRefNotFound);
}

/// Credentials change the answer, which is what makes *ask again with
/// credentials* a thing MCF can do rather than a thing it hopes for.
#[test]
fn credentials_change_what_a_repository_says() {
    let hub = FakeHub::new().with(
        "owner/private",
        Repository::holding("model.gguf", b"x").behaving(Behaviour::NeedsCredentials),
    );
    let reference = parse("owner/private").expect("a reference");
    let failure = hub.list(&reference).expect_err("no credentials");
    assert_eq!(failure.category(), Category::HubAuthRequired);

    let with_credentials = FakeHub::new()
        .with(
            "owner/private",
            Repository::holding("model.gguf", b"x").behaving(Behaviour::NeedsCredentials),
        )
        .authenticated();
    assert!(with_credentials.list(&reference).is_ok());
}

/// Gated is not the same as unauthenticated: credentials are present and the
/// terms are not accepted, and the two need different actions from a user.
#[test]
fn gated_is_its_own_answer_and_says_what_to_do() {
    let hub = FakeHub::new()
        .with(
            "owner/gated",
            Repository::holding("model.gguf", b"x").behaving(Behaviour::Gated),
        )
        .authenticated();
    let failure = hub
        .list(&parse("owner/gated").expect("a reference"))
        .expect_err("gated");
    assert_eq!(failure.category(), Category::HubAccessGated);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.key == "what_to_do"),
        "a gated repository must say what would change the answer"
    );
}

#[test]
fn throttling_carries_the_hint_a_hub_gives() {
    let hub = FakeHub::new().with(
        "owner/busy",
        Repository::holding("model.gguf", b"x")
            .behaving(Behaviour::RateLimited { retry_after: 30 }),
    );
    let failure = hub
        .list(&parse("owner/busy").expect("a reference"))
        .expect_err("throttled");
    assert_eq!(failure.category(), Category::HubRateLimited);
    assert!(
        failure.context().iter().any(|entry| entry.value == "30"),
        "the retry hint did not travel"
    );
}

/// A truncated transfer *writes* what it truncated, because the artifact on the
/// disk is what B-021 has to notice.
#[test]
fn a_truncated_transfer_leaves_a_partial_file() {
    let world = World::for_scenario("hub-truncates");
    let hub = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789")
            .behaving(Behaviour::Truncates { after: 4 }),
    );
    let reference = parse("owner/model").expect("a reference");
    let listing = hub.list(&reference).expect("it lists");
    let entry = listing.entry("model.gguf").expect("it is listed").clone();
    assert_eq!(entry.size, 10, "the hub claims the whole file");

    let into = world.path("model.gguf");
    let fetched = hub.fetch(&reference, &entry, &into).expect("it serves");
    assert_eq!(fetched.bytes, 4, "and serves four");
    assert_eq!(
        std::fs::read(&into)
            .expect("the partial file is there")
            .len(),
        4,
        "the partial artifact must be on the disk for a fetcher to find"
    );
}

/// The same length, different bytes: a file that changed under the fetch, which
/// a size check cannot see and a digest can.
#[test]
fn a_file_that_changed_under_the_fetch_has_a_different_digest() {
    let world = World::for_scenario("hub-mutates");
    let honest = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789"),
    );
    let mutating = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789").behaving(Behaviour::ServesDifferentBytes),
    );
    let reference = parse("owner/model").expect("a reference");
    let entry = honest
        .list(&reference)
        .expect("lists")
        .entry("model.gguf")
        .expect("listed")
        .clone();

    let one = honest
        .fetch(&reference, &entry, &world.path("honest.gguf"))
        .expect("serves");
    let other = mutating
        .fetch(&reference, &entry, &world.path("mutated.gguf"))
        .expect("serves");

    assert_eq!(one.bytes, other.bytes, "the size is the same");
    assert_ne!(one.digest, other.digest, "and the contents are not");
}

/// The digest is computed from what arrived, not reported by the source — a
/// checksum a hostile hub supplies is a checksum of what it wishes it had sent.
#[test]
fn the_digest_is_of_what_arrived() {
    let world = World::for_scenario("hub-digest");
    let hub = hub();
    let reference = parse("owner/model").expect("a reference");
    let entry = hub
        .list(&reference)
        .expect("lists")
        .entry("model.gguf")
        .expect("listed")
        .clone();
    let into = world.path("model.gguf");
    let fetched = hub.fetch(&reference, &entry, &into).expect("serves");

    let bytes = std::fs::read(&into).expect("it is there");
    assert_eq!(fetched.digest, mcf_core::digest::sha256(&bytes).hex());
}

/// A source says what it is, because that becomes a condition the moment
/// something acquired through it is measured (§3.4).
#[test]
fn a_source_describes_itself() {
    let described = hub().describe();
    assert!(described.contains("simulated hub"), "{described}");
    assert!(described.contains("credentials absent"), "{described}");
}

/// A repository with no licence declares none, rather than a plausible one.
#[test]
fn an_undeclared_licence_is_absent_rather_than_guessed() {
    let hub = FakeHub::new().with(
        "owner/quiet",
        Repository::holding("model.gguf", b"x").without_licence(),
    );
    let listing = hub
        .list(&parse("owner/quiet").expect("a reference"))
        .expect("lists");
    assert_eq!(listing.declared_licence, None);
}

/// Continuing a transfer appends where it left off, which is what makes a
/// resumption a resumption rather than a second copy of the beginning.
#[test]
fn a_continued_transfer_appends_from_the_offset() {
    let world = World::for_scenario("hub-resumes");
    let hub = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789")
            .behaving(Behaviour::StopsEvery { bytes: 4 }),
    );
    let reference = parse("owner/model").expect("a reference");
    let listing = hub.list(&reference).expect("lists");
    let entry = listing.entry("model.gguf").expect("listed").clone();
    let into = world.path("model.gguf.partial");

    let first = hub.fetch(&reference, &entry, &into).expect("serves");
    assert_eq!(first.bytes, 4);
    assert_eq!(std::fs::read(&into).expect("there"), b"0123");

    let second = hub
        .fetch_from(&reference, &entry, 4, &into)
        .expect("continues");
    assert_eq!(second.bytes, 4);
    assert_eq!(
        std::fs::read(&into).expect("there"),
        b"01234567",
        "it repeated the beginning instead of continuing"
    );
}

/// A hub without ranges says so rather than starting again and reporting
/// progress that did not happen.
#[test]
fn a_hub_without_ranges_says_so() {
    let world = World::for_scenario("hub-no-ranges");
    let hub = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789").behaving(Behaviour::NeverResumes),
    );
    let reference = parse("owner/model").expect("a reference");
    let entry = hub
        .list(&reference)
        .expect("lists")
        .entry("model.gguf")
        .expect("listed")
        .clone();
    let failure = hub
        .fetch_from(&reference, &entry, 4, &world.path("model.gguf.partial"))
        .expect_err("no ranges");
    assert_eq!(failure.category(), Category::HubUnreachable);
}

/// A listing carries the digest the hub declares, and a hub that declares none
/// says nothing rather than something plausible.
#[test]
fn a_listing_carries_the_digest_a_hub_declares() {
    let declaring =
        FakeHub::new().with("owner/model", Repository::holding("model.gguf", b"weights"));
    let reference = parse("owner/model").expect("a reference");
    let entry = declaring
        .list(&reference)
        .expect("lists")
        .entry("model.gguf")
        .expect("listed")
        .clone();
    assert_eq!(
        entry.digest.as_deref(),
        Some(mcf_core::digest::sha256(b"weights").hex().as_str())
    );

    let quiet = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"weights").without_digests(),
    );
    let entry = quiet
        .list(&reference)
        .expect("lists")
        .entry("model.gguf")
        .expect("listed")
        .clone();
    assert_eq!(entry.digest, None);
}
