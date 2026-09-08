use crate::machine::Reading;
use crate::screen::{Ink, Screen};
use crate::screens::{UNKNOWN, columns, gigabytes, or_unknown};

#[derive(Debug, Clone)]
pub enum Doing {
    Idle,
    Serving {
        model: String,
        engine: String,
        device: String,
        context: u64,
        uptime: u64,
    },
}

fn clock(seconds: u64) -> String {
    #[allow(clippy::integer_division, reason = "seconds into hours and minutes")]
    {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }
}

pub fn draw(into: &mut Screen, from: usize, reading: &Reading, doing: &Doing) {
    let mut row = processors(into, from + 1, reading);
    row = memory(into, row + 1, reading);
    storage(into, row + 1, reading);
    footer(into, doing);
}

fn processors(into: &mut Screen, from: usize, reading: &Reading) -> usize {
    let mut row = from;

    let head = [
        ("PROCESSOR", 17, false, Ink::Heading),
        ("LOAD", 8, true, Ink::Quiet),
        ("TEMP", 9, true, Ink::Quiet),
        ("POWER", 8, true, Ink::Quiet),
        ("CLOCK", 11, true, Ink::Quiet),
        ("CORES", 9, true, Ink::Quiet),
    ];
    columns(into, 2, row, &head);
    row += 1;

    let processor = &reading.processor;
    let clock_text = processor.clock.map_or_else(
        || UNKNOWN.to_owned(),
        |mhz| {
            #[allow(
                clippy::integer_division,
                reason = "megahertz to gigahertz, to two places"
            )]
            {
                format!("{}.{:02} GHz", mhz / 1000, (mhz % 1000) / 10)
            }
        },
    );
    columns(
        into,
        2,
        row,
        &[
            ("CPU", 17, false, Ink::Plain),
            (
                &or_unknown(
                    processor.load.map(super::super::machine::Tenths::whole),
                    " %",
                ),
                8,
                true,
                Ink::Plain,
            ),
            (
                &or_unknown(processor.temperature, " °C"),
                9,
                true,
                Ink::Plain,
            ),
            (UNKNOWN, 8, true, Ink::Quiet),
            (&clock_text, 11, true, Ink::Plain),
            (&or_unknown(processor.cores, ""), 9, true, Ink::Plain),
        ],
    );
    row += 1;

    for card in &reading.cards {
        let name = format!("GPU  {}", card.name);
        let short: String = name.chars().take(17).collect();
        columns(
            into,
            2,
            row,
            &[
                (&short, 17, false, Ink::Plain),
                (
                    &or_unknown(card.load.map(super::super::machine::Tenths::whole), " %"),
                    8,
                    true,
                    Ink::Plain,
                ),
                (&or_unknown(card.temperature, " °C"), 9, true, Ink::Plain),
                (&or_unknown(card.power, " W"), 8, true, Ink::Plain),
                (UNKNOWN, 11, true, Ink::Quiet),
                (UNKNOWN, 9, true, Ink::Quiet),
            ],
        );
        row += 1;
    }
    row
}

fn memory(into: &mut Screen, from: usize, reading: &Reading) -> usize {
    let mut row = from;
    columns(
        into,
        2,
        row,
        &[
            ("MEMORY", 17, false, Ink::Heading),
            ("USED", 10, true, Ink::Quiet),
            ("TOTAL", 10, true, Ink::Quiet),
            ("FREE", 10, true, Ink::Quiet),
        ],
    );
    row += 1;
    let size = |held: Option<u64>| held.map_or_else(|| UNKNOWN.to_owned(), gigabytes);
    columns(
        into,
        2,
        row,
        &[
            ("System", 17, false, Ink::Plain),
            (&size(reading.memory.used()), 10, true, Ink::Plain),
            (&size(reading.memory.total), 10, true, Ink::Plain),
            (&size(reading.memory.available), 10, true, Ink::Plain),
        ],
    );
    row += 1;
    for card in &reading.cards {
        let free = card
            .total
            .and_then(|total| total.checked_sub(card.used.unwrap_or(0)));
        columns(
            into,
            2,
            row,
            &[
                ("Graphics", 17, false, Ink::Plain),
                (&size(card.used), 10, true, Ink::Plain),
                (&size(card.total), 10, true, Ink::Plain),
                (&size(free), 10, true, Ink::Plain),
            ],
        );
        row += 1;
    }
    row
}

fn storage(into: &mut Screen, from: usize, reading: &Reading) {
    let mut row = from;
    columns(
        into,
        2,
        row,
        &[
            ("STORAGE", 17, false, Ink::Heading),
            ("READ", 12, true, Ink::Quiet),
            ("WRITE", 12, true, Ink::Quiet),
            ("TEMP", 10, true, Ink::Quiet),
        ],
    );
    row += 1;
    let rate = |held: Option<u64>| {
        held.map_or_else(
            || UNKNOWN.to_owned(),
            |bytes| {
                #[allow(clippy::integer_division, reason = "bytes per second to megabytes")]
                {
                    format!(
                        "{}.{} MB/s",
                        bytes / 1_000_000,
                        (bytes % 1_000_000) / 100_000
                    )
                }
            },
        )
    };
    for disk in reading.disks.iter().take(2) {
        columns(
            into,
            2,
            row,
            &[
                (&disk.name, 17, false, Ink::Plain),
                (&rate(disk.read), 12, true, Ink::Plain),
                (&rate(disk.written), 12, true, Ink::Plain),
                (&or_unknown(disk.temperature, " °C"), 10, true, Ink::Plain),
            ],
        );
        row += 1;
    }
}

fn footer(into: &mut Screen, doing: &Doing) {
    let last = into.height().saturating_sub(1);
    let divider = last.saturating_sub(7);
    if divider > 3 {
        into.put(0, divider, "├", Ink::Quiet);
        into.rule(1, divider, into.width().saturating_sub(2), Ink::Quiet);
        into.put(into.width().saturating_sub(1), divider, "┤", Ink::Quiet);
    }
    let mut row = divider + 2;
    match doing {
        Doing::Idle => {
            into.put(2, row, "IDLE", Ink::Heading);
            into.put(
                10,
                row,
                "nothing is being served — Models holds a model here",
                Ink::Quiet,
            );
        }
        Doing::Serving {
            model,
            engine,
            device,
            context,
            uptime,
        } => {
            into.put(2, row, "SERVING", Ink::Heading);
            into.put(11, row, model, Ink::Plain);
            row += 2;
            columns(
                into,
                2,
                row,
                &[
                    ("ENGINE", 22, false, Ink::Quiet),
                    ("DEVICE", 24, false, Ink::Quiet),
                    ("CONTEXT", 12, true, Ink::Quiet),
                    ("UPTIME", 12, true, Ink::Quiet),
                ],
            );
            row += 1;
            let short: String = device.chars().take(23).collect();
            columns(
                into,
                2,
                row,
                &[
                    (engine, 22, false, Ink::Plain),
                    (&short, 24, false, Ink::Plain),
                    (&context.to_string(), 12, true, Ink::Plain),
                    (&clock(*uptime), 12, true, Ink::Held),
                ],
            );
        }
    }
}
