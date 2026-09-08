use mcf_record::json::Value;

use super::paraphrase::answer_in;
use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "arithmetic";

pub const DIGITS: [u32; 6] = [2, 4, 6, 8, 10, 12];

pub const EACH: usize = 3;

const BUDGET: usize = 160;

fn operand(seed: &mut u64, digits: u32) -> i64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let span = 10_i64.saturating_pow(digits.saturating_sub(1));
    let below = i64::try_from((*seed >> 20) % u64::try_from(span.saturating_mul(9)).unwrap_or(9))
        .unwrap_or(0);
    span.saturating_add(below)
}

#[must_use]
pub fn sums() -> Vec<(&'static str, u32, i64, i64, i64)> {
    let mut seed = 20_260_906_u64;
    let mut out = Vec::new();
    for digits in DIGITS {
        for _ in 0..EACH {
            let (a, b) = (operand(&mut seed, digits), operand(&mut seed, digits));
            out.push(("add", digits, a, b, a.saturating_add(b)));
        }
        for _ in 0..EACH {
            let (a, b) = (operand(&mut seed, digits), operand(&mut seed, digits));
            let (high, low) = (a.max(b), a.min(b));
            out.push(("subtract", digits, high, low, high - low));
        }
        let factor_digits = digits.min(9);
        for _ in 0..EACH {
            let (a, b) = (
                operand(&mut seed, factor_digits),
                operand(&mut seed, factor_digits),
            );
            out.push(("multiply", digits, a, b, a.saturating_mul(b)));
        }
    }
    out
}

#[must_use]
pub fn ask_for(operation: &str, a: i64, b: i64) -> String {
    let words = match operation {
        "add" => format!("What is {a} plus {b}?"),
        "subtract" => format!("What is {a} minus {b}?"),
        _ => format!("What is {a} times {b}?"),
    };
    format!("{words} Answer with the number only.")
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each sum a row set, the count a digit count"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let put = sums();
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} sum(s): {EACH} each of adding, subtracting and multiplying at {} digits, greedy; \
         the reply's number read as the paraphrase measurement reads it",
        put.len(),
        DIGITS
            .iter()
            .map(u32::to_string)
            .collect::<Vec<String>>()
            .join(", ")
    )];
    let mut by_digits: std::collections::BTreeMap<(u32, &str), (usize, usize)> =
        std::collections::BTreeMap::new();
    for (which, (operation, digits, a, b, answer)) in put.iter().enumerate() {
        site.progress(which, put.len(), &format!("{operation} at {digits} digits"));
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let ids = match framed_ids(&engine, &ask_for(operation, *a, *b)) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        let completed = match engine.complete(
            Prompt::Identifiers(&ids),
            BUDGET,
            Draw::greedy(0),
            false,
            site.timed(),
        ) {
            Ok(completed) => completed,
            Err(failure) => return Found::could_not_tell(failure.detail()),
        };
        let read = answer_in(&completed.text);
        let was_right = read == Some(*answer);
        let tally = by_digits.entry((*digits, operation)).or_insert((0, 0));
        tally.0 = tally.0.saturating_add(usize::from(was_right));
        tally.1 = tally.1.saturating_add(1);
        let dims = [
            ("operation", Value::text(*operation)),
            ("digits", Value::Integer(i64::from(*digits))),
            ("sum", Value::Integer(as_integer(which))),
        ];
        rows.push(Reading::new(&dims, "right", i64::from(was_right), "bool"));
        rows.push(Reading::new(
            &dims,
            "read",
            i64::from(read.is_some()),
            "bool",
        ));
        if let Some(read) = read {
            rows.push(Reading::new(&dims, "answer", read, "count"));
        }
        rows.push(Reading::new(&dims, "expected", *answer, "count"));
        rows.push(Reading::new(
            &dims,
            "tokens",
            as_integer(completed.predicted),
            "tokens",
        ));
    }
    let mut fields = vec![("sums", Value::Integer(as_integer(put.len())))];
    let mut right_all = 0_usize;
    for digits in DIGITS {
        let said: Vec<String> = ["add", "subtract", "multiply"]
            .iter()
            .map(|operation| {
                let (right, of) = by_digits
                    .get(&(digits, operation))
                    .copied()
                    .unwrap_or((0, 0));
                right_all = right_all.saturating_add(right);
                rows.push(Reading::new(
                    &[
                        ("operation", Value::text(*operation)),
                        ("digits", Value::Integer(i64::from(digits))),
                    ],
                    "right",
                    as_integer(right),
                    "count",
                ));
                format!("{operation} {right}/{of}")
            })
            .collect();
        lines.push(format!("  {digits:>2} digit(s)   {}", said.join("   ")));
    }
    for operation in ["add", "subtract", "multiply"] {
        let widest = DIGITS
            .iter()
            .filter(|digits| {
                by_digits
                    .get(&(**digits, operation))
                    .is_some_and(|(right, of)| right == of && *of > 0)
            })
            .copied()
            .max();
        fields.push((
            match operation {
                "add" => "add_holds_to",
                "subtract" => "subtract_holds_to",
                _ => "multiply_holds_to",
            },
            widest.map_or(Value::Null, |digits| Value::Integer(i64::from(digits))),
        ));
    }
    fields.push(("right", Value::Integer(as_integer(right_all))));
    lines.push(format!("  {right_all} of {} right", put.len()));
    Found {
        lines,
        fields,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{DIGITS, EACH, ask_for, sums};

    #[test]
    fn the_sums_are_fixed_and_their_answers_hold() {
        let put = sums();
        assert_eq!(put.len(), DIGITS.len() * 3 * EACH);
        assert_eq!(put, sums(), "the same sums every run");
        for (operation, digits, a, b, answer) in &put {
            let width = u32::try_from(a.to_string().len()).unwrap_or(0);
            match *operation {
                "add" => assert_eq!(*answer, a + b),
                "subtract" => {
                    assert_eq!(*answer, a - b);
                    assert!(answer >= &0);
                }
                _ => assert_eq!(*answer, a * b),
            }
            if *operation != "multiply" {
                assert_eq!(width, *digits, "{operation} {a}");
            }
        }
        assert!(ask_for("add", 12, 34).starts_with("What is 12 plus 34?"));
    }
}
