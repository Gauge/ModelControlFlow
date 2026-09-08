use std::path::{Path, PathBuf};

use super::{
    Authorization, held, preview, provenance_of, provenance_path, purge, record_provenance, remove,
    restore,
};
use mcf_core::attested::Attested;
use mcf_core::failure::Category;
use mcf_core::provenance::{Licence, Origin, Provenance, Repository, Revision};
use mcf_core::time::Timestamp;
use mcf_record::journal::Journal;

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mcf-store-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _fresh = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Self { path }
    }

    fn at(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    fn holding(&self, name: &str, bytes: usize) -> PathBuf {
        let path = self.at(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the parent exists");
        }
        std::fs::write(&path, vec![b'w'; bytes]).expect("the file is written");
        path
    }

    fn journal(&self) -> Journal {
        Journal::open(&self.at("journal.jsonl")).expect("a journal opens")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.path);
    }
}

fn now() -> Timestamp {
    Timestamp::now()
}

#[test]
fn a_preview_touches_nothing() {
    let scratch = Scratch::new("preview");
    let model = scratch.holding("model.gguf", 128);
    let plan = preview(std::slice::from_ref(&model), &scratch.at("shelf")).expect("a plan");

    assert!(model.exists(), "the preview removed something");
    assert_eq!(plan.bytes(), Some(128));
    assert_eq!(plan.doomed().len(), 1);
    let described = plan.describe();
    assert!(described.contains("model.gguf"), "{described}");
    assert!(described.contains("128 bytes"), "{described}");
}

#[test]
fn a_plan_will_not_describe_what_is_not_there() {
    let scratch = Scratch::new("absent");
    let failure = preview(&[scratch.at("never-existed.gguf")], &scratch.at("shelf"))
        .expect_err("nothing to look at");
    assert_eq!(failure.category(), Category::ArtifactMissing);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("never-existed.gguf")),
        "the refusal does not say which path"
    );
}

#[test]
fn an_authorization_without_a_reason_is_not_one() {
    let scratch = Scratch::new("reason");
    let model = scratch.holding("model.gguf", 8);
    let plan = preview(std::slice::from_ref(&model), &scratch.at("shelf")).expect("a plan");

    let failure = Authorization::given(&plan, "   ").expect_err("no reason given");
    assert_eq!(failure.category(), Category::ConfigInvalid);
    Authorization::given(&plan, "replaced by the Q6 quantization").expect("a reason");
}

#[test]
fn an_authorization_does_not_carry_to_a_removal_nobody_previewed() {
    let scratch = Scratch::new("stale");
    let model = scratch.holding("model.gguf", 8);
    let shelf = scratch.at("shelf");
    let plan = preview(std::slice::from_ref(&model), &shelf).expect("a plan");
    let authorization = Authorization::given(&plan, "making room").expect("authorized");

    std::fs::write(&model, vec![b'w'; 4096]).expect("the model grows");
    let now_a_different_removal =
        preview(std::slice::from_ref(&model), &shelf).expect("a second plan");
    assert_ne!(
        plan.identity(),
        now_a_different_removal.identity(),
        "a changed artifact produced the same plan identity"
    );

    let mut journal = scratch.journal();
    let failure = remove(
        &now_a_different_removal,
        &authorization,
        &mut journal,
        now(),
    )
    .expect_err("the authorization is stale");
    assert_eq!(failure.category(), Category::ConfigInvalid);
    assert!(model.exists(), "a refused removal removed something");
}

#[test]
fn a_removal_records_first_and_deletes_nothing() {
    let scratch = Scratch::new("removal");
    let model = scratch.holding("model.gguf", 64);
    let shelf = scratch.at("shelf");
    let plan = preview(std::slice::from_ref(&model), &shelf).expect("a plan");
    let authorization =
        Authorization::given(&plan, "superseded by a larger quantization").expect("authorized");

    let mut journal = scratch.journal();
    let removed = remove(&plan, &authorization, &mut journal, now()).expect("the removal runs");

    assert!(removed.complete(), "{:?}", removed.refused);
    assert_eq!(removed.bytes, 64);
    assert!(!model.exists(), "the artifact is still in its old place");
    let shelved = removed.shelved.first().expect("one shelved file");
    assert!(
        shelved.exists(),
        "the artifact was deleted rather than moved"
    );
    assert!(
        shelved.starts_with(&shelf),
        "it went somewhere else entirely"
    );

    let written = std::fs::read_to_string(journal.path()).expect("the journal is readable");
    assert!(written.contains("artifact_removed"), "{written}");
    assert!(
        written.contains("superseded by a larger quantization"),
        "{written}"
    );
    assert!(written.contains("model.gguf"), "{written}");
}

#[test]
fn the_record_says_what_left_and_on_whose_word() {
    let scratch = Scratch::new("record");
    let model = scratch.holding("model.gguf", 4096);
    let shelf = scratch.at("shelf");
    let plan = preview(std::slice::from_ref(&model), &shelf).expect("a plan");
    let authorization = Authorization::given(&plan, "the disk was short").expect("authorized");

    let mut journal = scratch.journal();
    remove(&plan, &authorization, &mut journal, now()).expect("the removal runs");

    let written = std::fs::read_to_string(journal.path()).expect("the journal is readable");
    assert!(written.contains("4096"), "the size is not in the record");
    assert!(written.contains(plan.identity()), "the plan is not named");
    assert!(
        written.contains(&shelf.display().to_string()),
        "the shelf is not named, so nobody can find what was removed"
    );
}

#[test]
fn what_was_shelved_can_be_put_back() {
    let scratch = Scratch::new("restore");
    let model = scratch.holding("model.gguf", 32);
    let plan = preview(std::slice::from_ref(&model), &scratch.at("shelf")).expect("a plan");
    assert!(
        plan.reversible(),
        "a shelf beside the artifact should be reversible"
    );
    let authorization = Authorization::given(&plan, "a mistake in progress").expect("authorized");

    let mut journal = scratch.journal();
    let removed = remove(&plan, &authorization, &mut journal, now()).expect("the removal runs");
    let back = restore(&removed, &plan).expect("it comes back");

    assert_eq!(back, vec![model.clone()]);
    assert!(model.exists());
    assert_eq!(
        std::fs::read(&model).expect("readable").len(),
        32,
        "it came back as something else"
    );
}

#[test]
fn purging_is_a_separate_decision() {
    let scratch = Scratch::new("purge");
    let model = scratch.holding("model.gguf", 100);
    let plan = preview(std::slice::from_ref(&model), &scratch.at("shelf")).expect("a plan");
    let authorization = Authorization::given(&plan, "done with it").expect("authorized");

    let mut journal = scratch.journal();
    let removed = remove(&plan, &authorization, &mut journal, now()).expect("the removal runs");
    let shelved = removed.shelved.first().cloned().expect("one shelved file");
    assert!(shelved.exists(), "the removal deleted it by itself");

    let freed = purge(&removed, &authorization, &plan).expect("the purge runs");
    assert_eq!(freed, 100);
    assert!(!shelved.exists());
}

#[test]
fn a_purge_checks_which_removal_it_was_told_about() {
    let scratch = Scratch::new("purge-mismatch");
    let one = scratch.holding("one.gguf", 10);
    let two = scratch.holding("two.gguf", 20);
    let shelf = scratch.at("shelf");
    let plan = preview(std::slice::from_ref(&one), &shelf).expect("a plan");
    let other = preview(std::slice::from_ref(&two), &shelf).expect("another plan");
    let for_the_other = Authorization::given(&other, "the other one").expect("authorized");

    let mine = Authorization::given(&plan, "this one").expect("authorized");
    let mut journal = scratch.journal();
    let removed = remove(&plan, &mine, &mut journal, now()).expect("the removal runs");

    let failure = purge(&removed, &for_the_other, &plan).expect_err("the wrong authorization");
    assert_eq!(failure.category(), Category::ConfigInvalid);
    assert!(
        removed.shelved.first().is_some_and(|path| path.exists()),
        "a refused purge deleted something"
    );
}

#[test]
fn two_artifacts_of_the_same_name_do_not_collide_on_the_shelf() {
    let scratch = Scratch::new("collide");
    let first = scratch.holding("a/model.gguf", 10);
    let second = scratch.holding("b/model.gguf", 20);
    let shelf = scratch.at("shelf");
    let plan = preview(&[first, second], &shelf).expect("a plan");
    let authorization = Authorization::given(&plan, "clearing both").expect("authorized");

    let mut journal = scratch.journal();
    let removed = remove(&plan, &authorization, &mut journal, now()).expect("the removal runs");

    assert_eq!(removed.shelved.len(), 2);
    assert_ne!(
        removed.shelved.first(),
        removed.shelved.get(1),
        "both artifacts were shelved under one name"
    );
    for (path, size) in removed.shelved.iter().zip([10_usize, 20]) {
        assert_eq!(
            std::fs::read(path).expect("shelved and readable").len(),
            size
        );
    }
    assert_eq!(removed.bytes, 30);
}

#[test]
fn reversibility_is_what_the_filesystem_says_it_is() {
    use std::os::unix::fs::MetadataExt as _;

    let scratch = Scratch::new("devices");
    let model = scratch.holding("model.gguf", 16);
    let elsewhere = Path::new("/dev/shm");
    if !elsewhere.exists() {
        return;
    }
    let shelf = elsewhere.join(format!("mcf-store-shelf-{}", std::process::id()));

    let artifact_device = std::fs::metadata(&model).expect("readable").dev();
    let shelf_device = std::fs::metadata(elsewhere).expect("readable").dev();
    let plan = preview(std::slice::from_ref(&model), &shelf).expect("a plan");

    assert_eq!(
        plan.reversible(),
        artifact_device == shelf_device,
        "MCF and the kernel disagree about whether this removal could be undone"
    );
    let described = plan.describe();
    if plan.reversible() {
        assert!(described.contains("recoverable from"), "{described}");
    } else {
        assert!(described.contains("NOT recoverable"), "{described}");
    }
    let _cleared = std::fs::remove_dir_all(&shelf);
}

#[test]
fn a_plan_with_nothing_in_it_is_not_reversible_or_otherwise() {
    let scratch = Scratch::new("empty");
    let plan = preview(&[], &scratch.at("shelf")).expect("an empty plan");
    assert_eq!(plan.bytes(), Some(0));
    assert!(!plan.reversible(), "there is nothing to reverse");
    assert!(plan.doomed().is_empty());
}

#[test]
fn provenance_is_written_beside_the_artifact_and_read_from_there() {
    let scratch = Scratch::new("sidecar");
    let model = scratch.holding("model.gguf", 16);
    let provenance = Provenance::acquired(
        Origin::hub(
            Repository::new("owner/model"),
            Some(Revision::new("abc123")),
        ),
        Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
    )
    .with_licence(Licence::spdx("apache-2.0"));

    let sidecar = record_provenance(&model, &provenance).expect("it is written");
    assert!(sidecar.exists());
    assert!(
        sidecar
            .display()
            .to_string()
            .starts_with(&model.display().to_string()),
        "the sidecar is not beside the artifact: {}",
        sidecar.display()
    );
    assert_eq!(provenance_of(&model).expect("it reads back"), provenance);
}

#[test]
fn an_artifact_with_no_provenance_says_so() {
    let scratch = Scratch::new("no-sidecar");
    let model = scratch.holding("model.gguf", 16);
    let failure = provenance_of(&model).expect_err("nothing beside it");
    assert_eq!(failure.category(), Category::ArtifactMissing);

    let holding = held(&scratch.path).expect("the directory reads");
    let listed = holding
        .iter()
        .find(|held| held.path == model)
        .expect("the artifact is listed");
    assert_eq!(listed.provenance, Err(None));
    let described = listed.describe();
    assert!(described.contains("origin unknown"), "{described}");
}

#[test]
fn a_provenance_that_cannot_be_read_is_not_the_same_as_none() {
    let scratch = Scratch::new("bad-sidecar");
    let model = scratch.holding("model.gguf", 16);
    std::fs::write(provenance_path(&model), "{not json at all").expect("the sidecar is written");

    let holding = held(&scratch.path).expect("the directory reads");
    let listed = holding
        .iter()
        .find(|held| held.path == model)
        .expect("the artifact is listed");
    match &listed.provenance {
        Err(Some(failure)) => assert_eq!(
            failure.category(),
            Category::ArtifactProvenanceIncomplete,
            "{failure}"
        ),
        other => panic!("an unreadable provenance was listed as {other:?}"),
    }
    assert!(listed.describe().contains("unreadable"));
}

#[test]
fn a_listing_does_not_list_the_sidecars() {
    let scratch = Scratch::new("listing");
    let first = scratch.holding("a/model.gguf", 10);
    let second = scratch.holding("b/model.gguf", 20);
    for artifact in [&first, &second] {
        record_provenance(
            artifact,
            &Provenance::acquired(
                Origin::Unattributed,
                Timestamp::from_utc_nanos(0, Attested::Unknown),
            ),
        )
        .expect("it is written");
    }

    let holding = held(&scratch.path).expect("the directory reads");
    let paths: Vec<&PathBuf> = holding.iter().map(|held| &held.path).collect();
    assert_eq!(paths, vec![&first, &second], "{holding:?}");
    assert_eq!(holding.first().map(|held| held.bytes), Some(10));
}

#[test]
fn a_removal_takes_the_provenance_with_the_artifact() {
    let scratch = Scratch::new("removal-sidecar");
    let model = scratch.holding("model.gguf", 24);
    let sidecar = record_provenance(
        &model,
        &Provenance::acquired(
            Origin::hub(Repository::new("owner/model"), None),
            Timestamp::from_utc_nanos(0, Attested::Unknown),
        ),
    )
    .expect("it is written");

    let plan = preview(std::slice::from_ref(&model), &scratch.at("shelf")).expect("a plan");
    assert_eq!(
        plan.doomed().len(),
        2,
        "the plan does not name the provenance: {}",
        plan.describe()
    );
    assert!(
        plan.describe().contains("mcf-provenance"),
        "{}",
        plan.describe()
    );

    let authorization = Authorization::given(&plan, "clearing it out").expect("authorized");
    let mut journal = scratch.journal();
    let removed = remove(&plan, &authorization, &mut journal, now()).expect("the removal runs");

    assert!(removed.complete(), "{:?}", removed.refused);
    assert!(!model.exists());
    assert!(!sidecar.exists(), "the provenance was left behind");
    assert_eq!(removed.shelved.len(), 2);

    restore(&removed, &plan).expect("it comes back");
    assert!(model.exists());
    assert!(sidecar.exists(), "the provenance did not come back");
    assert!(provenance_of(&model).is_ok());
}

#[test]
fn the_parts_of_a_model_are_gathered_into_the_model() {
    assert_eq!(
        super::part_of_a_set(std::path::Path::new("/m/a-model-00001-of-00004.gguf")),
        Some(("a-model".to_owned(), 1))
    );
    assert_eq!(
        super::part_of_a_set(std::path::Path::new("/m/a-model-00003-of-00004.gguf")),
        Some(("a-model".to_owned(), 3))
    );

    assert_eq!(
        super::part_of_a_set(std::path::Path::new("/m/a-model.gguf")),
        None
    );
    assert_eq!(
        super::part_of_a_set(std::path::Path::new("/m/a-model-Q4_K_M.gguf")),
        None
    );
    assert_eq!(
        super::part_of_a_set(std::path::Path::new("/m/a-model-00000-of-00004.gguf")),
        None
    );
    assert_eq!(
        super::part_of_a_set(std::path::Path::new("/m/best-of-breed.gguf")),
        None
    );
    assert_eq!(
        super::part_of_a_set(std::path::Path::new("/m/a-model-0000x-of-00004.gguf")),
        None
    );
}

#[test]
fn a_set_is_one_entry_of_the_whole_length() {
    let part = |name: &str, bytes: u64| super::Held {
        path: std::path::PathBuf::from(format!("/m/{name}")),
        bytes,
        parts: 1,
        companion: false,
        provenance: Err(None),
    };
    let gathered = super::gathered(vec![
        part("a-model-00001-of-00002.gguf", 40),
        part("a-model-00002-of-00002.gguf", 30),
        part("whole.gguf", 7),
    ]);
    assert_eq!(gathered.len(), 2, "two models, not three files");

    let set = gathered
        .iter()
        .find(|held| held.path.ends_with("a-model-00001-of-00002.gguf"))
        .expect("the set is named by its first part, which is what an engine is pointed at");
    assert_eq!(set.bytes, 70, "the set's length, not the first part's");
    assert_eq!(set.parts, 2);

    let whole = gathered
        .iter()
        .find(|held| held.path.ends_with("whole.gguf"))
        .expect("a model in one file is untouched");
    assert_eq!(whole.bytes, 7);
    assert_eq!(whole.parts, 1);
}

#[test]
fn a_set_missing_its_first_part_is_not_offered() {
    let gathered = super::gathered(vec![super::Held {
        path: std::path::PathBuf::from("/m/a-model-00002-of-00002.gguf"),
        bytes: 30,
        parts: 1,
        companion: false,
        provenance: Err(None),
    }]);
    assert!(
        gathered.is_empty(),
        "there is no first part to point an engine at, so there is no model to offer"
    );
}

#[test]
fn a_projector_is_a_companion_and_a_model_is_not() {
    assert!(super::is_a_companion(std::path::Path::new(
        "/m/mmproj-F16.gguf"
    )));
    assert!(super::is_a_companion(std::path::Path::new(
        "/m/MMPROJ-BF16.gguf"
    )));
    assert!(!super::is_a_companion(std::path::Path::new(
        "/m/a-model-Q4_K_M.gguf"
    )));
    assert!(!super::is_a_companion(std::path::Path::new(
        "/m/a-model-with-mmproj-inside.gguf"
    )));
}
