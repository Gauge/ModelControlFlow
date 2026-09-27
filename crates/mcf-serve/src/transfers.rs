//! The queue of model files MCF is bringing here.
//!
//! A transfer is not a hold. Holding a model runs it, and MCF holds one at a time because
//! a machine has one lot of memory; fetching a file only writes bytes to a disk, and
//! nothing about that says it must wait for whatever is running. So a transfer is asked
//! for whenever the operator likes — while a model is held, while a sweep is under way,
//! while another transfer is still arriving — and it is queued here rather than carried on
//! the connection that asked for it.
//!
//! Queuing it here is also what lets the window be closed without losing it. The queue
//! belongs to the daemon, so the window, the terminal interface and the command line are
//! all looking at the same one, and closing any of them changes nothing about what is
//! still arriving.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mcf_core::failure::Failure;
use mcf_core::time::Timestamp;
use mcf_record::json::Value;

/// How many files are fetched at once.
///
/// More than one, because a single transfer rarely saturates a connection and waiting for
/// a large file to finish before a small one starts is time spent for nothing. Not many
/// more, because every one of them is writing to the same disk and competing for the same
/// connection, and a hub that is asked for six files at once starts refusing.
pub const AT_ONCE: usize = 2;

/// Where a transfer has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Asked for, and waiting for one of the places to fetch in.
    Queued,
    /// Bytes are arriving.
    Fetching,
    /// Everything asked for has arrived and is being read back against its digest.
    Checking,
    /// Stopped where it stood, on purpose. What arrived is on disk and resuming continues
    /// from there.
    Paused,
    /// Here, whole, and checked as far as the repository let it be checked.
    Done,
    /// It did not arrive, and the refusal says why.
    Failed,
    /// Given up, and what had arrived swept.
    Cancelled,
}

impl State {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Fetching => "fetching",
            Self::Checking => "checking",
            Self::Paused => "paused",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "queued" => Some(Self::Queued),
            "fetching" => Some(Self::Fetching),
            "checking" => Some(Self::Checking),
            "paused" => Some(Self::Paused),
            "done" => Some(Self::Done),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Whether a transfer in this state is still going to move on its own. A queue that
    /// says nothing is moving when something is is worse than no queue at all.
    #[must_use]
    pub const fn under_way(self) -> bool {
        matches!(self, Self::Queued | Self::Fetching | Self::Checking)
    }

    /// Whether there is nothing more for this one to do.
    #[must_use]
    pub const fn settled(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }
}

/// Why a transfer was asked to stop. A pause keeps what arrived; giving up sweeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    Pause,
    GiveUp,
}

/// What a worker reads to know it should stop, and why.
///
/// The asking is kept behind its own `Arc` because that is what is handed to the worker:
/// the worker reads whether to stop without holding a reference to the queue, so the queue
/// stays lockable while a transfer is running.
#[derive(Debug, Default)]
struct Stop {
    asked: Arc<AtomicBool>,
    giving_up: AtomicBool,
}

impl Stop {
    fn ask(&self, asked: Asked) {
        if asked == Asked::GiveUp {
            self.giving_up.store(true, Ordering::Relaxed);
        }
        self.asked.store(true, Ordering::Relaxed);
    }

    fn clear(&self) {
        self.giving_up.store(false, Ordering::Relaxed);
        self.asked.store(false, Ordering::Relaxed);
    }

    fn giving_up(&self) -> bool {
        self.giving_up.load(Ordering::Relaxed)
    }

    /// The flag the worker watches.
    fn watched(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.asked)
    }
}

/// One file MCF was asked to bring here, and how far it has got.
#[derive(Debug)]
pub struct Transfer {
    pub id: u64,
    pub reference: String,
    pub file: String,
    pub from: Option<String>,
    /// Which part of a file published in parts is arriving, and how many there are. One
    /// of one for a file published whole.
    pub part: usize,
    pub parts: usize,
    /// Bytes on disk across every part, and bytes the repository says the whole is.
    pub arrived: u64,
    pub whole: u64,
    pub state: State,
    /// The refusal, written the way the record writes one, so that what the queue shows
    /// and what the record holds are the same account.
    pub why: Option<Value>,
    pub asked_at: Timestamp,
    /// Where it landed, once it has.
    pub path: Option<PathBuf>,
    stop: Arc<Stop>,
}

impl Transfer {
    #[must_use]
    pub fn to_value(&self) -> Value {
        let whole = |held: u64| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
        Value::map([
            ("id", whole(self.id)),
            ("reference", Value::text(self.reference.clone())),
            ("file", Value::text(self.file.clone())),
            (
                "from",
                self.from
                    .as_ref()
                    .map_or(Value::Null, |hub| Value::text(hub.clone())),
            ),
            ("part", whole(self.part as u64)),
            ("parts", whole(self.parts as u64)),
            ("arrived_bytes", whole(self.arrived)),
            ("whole_bytes", whole(self.whole)),
            ("state", Value::text(self.state.as_str())),
            ("why", self.why.clone().unwrap_or(Value::Null)),
            ("asked_at", mcf_record::encode::timestamp(self.asked_at)),
            (
                "path",
                self.path
                    .as_ref()
                    .map_or(Value::Null, |path| Value::text(path.display().to_string())),
            ),
        ])
    }

    /// How far along, in thousandths, or nothing when the whole is not yet known. A share
    /// is only honest once there is something to be a share of.
    #[must_use]
    pub fn thousandths(&self) -> Option<u64> {
        (self.whole > 0).then(|| {
            #[expect(
                clippy::integer_division,
                reason = "whole thousandths; what is discarded is under a thousandth"
            )]
            let share = self.arrived.saturating_mul(1_000) / self.whole;
            share.min(1_000)
        })
    }
}

/// The queue itself.
#[derive(Debug, Default)]
pub struct Queue {
    held: Mutex<Vec<Transfer>>,
    next: AtomicU64,
}

/// A transfer that was given up, and whether its partial bytes are the caller's to sweep.
#[derive(Debug, Clone)]
pub struct GivenUp {
    pub reference: String,
    pub file: String,
    pub sweep_it_here: bool,
}

/// What the daemon must do for a transfer the queue has decided should start. The queue
/// works out what should run while holding its own lock; fetching cannot be done under
/// that lock, so what to start is handed back instead of started here.
#[derive(Debug, Clone)]
pub struct Start {
    pub id: u64,
    pub reference: String,
    pub file: String,
    pub from: Option<String>,
    pub stopping: mcf_hub::stopping::Stopping,
}

impl Queue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Vec<Transfer>> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Ask for a file. A file already in the queue and still going is not asked for twice:
    /// the transfer already under way is the answer, and two workers writing one partial
    /// file would corrupt it.
    ///
    /// One already settled — done, failed, given up — is asked for again from where it
    /// stands, because that is what asking again plainly means.
    pub fn ask_for(&self, reference: &str, file: &str, from: Option<&str>) -> u64 {
        let mut held = self.held();
        if let Some(already) = held
            .iter_mut()
            .find(|held| held.reference == reference && held.file == file)
        {
            if already.state.under_way() {
                return already.id;
            }
            already.stop.clear();
            already.state = State::Queued;
            already.why = None;
            already.asked_at = Timestamp::now();
            return already.id;
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        held.push(Transfer {
            id,
            reference: reference.to_owned(),
            file: file.to_owned(),
            from: from.map(str::to_owned),
            part: 0,
            parts: 0,
            arrived: 0,
            whole: 0,
            state: State::Queued,
            why: None,
            asked_at: Timestamp::now(),
            path: None,
            stop: Arc::new(Stop::default()),
        });
        id
    }

    /// Take the next transfer that should start, if there is room for it, and mark it as
    /// fetching so that nobody else takes the same one.
    ///
    /// Oldest first: a queue that started the newest arrival first would leave the thing
    /// asked for earliest waiting longest. Claiming and marking happen under one lock,
    /// because two workers that both read "queued" and both started would write to one
    /// partial file.
    pub fn claim_one(&self) -> Option<Start> {
        let mut held = self.held();
        let running = held
            .iter()
            .filter(|transfer| matches!(transfer.state, State::Fetching | State::Checking))
            .count();
        if running >= AT_ONCE {
            return None;
        }
        let transfer = held
            .iter_mut()
            .find(|transfer| transfer.state == State::Queued)?;
        transfer.state = State::Fetching;
        transfer.stop.clear();
        Some(Start {
            id: transfer.id,
            reference: transfer.reference.clone(),
            file: transfer.file.clone(),
            from: transfer.from.clone(),
            stopping: mcf_hub::stopping::Stopping::on(transfer.stop.watched()),
        })
    }

    /// Every transfer that should start now. Each wants a worker of its own.
    pub fn what_can_start(&self) -> Vec<Start> {
        let mut starting = Vec::new();
        while let Some(start) = self.claim_one() {
            starting.push(start);
        }
        starting
    }

    /// Pause one. It stops where it stands, and what arrived stays on disk.
    pub fn pause(&self, id: u64) -> bool {
        let mut held = self.held();
        let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id) else {
            return false;
        };
        if !transfer.state.under_way() {
            return false;
        }
        transfer.stop.ask(Asked::Pause);
        // A transfer that had not started yet has no worker to notice the asking, so it is
        // put straight into the state the worker would have put it in.
        if transfer.state == State::Queued {
            transfer.state = State::Paused;
            transfer.stop.clear();
        }
        true
    }

    /// Start a paused or settled one again, from wherever it got to.
    pub fn resume(&self, id: u64) -> bool {
        let mut held = self.held();
        let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id) else {
            return false;
        };
        if transfer.state.under_way() {
            return false;
        }
        transfer.stop.clear();
        transfer.state = State::Queued;
        transfer.why = None;
        true
    }

    /// Give one up. What had arrived is swept: a partial file nobody is going to finish is
    /// disk held for nothing, and leaving it would have the next ask for that file resume
    /// a transfer the operator said they did not want.
    pub fn give_up(&self, id: u64) -> Option<GivenUp> {
        let mut held = self.held();
        let transfer = held.iter_mut().find(|transfer| transfer.id == id)?;
        transfer.stop.ask(Asked::GiveUp);
        let running = matches!(transfer.state, State::Fetching | State::Checking);
        if !running {
            transfer.state = State::Cancelled;
            // Whatever it last said about itself is no longer why it is where it is. A
            // transfer given up while paused would otherwise keep the pause's words and
            // read as though it had stopped by itself.
            transfer.why = None;
        }
        Some(GivenUp {
            reference: transfer.reference.clone(),
            file: transfer.file.clone(),
            // A worker that is running does its own sweeping, on its way out. One that is
            // not has nobody to do it, so the caller is told to.
            sweep_it_here: !running,
        })
    }

    /// Forget every transfer that has nothing more to do, so that a queue read a week
    /// later is a list of what is happening rather than a history of what did.
    pub fn forget_the_settled(&self) -> usize {
        let mut held = self.held();
        let before = held.len();
        held.retain(|transfer| !transfer.state.settled());
        before.saturating_sub(held.len())
    }

    pub fn to_value(&self) -> Value {
        let held = self.held();
        Value::map([
            (
                "transfers",
                Value::List(held.iter().map(Transfer::to_value).collect()),
            ),
            (
                "at_once",
                Value::Integer(i64::try_from(AT_ONCE).unwrap_or(i64::MAX)),
            ),
            (
                "under_way",
                Value::Integer(
                    i64::try_from(
                        held.iter()
                            .filter(|transfer| transfer.state.under_way())
                            .count(),
                    )
                    .unwrap_or(i64::MAX),
                ),
            ),
        ])
    }

    /// Whether anything is still arriving. The daemon will not shut a transfer out from
    /// under itself without saying so.
    #[must_use]
    pub fn anything_under_way(&self) -> bool {
        self.held()
            .iter()
            .any(|transfer| transfer.state.under_way())
    }

    #[must_use]
    pub fn giving_up_on(&self, id: u64) -> bool {
        self.held()
            .iter()
            .find(|transfer| transfer.id == id)
            .is_some_and(|transfer| transfer.stop.giving_up())
    }

    /// Note what the repository said a file is made of, once the listing has been read.
    pub fn sized(&self, id: u64, parts: usize, whole: u64) {
        let mut held = self.held();
        if let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id) {
            transfer.parts = parts;
            transfer.whole = whole;
        }
    }

    /// Note how far a transfer has got. Called often, from the worker's own thread.
    pub fn moved(&self, id: u64, part: usize, arrived: u64) {
        let mut held = self.held();
        if let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id) {
            transfer.part = part;
            transfer.arrived = arrived;
        }
    }

    /// Back to fetching, for the next part of a file published in parts. Without this a
    /// multi-part file would read as "checking" from the moment its first part was whole
    /// until the last one finished.
    pub fn fetching(&self, id: u64) {
        let mut held = self.held();
        if let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id)
            && transfer.state == State::Checking
        {
            transfer.state = State::Fetching;
        }
    }

    pub fn checking(&self, id: u64) {
        let mut held = self.held();
        if let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id)
            && transfer.state == State::Fetching
        {
            transfer.state = State::Checking;
        }
    }

    pub fn arrived(&self, id: u64, at: &Path) {
        let mut held = self.held();
        if let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id) {
            transfer.state = State::Done;
            transfer.path = Some(at.to_path_buf());
            transfer.arrived = transfer.whole;
            transfer.why = None;
            transfer.stop.clear();
        }
    }

    /// A transfer that stopped. Which state it lands in is the failure's to say, not the
    /// caller's: a transfer interrupted on purpose is paused or given up, and anything
    /// else failed.
    pub fn stopped(&self, id: u64, failure: &Failure) {
        let interrupted = failure.category() == mcf_core::failure::Category::TransferInterrupted;
        let mut held = self.held();
        let Some(transfer) = held.iter_mut().find(|transfer| transfer.id == id) else {
            return;
        };
        transfer.state = if !interrupted {
            State::Failed
        } else if transfer.stop.giving_up() {
            State::Cancelled
        } else {
            State::Paused
        };
        transfer.why =
            (transfer.state != State::Cancelled).then(|| mcf_record::encode::failure(failure));
        transfer.stop.clear();
    }
}

/// The default hub, for a transfer that named none.
const DEFAULT_HUB: &str = "https://huggingface.co/";

/// Work one slot of the queue until there is nothing left to work.
///
/// A worker holds its place rather than handing it back and being spawned again: one
/// thread per place in the queue, for as long as the queue has anything in it.
pub fn work_a_place(queue: &Arc<Queue>, root: &Path, journal: &Path, first: Start) {
    let mut doing = Some(first);
    while let Some(start) = doing {
        carry_out(queue, &start, root, journal);
        doing = queue.claim_one();
    }
}

/// Bring one file here, part by part, stopping where it stands if it is asked to.
pub fn carry_out(queue: &Queue, start: &Start, root: &Path, journal: &Path) {
    let parsed = match mcf_hub::reference::parse(&start.reference) {
        Ok(parsed) => parsed,
        Err(failure) => return queue.stopped(start.id, &failure),
    };
    let base = match mcf_hub::http::Url::parse(start.from.as_deref().unwrap_or(DEFAULT_HUB)) {
        Ok(base) => base,
        Err(failure) => return queue.stopped(start.id, &failure),
    };
    let listing = match listing_of(&base, &parsed) {
        Ok(listing) => listing,
        Err(failure) => return queue.stopped(start.id, &failure),
    };
    let parts = match parts_of(&listing, &start.file) {
        Ok(parts) => parts,
        Err(failure) => return queue.stopped(start.id, &failure),
    };
    let whole: u64 = parts
        .iter()
        .map(|part| part.size)
        .fold(0, u64::saturating_add);
    queue.sized(start.id, parts.len(), whole);

    let mut before = 0_u64;
    let mut landed = None;
    for (index, part) in parts.iter().enumerate() {
        let at = index.saturating_add(1);
        match one_part(queue, start, &base, &listing, part, (at, before), root) {
            Ok(done) => {
                before = before.saturating_add(part.size);
                queue.moved(start.id, at, before);
                if landed.is_none() {
                    landed = Some(done.acquired.path.clone());
                }
            }
            Err(failure) => {
                if queue.giving_up_on(start.id) {
                    sweep_what_arrived(root, &listing, &parts);
                }
                queue.stopped(start.id, &failure);
                return;
            }
        }
    }
    match landed {
        Some(path) => {
            queue.arrived(start.id, &path);
            note_it_arrived(journal, start, whole, &path);
            ask_for_its_projector(
                queue,
                &start.reference,
                start.from.as_deref(),
                &listing,
                &start.file,
                root,
            );
        }
        None => queue.stopped(
            start.id,
            &crate::control::refused("a file this repository publishes", &start.file),
        ),
    }
}

/// Queue the projector a model's publisher ships beside it, where one does and it is not
/// here yet. A model that reads pictures, video or sound reads only text without it, and
/// nobody asking for such a model is asking for the half that cannot see.
///
/// Queued rather than fetched here, so it shows in the queue like any other file and can be
/// paused or given up like one; the worker that brought the model takes it next.
fn ask_for_its_projector(
    queue: &Queue,
    reference: &str,
    from: Option<&str>,
    listing: &mcf_hub::source::Listing,
    model: &str,
    root: &Path,
) {
    let Some(projector) = listing.projector_for(model) else {
        return;
    };
    let landing = mcf_hub::acquisition::destination(root, &listing.reference, &projector.path);
    if landing.is_file() {
        return;
    }
    let _id = queue.ask_for(reference, &projector.path, from);
}

/// Look for the projector of a model already on the shelf, fetched before MCF brought
/// projectors with their models, and queue it if its publisher ships one.
///
/// Where the model came from is read from the provenance written beside it when it
/// arrived; a model with none, or one that came from a local file, has nowhere to ask.
/// This asks the hub, so it is for a thread of its own, never for a request's.
pub fn seek_the_projector_of(queue: &Arc<Queue>, root: &Path, journal: &Path, model: &Path) {
    let Ok(provenance) = mcf_hub::store::provenance_of(model) else {
        return;
    };
    let mcf_core::provenance::Origin::Hub { repository, .. } = provenance.origin() else {
        return;
    };
    let Ok(reference) = mcf_hub::reference::parse(repository.as_str()) else {
        return;
    };
    let under = root.join(&reference.owner).join(&reference.name);
    let Some(file) = model
        .strip_prefix(&under)
        .ok()
        .and_then(Path::to_str)
        .map(str::to_owned)
    else {
        return;
    };
    let Ok(base) = mcf_hub::http::Url::parse(DEFAULT_HUB) else {
        return;
    };
    let Ok(listing) = listing_of(&base, &reference) else {
        return;
    };
    ask_for_its_projector(queue, repository.as_str(), None, &listing, &file, root);
    start_what_can_start(queue, root, journal);
}

/// Start whatever the queue says can start now, each on its own thread.
///
/// The queue decides; this only obeys. A transfer's thread is not scoped to whatever asked
/// for it, because a transfer outlives the connection that asked for it — that is the point
/// of queuing it.
pub fn start_what_can_start(queue: &Arc<Queue>, root: &Path, journal: &Path) {
    for start in queue.what_can_start() {
        let working = Arc::clone(queue);
        let (root_here, journal_here) = (root.to_path_buf(), journal.to_path_buf());
        let id = start.id;
        let spawned = std::thread::Builder::new()
            .name(format!("mcf-transfer-{id}"))
            .spawn(move || work_a_place(&working, &root_here, &journal_here, start));
        if let Err(error) = spawned {
            queue.stopped(
                id,
                &crate::control::refused("a thread for the transfer", &error.to_string()),
            );
        }
    }
}

fn listing_of(
    base: &mcf_hub::http::Url,
    reference: &mcf_hub::reference::Reference,
) -> Result<mcf_hub::source::Listing, Failure> {
    use mcf_hub::source::Source as _;
    let wire = mcf_hub::wire::for_url(base)?;
    mcf_hub::client::Hub::at(base.clone(), wire).list(reference)
}

fn parts_of(
    listing: &mcf_hub::source::Listing,
    file: &str,
) -> Result<Vec<mcf_hub::source::Entry>, Failure> {
    if listing.entry(file).is_none() {
        return Err(crate::control::refused(
            "a file this repository publishes",
            file,
        ));
    }
    match listing.parts_of(file) {
        Some(set) if !set.is_whole() => Err(crate::control::refused(
            "a model published in parts, not all of which this repository publishes",
            file,
        )
        .with_context("parts_found", set.parts.len().to_string())
        .with_context("parts_declared", set.of.to_string())),
        Some(set) => Ok(set.parts.into_iter().cloned().collect()),
        None => Ok(listing.entry(file).into_iter().cloned().collect()),
    }
}

/// Fetch one part, watching the partial file so the queue can say how far it has got.
///
/// The fetching itself runs on a thread of its own, because the only honest measure of
/// progress is the size of the file on disk, and reading that means being somewhere other
/// than inside the call that is writing it.
fn one_part(
    queue: &Queue,
    start: &Start,
    base: &mcf_hub::http::Url,
    listing: &mcf_hub::source::Listing,
    entry: &mcf_hub::source::Entry,
    (at, before): (usize, u64),
    root: &Path,
) -> Result<mcf_hub::acquisition::Done, Failure> {
    queue.fetching(start.id);
    let arriving = mcf_hub::acquisition::arriving_at(root, listing, entry);
    let landing = mcf_hub::acquisition::destination(root, &listing.reference, &entry.path);
    let (listing, entry_here) = (listing.clone(), entry.clone());
    let (where_from, into) = (base.clone(), root.to_path_buf());
    let stopping = start.stopping.clone();
    let fetching = std::thread::spawn(move || {
        let wire = mcf_hub::wire::for_url(&where_from)?;
        let hub = mcf_hub::client::Hub::at(where_from, wire).stopped_by(stopping.clone());
        mcf_hub::acquisition::one(&hub, &listing, &entry_here, &into, &stopping)
    });

    let mut furthest = 0_u64;
    while !fetching.is_finished() {
        std::thread::sleep(WATCHING);
        let reached = std::fs::metadata(&arriving)
            .or_else(|_| std::fs::metadata(&landing))
            .map_or(0, |about| about.len());
        furthest = furthest.max(reached);
        queue.moved(start.id, at, before.saturating_add(furthest));
        if furthest >= entry.size && entry.size > 0 {
            queue.checking(start.id);
        }
    }

    match fetching.join() {
        Ok(done) => done,
        Err(_) => Err(crate::control::refused(
            "the transfer stopped without saying why",
            &entry.path,
        )),
    }
}

/// How often the partial file is measured while a part arrives.
const WATCHING: std::time::Duration = std::time::Duration::from_millis(700);

/// Sweep what a given-up transfer had written. A partial file nobody is going to finish is
/// disk held for nothing, and leaving it would have the next ask for that file quietly
/// continue a transfer the operator said they did not want.
fn sweep_what_arrived(
    root: &Path,
    listing: &mcf_hub::source::Listing,
    parts: &[mcf_hub::source::Entry],
) {
    for part in parts {
        let _swept = std::fs::remove_file(mcf_hub::acquisition::arriving_at(root, listing, part));
    }
}

/// Sweep the partial file of a transfer given up before any worker was running on it.
///
/// A transfer that was paused and then given up has no worker to do the sweeping, and the
/// bytes it had written would otherwise sit on the disk with nothing ever going to finish
/// them — and would be silently continued by the next ask for that file, which is the one
/// thing somebody who gave it up did not want. What the file is made of cannot be known
/// without asking the hub, so this sweeps the one part that was named; a multi-part file
/// given up while running is swept whole by the worker that was carrying it.
pub fn sweep_what_is_not_running(root: &Path, reference: &str, file: &str) {
    let Ok(parsed) = mcf_hub::reference::parse(reference) else {
        return;
    };
    let landing = mcf_hub::acquisition::destination(root, &parsed, file);
    let _swept = std::fs::remove_file(mcf_hub::fetch::partial_path(&landing));
}

fn note_it_arrived(journal: &Path, start: &Start, whole: u64, path: &Path) {
    let Ok(mut held) = mcf_record::journal::Journal::open(journal) else {
        return;
    };
    let _appended = held.append(&mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::ArtifactAcquired,
        Timestamp::now(),
        Value::map([
            ("repository", Value::text(start.reference.clone())),
            ("file", Value::text(start.file.clone())),
            ("path", Value::text(path.display().to_string())),
            (
                "bytes",
                Value::Integer(i64::try_from(whole).unwrap_or(i64::MAX)),
            ),
            ("queued", Value::Bool(true)),
        ]),
    ));
}

#[cfg(test)]
mod tests;
