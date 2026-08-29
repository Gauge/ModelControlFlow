//! Splitting work across processors without changing the answer (B-366, D38,
//! §3.12, D19).
//!
//! **The hazard this module exists to remove.** Floating-point addition is not
//! associative: `(a + b) + c` and `a + (b + c)` differ in the last bits, and a
//! reduction whose *order* depends on how many threads happened to be free is a
//! reduction whose answer depends on how busy the machine was. That is the one
//! place in this crate where reproducibility can be lost with nobody noticing —
//! the output is the right shape, the text is fluent, and two runs of the same
//! command disagree. §3.12 makes reproducibility a precedence rule and D19
//! makes a run's conditions the thing that lets somebody else get the same
//! answer; a thread count that reaches the arithmetic breaks both.
//!
//! **The rule, and it is structural rather than careful.** *A reduction is
//! never split.* Work is partitioned by **output index** — one output element
//! is computed start to finish by exactly one thread, in the same order a
//! single thread would use — so the partition decides only *who* computes an
//! element and never *how*. Bit-identity across thread counts is then a
//! property of the shape of this module, not a property somebody has to keep
//! remembering, and [`crate::ops`] holds the one function that both the serial
//! and the partitioned path call.
//!
//! `checks/tests/a_reduction_is_never_split.rs` holds that shape from the
//! outside, and `tests/threads_do_not_change_the_answer.rs` asserts the
//! consequence over generated inputs.
//!
//! **A thread count is stated, never assumed.** [`Threads`] has no `Default`
//! and no way to exist without saying where its number came from — the same
//! shape B-281 gave sampling, and for the same reason: a number nobody chose is
//! a number nobody can defend. One thread is the *definition*, which is what a
//! model loads at until a caller asks for more.
//!
//! **What threads do change is the timing, and that is measured elsewhere.**
//! F52 measured thread count moving a benchmark's noise by a factor of five, in
//! the direction opposite to the one predicted, and is the standing evidence
//! that the effect of threads on a timing must be measured rather than reasoned
//! about. This crate cannot report a speed at all (B65), so nothing here
//! claims one.

use std::num::NonZeroUsize;
use std::sync::{Mutex, PoisonError};
use std::thread;

/// How many threads a partitioned computation may use, and where the number
/// came from.
///
/// The origin travels with the count because the two questions a reader has are
/// *how many* and *who decided*, and a bare integer answers only the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Threads {
    /// How many.
    count: NonZeroUsize,
    /// Whose number it is.
    origin: Origin,
}

/// Where a thread count came from.
///
/// Four states and no fifth, in the shape A21 uses for a model's declarations:
/// a number MCF was given, a number the machine reported, a number MCF fell
/// back to because the machine would not say, and the definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// One thread: the serial definition every other count must agree with.
    Definition,
    /// A caller said so.
    Stated,
    /// What `available_parallelism` reported on this machine.
    MachineReported,
    /// The machine would not say, so the definition was used.
    ///
    /// Distinct from [`Self::Definition`] on purpose: *nobody asked for one
    /// thread* and *MCF could not find out* are different facts about a run,
    /// and A7 forbids rendering the second as the first.
    MachineUnreadable,
}

impl Threads {
    /// One thread — the definition.
    #[must_use]
    pub const fn definition() -> Self {
        Self {
            count: NonZeroUsize::MIN,
            origin: Origin::Definition,
        }
    }

    /// A count a caller stated.
    ///
    /// Zero is not a thread count, so it becomes one thread with the origin
    /// kept as stated: the caller did ask, and got the smallest answer that is
    /// a number of threads.
    #[must_use]
    pub const fn stated(count: usize) -> Self {
        let Some(count) = NonZeroUsize::new(count) else {
            return Self {
                count: NonZeroUsize::MIN,
                origin: Origin::Stated,
            };
        };
        Self {
            count,
            origin: Origin::Stated,
        }
    }

    /// What this machine reports it can run at once.
    ///
    /// An observation rather than a policy: MCF reads the number the platform
    /// publishes and says that is where it came from. A machine that will not
    /// answer produces one thread marked [`Origin::MachineUnreadable`], because
    /// A7 wants the unknown recorded as unknown rather than dressed as a choice.
    #[must_use]
    pub fn what_the_machine_reports() -> Self {
        thread::available_parallelism().map_or(
            Self {
                count: NonZeroUsize::MIN,
                origin: Origin::MachineUnreadable,
            },
            |count| Self {
                count,
                origin: Origin::MachineReported,
            },
        )
    }

    /// How many threads.
    #[must_use]
    pub const fn count(self) -> usize {
        self.count.get()
    }

    /// Whose number it is.
    #[must_use]
    pub const fn origin(self) -> Origin {
        self.origin
    }

    /// One sentence a surface can print beside a run.
    ///
    /// Written once here rather than at each surface: two surfaces rendering
    /// the same condition differently are two answers to one question (A6).
    #[must_use]
    pub fn describe(self) -> String {
        let count = self.count.get();
        match self.origin {
            Origin::Definition => "1 thread — the serial definition".to_owned(),
            Origin::Stated => format!("{count} threads — asked for"),
            Origin::MachineReported => {
                format!("{count} threads — what this machine reports it can run at once")
            }
            Origin::MachineUnreadable => {
                "1 thread — this machine does not report how many it can run at once".to_owned()
            }
        }
    }
}

/// Computes each row of an output buffer, partitioning the *rows* across
/// threads.
///
/// `compute` is handed a row index and that row's slice of `out`, and is the
/// whole of the work for that row. Nothing accumulates across rows and no row
/// is seen by two threads, which is what makes the result identical whatever
/// `threads` says — the partition chooses *who* computes a row, never *how*.
///
/// **Which thread takes which chunk is deliberately not fixed.** Fixing it
/// would be a promise about scheduling that this code cannot keep — a thread
/// the operating system declines to give MCF would leave its rows uncomputed —
/// and it is not needed for the property that matters: a row is computed by one
/// thread, start to finish, in the order a single thread would use. So the
/// chunks are a queue, every worker takes the next one, and the calling thread
/// works alongside them rather than waiting. A worker MCF could not obtain is
/// then a slower run and never a wrong one (A2).
///
/// The chunks themselves are contiguous and by index — rows `0..k`, then
/// `k..2k` — so the mapping from row to chunk is something a reader can state
/// in one sentence.
///
/// `out.len()` must be a whole number of `width`-sized rows; a buffer that is
/// not is computed serially rather than partitioned, since a partition of a
/// shape that does not exist is the wrong thing to guess at.
pub fn each_row<F>(out: &mut [f32], width: usize, threads: Threads, compute: &F)
where
    F: Fn(usize, &mut [f32]) + Sync,
{
    let rows = out.len().checked_div(width).unwrap_or(0);
    if width == 0 || rows.saturating_mul(width) != out.len() {
        run_chunk(0, out, width.max(1), compute);
        return;
    }
    if threads.count() <= 1 || rows <= 1 {
        run_chunk(0, out, width, compute);
        return;
    }

    let per_chunk = rows.div_ceil(threads.count()).max(1);
    let stride = per_chunk.saturating_mul(width);
    let mut queue: Vec<(usize, &mut [f32])> = out
        .chunks_mut(stride)
        .enumerate()
        .map(|(index, slice)| (index.saturating_mul(per_chunk), slice))
        .collect();
    // Taken from the back, which costs nothing and is the only order a `Vec`
    // gives cheaply. It changes who computes what and therefore nothing.
    queue.reverse();
    // No more workers than there is work: ninety-seven threads for two chunks is
    // ninety-five starts that find an empty queue, and starting a thread is not
    // free.
    let workers = threads.count().min(queue.len());
    let queue = Mutex::new(queue);

    thread::scope(|scope| {
        for _ in 1..workers {
            let worker = || drain(&queue, width, compute);
            match thread::Builder::new().spawn_scoped(scope, worker) {
                Ok(handle) => drop(handle),
                // The chunks this worker would have taken stay in the queue and
                // are computed by somebody who did start.
                Err(_refused) => {}
            }
        }
        drain(&queue, width, compute);
    });
}

/// Takes chunks until there are none, computing each one's rows in order.
fn drain<F>(queue: &Mutex<Vec<(usize, &mut [f32])>>, width: usize, compute: &F)
where
    F: Fn(usize, &mut [f32]) + Sync,
{
    loop {
        // A poisoned lock means a worker stopped in the middle of a chunk, which
        // is a defect elsewhere; the remaining chunks are still owed to the
        // caller, so the queue is taken back rather than the work abandoned.
        let taken = queue.lock().unwrap_or_else(PoisonError::into_inner).pop();
        let Some((first, slice)) = taken else {
            return;
        };
        run_chunk(first, slice, width, compute);
    }
}

/// One chunk's rows, in index order — the serial definition, which is also what
/// every partitioned path calls.
fn run_chunk<F>(first: usize, slice: &mut [f32], width: usize, compute: &F)
where
    F: Fn(usize, &mut [f32]) + Sync,
{
    for (offset, row) in slice.chunks_mut(width).enumerate() {
        compute(first.saturating_add(offset), row);
    }
}

#[cfg(test)]
mod tests;
