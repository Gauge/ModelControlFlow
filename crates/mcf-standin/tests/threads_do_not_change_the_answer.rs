#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::integer_division
)]

use mcf_standin::ops::{matmul_vec, matmul_vec_across};
use mcf_standin::threads::Threads;

const COUNTS: [usize; 6] = [1, 2, 3, 5, 32, 97];

struct Noise(u64);

impl Noise {
    const fn seeded(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let bits = u32::try_from(self.0 >> 40).unwrap_or(0);
        let half = u16::try_from(bits & 0xFFFF).unwrap_or(0);
        let unit = (f32::from(half) - 32_768.0) / 32_768.0;
        let decade = match (bits >> 16) % 4 {
            0 => 0.001,
            1 => 1.0,
            2 => 1_000.0,
            _ => 1_000_000.0,
        };
        unit * decade
    }

    fn values(&mut self, count: usize) -> Vec<f32> {
        (0..count).map(|_| self.next()).collect()
    }
}

const SHAPES: [(usize, usize); 13] = [
    (0, 4),
    (1, 1),
    (1, 64),
    (2, 3),
    (3, 1),
    (7, 13),
    (13, 7),
    (64, 64),
    (257, 31),
    (1_009, 127),
    (2_003, 97),
    (523, 631),
    (4_001, 257),
];

fn is_partitioned(rows: usize, columns: usize) -> bool {
    Threads::stated(32).worth_starting(rows * columns) > 1
}

#[test]
fn a_product_is_the_same_bytes_at_every_thread_count() {
    let partitioned = SHAPES
        .iter()
        .filter(|(rows, columns)| is_partitioned(*rows, *columns))
        .count();
    assert!(
        partitioned >= 4,
        "only {partitioned} of these shapes is large enough for the engine to partition at \
         all (F99's rule), so this file is mostly comparing the serial path with itself"
    );

    for (seed, (rows, columns)) in SHAPES.iter().copied().enumerate() {
        let mut noise = Noise::seeded(seed as u64 + 1);
        let matrix = noise.values(rows * columns);
        let vector = noise.values(columns);

        let definition = matmul_vec(&matrix, &vector, rows, columns);
        assert_eq!(
            definition.len(),
            rows,
            "the serial definition did not produce {rows} rows"
        );

        for count in COUNTS {
            let produced =
                matmul_vec_across(&matrix, &vector, rows, columns, Threads::stated(count));
            assert_bits(
                &produced,
                &definition,
                &format!("{rows}×{columns} at {count} threads"),
            );
        }

        let machine = matmul_vec_across(
            &matrix,
            &vector,
            rows,
            columns,
            Threads::what_the_machine_reports(),
        );
        assert_bits(
            &machine,
            &definition,
            &format!("{rows}×{columns} at what the machine reports"),
        );
    }
}

#[test]
fn a_partitioned_product_is_the_same_bytes_every_time_it_is_run() {
    let mut noise = Noise::seeded(99);
    let (rows, columns) = (1_009, 127);
    assert!(
        is_partitioned(rows, columns),
        "this repetition test no longer partitions anything"
    );
    let matrix = noise.values(rows * columns);
    let vector = noise.values(columns);
    let first = matmul_vec_across(&matrix, &vector, rows, columns, Threads::stated(16));
    for repeat in 0..32 {
        let again = matmul_vec_across(&matrix, &vector, rows, columns, Threads::stated(16));
        assert_bits(&again, &first, &format!("repeat {repeat}"));
    }
}

#[test]
fn a_shape_that_disagrees_produces_nothing_at_every_thread_count() {
    let matrix = [1.0_f32, 2.0, 3.0, 4.0];
    let vector = [1.0_f32, 1.0];
    for count in COUNTS {
        assert!(
            matmul_vec_across(&matrix, &vector, 3, 2, Threads::stated(count)).is_empty(),
            "a matrix too short for its stated rows produced a rectangle at {count} threads"
        );
        assert!(
            matmul_vec_across(&matrix, &vector, 2, 3, Threads::stated(count)).is_empty(),
            "a vector that is not the column count produced a product at {count} threads"
        );
    }
}

#[test]
fn splitting_a_sum_does_change_the_bytes() {
    let mut noise = Noise::seeded(7);
    let rows = 523;
    let columns = 631;
    let matrix = noise.values(rows * columns);
    let vector = noise.values(columns);

    let definition = matmul_vec(&matrix, &vector, rows, columns);
    let split = split_reduction(&matrix, &vector, rows, columns);

    let differing = definition
        .iter()
        .zip(split.iter())
        .filter(|(one, other)| one.to_bits() != other.to_bits())
        .count();
    assert!(
        differing > 0,
        "summing each row in two halves gave the same bytes as summing it in one pass, so \
         these inputs cannot detect a split reduction and this file proves nothing"
    );
}

fn split_reduction(matrix: &[f32], vector: &[f32], rows: usize, columns: usize) -> Vec<f32> {
    let middle = columns / 2;
    (0..rows)
        .map(|row| {
            let start = row * columns;
            let slice = &matrix[start..start + columns];
            let mut left = 0.0_f32;
            for (weight, value) in slice[..middle].iter().zip(vector[..middle].iter()) {
                left = weight.mul_add(*value, left);
            }
            let mut right = 0.0_f32;
            for (weight, value) in slice[middle..].iter().zip(vector[middle..].iter()) {
                right = weight.mul_add(*value, right);
            }
            left + right
        })
        .collect()
}

fn assert_bits(produced: &[f32], expected: &[f32], what: &str) {
    assert_eq!(produced.len(), expected.len(), "{what}: different lengths");
    for (index, (one, other)) in produced.iter().zip(expected.iter()).enumerate() {
        assert!(
            one.to_bits() == other.to_bits(),
            "{what}: element {index} is {one} ({:#010x}) and the serial definition says {other} \
             ({:#010x})",
            one.to_bits(),
            other.to_bits()
        );
    }
}
