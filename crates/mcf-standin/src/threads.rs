use std::num::NonZeroUsize;
use std::sync::{Mutex, PoisonError};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Threads {
    count: NonZeroUsize,
    origin: Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Definition,
    Stated,
    MachineReported,
    MachineUnreadable,
}

impl Threads {
    #[must_use]
    pub const fn definition() -> Self {
        Self {
            count: NonZeroUsize::MIN,
            origin: Origin::Definition,
        }
    }

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

    #[must_use]
    pub const fn count(self) -> usize {
        self.count.get()
    }

    #[must_use]
    pub const fn origin(self) -> Origin {
        self.origin
    }

    #[must_use]
    pub fn worth_starting(self, work: usize) -> usize {
        let earned = work.checked_div(WORTH_A_WORKER).unwrap_or(0);
        self.count().min(earned.max(1))
    }

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

pub const WORTH_A_WORKER: usize = 50_000;

pub fn each_row<F>(
    out: &mut [f32],
    width: usize,
    work_per_row: usize,
    threads: Threads,
    compute: &F,
) where
    F: Fn(usize, &mut [f32]) + Sync,
{
    let rows = out.len().checked_div(width).unwrap_or(0);
    if width == 0 || rows.saturating_mul(width) != out.len() {
        run_chunk(0, out, width.max(1), compute);
        return;
    }
    let workers = threads.worth_starting(rows.saturating_mul(work_per_row));
    if workers <= 1 || rows <= 1 {
        run_chunk(0, out, width, compute);
        return;
    }

    let per_chunk = rows.div_ceil(workers).max(1);
    let stride = per_chunk.saturating_mul(width);
    let mut queue: Vec<(usize, &mut [f32])> = out
        .chunks_mut(stride)
        .enumerate()
        .map(|(index, slice)| (index.saturating_mul(per_chunk), slice))
        .collect();
    queue.reverse();
    let workers = workers.min(queue.len());
    let queue = Mutex::new(queue);

    thread::scope(|scope| {
        for _ in 1..workers {
            let worker = || drain(&queue, width, compute);
            match thread::Builder::new().spawn_scoped(scope, worker) {
                Ok(handle) => drop(handle),
                Err(_refused) => {}
            }
        }
        drain(&queue, width, compute);
    });
}

fn drain<F>(queue: &Mutex<Vec<(usize, &mut [f32])>>, width: usize, compute: &F)
where
    F: Fn(usize, &mut [f32]) + Sync,
{
    loop {
        let taken = queue.lock().unwrap_or_else(PoisonError::into_inner).pop();
        let Some((first, slice)) = taken else {
            return;
        };
        run_chunk(first, slice, width, compute);
    }
}

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
