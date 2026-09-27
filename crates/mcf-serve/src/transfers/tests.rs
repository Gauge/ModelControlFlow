use super::{AT_ONCE, Queue, State};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-serve::transfers::tests");

fn interrupted() -> Failure {
    Failure::new(
        Category::TransferInterrupted,
        Attribution::User,
        Disposition::Partial,
        WHERE,
        "stopped where it stood",
    )
}

fn broken() -> Failure {
    Failure::new(
        Category::HubUnreachable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the hub did not answer",
    )
}

fn state_of(queue: &Queue, id: u64) -> State {
    let held = queue.held();
    held.iter()
        .find(|transfer| transfer.id == id)
        .map(|transfer| transfer.state)
        .expect("the transfer is in the queue")
}

#[test]
fn more_than_one_file_can_be_asked_for_at_once() {
    let queue = Queue::new();
    let one = queue.ask_for("owner/model", "a.gguf", None);
    let two = queue.ask_for("owner/model", "b.gguf", None);
    let three = queue.ask_for("other/model", "c.gguf", None);
    assert_ne!(one, two);
    assert_ne!(two, three);
    assert_eq!(
        queue
            .to_value()
            .get("under_way")
            .and_then(mcf_record::json::Value::as_integer),
        Some(3),
        "everything asked for is in the queue, whatever else is going on"
    );
}

#[test]
fn only_so_many_fetch_at_once_and_the_rest_wait() {
    let queue = Queue::new();
    for at in 0..AT_ONCE.saturating_add(2) {
        let _asked = queue.ask_for("owner/model", &format!("{at}.gguf"), None);
    }
    let starting = queue.what_can_start();
    assert_eq!(
        starting.len(),
        AT_ONCE,
        "a queue that starts everything at once is not a queue"
    );
    assert!(
        queue.what_can_start().is_empty(),
        "asking again while they are still fetching must start nothing more"
    );
}

#[test]
fn asking_for_a_file_already_arriving_does_not_start_a_second_one() {
    let queue = Queue::new();
    let first = queue.ask_for("owner/model", "a.gguf", None);
    let _starting = queue.what_can_start();
    let again = queue.ask_for("owner/model", "a.gguf", None);
    assert_eq!(
        first, again,
        "two workers on one partial file would corrupt it"
    );
    assert!(queue.what_can_start().is_empty());
}

#[test]
fn a_paused_transfer_keeps_what_arrived_and_resumes_from_there() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    let _starting = queue.what_can_start();
    queue.sized(id, 1, 1_000);
    queue.moved(id, 1, 400);

    assert!(queue.pause(id));
    queue.stopped(id, &interrupted());
    assert_eq!(state_of(&queue, id), State::Paused);
    assert_eq!(
        queue.held()[0].arrived,
        400,
        "pausing must not forget what arrived, or resuming starts over"
    );

    assert!(queue.resume(id));
    assert_eq!(state_of(&queue, id), State::Queued);
    let starting = queue.what_can_start();
    assert_eq!(starting.len(), 1, "a resumed transfer runs again");
    assert!(
        !starting[0].stopping.asked(),
        "a resumed transfer must not still be carrying the asking that paused it"
    );
}

#[test]
fn pausing_one_that_has_not_started_needs_no_worker_to_notice() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    assert!(queue.pause(id));
    assert_eq!(state_of(&queue, id), State::Paused);
    assert!(
        queue.what_can_start().is_empty(),
        "a paused transfer is not started by the scheduler"
    );
}

#[test]
fn giving_up_is_not_pausing() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    let _starting = queue.what_can_start();
    let given = queue.give_up(id).expect("it is in the queue");
    assert!(
        !given.sweep_it_here,
        "a running worker sweeps its own partial on the way out"
    );
    assert!(
        queue.giving_up_on(id),
        "the worker must be able to tell why"
    );
    queue.stopped(id, &interrupted());
    assert_eq!(state_of(&queue, id), State::Cancelled);
}

#[test]
fn a_transfer_that_broke_is_not_reported_as_paused() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    let _starting = queue.what_can_start();
    queue.stopped(id, &broken());
    assert_eq!(state_of(&queue, id), State::Failed);
    assert!(
        queue.held()[0].why.is_some(),
        "a failure the operator cannot read is a failure they cannot act on"
    );
}

#[test]
fn one_that_arrived_says_where_it_landed() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    let _starting = queue.what_can_start();
    queue.sized(id, 1, 1_000);
    queue.arrived(id, std::path::Path::new("/models/owner/model/a.gguf"));
    assert_eq!(state_of(&queue, id), State::Done);
    assert_eq!(queue.held()[0].thousandths(), Some(1_000));
    assert!(!queue.anything_under_way());
}

#[test]
fn a_share_is_only_shown_once_there_is_a_whole_to_be_a_share_of() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    assert_eq!(
        queue.held()[0].thousandths(),
        None,
        "a share of an unknown whole is a number made up"
    );
    queue.sized(id, 2, 2_000);
    queue.moved(id, 1, 500);
    assert_eq!(queue.held()[0].thousandths(), Some(250));
}

#[test]
fn the_settled_are_forgotten_and_the_rest_are_kept() {
    let queue = Queue::new();
    let done = queue.ask_for("owner/model", "a.gguf", None);
    let waiting = queue.ask_for("owner/model", "b.gguf", None);
    queue.arrived(done, std::path::Path::new("/models/a.gguf"));
    assert_eq!(queue.forget_the_settled(), 1);
    assert_eq!(state_of(&queue, waiting), State::Queued);
}

#[test]
fn asking_again_for_one_that_failed_starts_it_again() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    let _starting = queue.what_can_start();
    queue.stopped(id, &broken());
    let again = queue.ask_for("owner/model", "a.gguf", None);
    assert_eq!(id, again, "it is the same file, not a second one");
    assert_eq!(state_of(&queue, id), State::Queued);
    assert!(queue.held()[0].why.is_none(), "the old refusal is stale");
}

#[test]
fn giving_up_one_that_is_not_running_hands_its_sweeping_to_the_caller() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a.gguf", None);
    assert!(queue.pause(id));
    assert_eq!(state_of(&queue, id), State::Paused);

    let given = queue.give_up(id).expect("it is in the queue");
    assert!(
        given.sweep_it_here,
        "a paused transfer has no worker to sweep what it wrote, so the bytes would sit \
         on the disk and be quietly resumed by the next ask for that file"
    );
    assert_eq!(given.file, "a.gguf");
    assert_eq!(state_of(&queue, id), State::Cancelled);
    assert!(
        queue.held()[0].why.is_none(),
        "it kept the words that explained the pause, and so read as having stopped by itself"
    );
}

#[test]
fn a_multi_part_file_goes_back_to_fetching_for_its_next_part() {
    let queue = Queue::new();
    let id = queue.ask_for("owner/model", "a-00001-of-00002.gguf", None);
    let _starting = queue.what_can_start();
    queue.sized(id, 2, 2_000);
    queue.checking(id);
    assert_eq!(state_of(&queue, id), State::Checking);
    queue.fetching(id);
    assert_eq!(
        state_of(&queue, id),
        State::Fetching,
        "a file still arriving must not read as one being checked"
    );
}

#[test]
fn nothing_the_queue_never_had_is_answered_about() {
    let queue = Queue::new();
    assert!(queue.give_up(404).is_none());
    assert!(!queue.pause(404));
    assert!(!queue.resume(404));
}

fn published(paths: &[&str]) -> mcf_hub::source::Listing {
    mcf_hub::source::Listing {
        reference: mcf_hub::reference::parse("owner/model").expect("a reference"),
        revision: Some("main".to_owned()),
        entries: paths
            .iter()
            .map(|path| mcf_hub::source::Entry::new(*path, 1))
            .collect(),
        gated: None,
        declared_licence: None,
        lineage: None,
    }
}

fn files_asked_for(queue: &Queue) -> Vec<String> {
    queue
        .held()
        .iter()
        .map(|transfer| transfer.file.clone())
        .collect()
}

#[test]
fn a_model_that_reads_pictures_queues_its_projector_when_it_arrives() {
    let root = std::env::temp_dir().join(format!("mcf-projector-queued-{}", std::process::id()));
    let queue = Queue::new();
    let listing = published(&["model-Q4_K_M.gguf", "mmproj-BF16.gguf", "mmproj-F16.gguf"]);
    super::ask_for_its_projector(
        &queue,
        "owner/model",
        None,
        &listing,
        "model-Q4_K_M.gguf",
        &root,
    );
    assert_eq!(
        files_asked_for(&queue),
        ["mmproj-F16.gguf"],
        "without its projector the model reads text only, which nobody asking for it asked for"
    );
}

#[test]
fn a_projector_already_here_is_not_asked_for_again() {
    let root = std::env::temp_dir().join(format!("mcf-projector-here-{}", std::process::id()));
    let beside = root.join("owner").join("model");
    std::fs::create_dir_all(&beside).expect("a directory for the model");
    std::fs::write(beside.join("mmproj-F16.gguf"), b"GGUF").expect("a projector on disk");
    let queue = Queue::new();
    let listing = published(&["model-Q4_K_M.gguf", "mmproj-F16.gguf"]);
    super::ask_for_its_projector(
        &queue,
        "owner/model",
        None,
        &listing,
        "model-Q4_K_M.gguf",
        &root,
    );
    let _swept = std::fs::remove_dir_all(&root);
    assert!(files_asked_for(&queue).is_empty());
}

#[test]
fn a_model_that_reads_text_only_queues_nothing_more() {
    let root = std::env::temp_dir().join(format!("mcf-projector-none-{}", std::process::id()));
    let queue = Queue::new();
    let listing = published(&["model-Q4_K_M.gguf", "README.md"]);
    super::ask_for_its_projector(
        &queue,
        "owner/model",
        None,
        &listing,
        "model-Q4_K_M.gguf",
        &root,
    );
    assert!(files_asked_for(&queue).is_empty());
}
