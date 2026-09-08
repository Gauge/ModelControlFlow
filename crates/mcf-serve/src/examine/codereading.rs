use mcf_record::json::Value;

use super::paraphrase::answer_in;
use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "code-reading";

const BUDGET: usize = 400;

pub const OUTPUTS: &[(&str, &str, i64)] = &[
    (
        "loop-sum",
        "total = 0\nfor i in range(1, 6):\n    if i % 2 == 0:\n        total += i * i\n    else:\n        total += i\nprint(total)",
        29,
    ),
    (
        "string-count",
        "s = 'abracadabra'\ncount = 0\nfor i, c in enumerate(s):\n    if c == 'a' and i % 2 == 0:\n        count += 1\nprint(count)",
        2,
    ),
    (
        "list-mutation",
        "xs = [3, 1, 4, 1, 5]\nys = xs\nys.append(9)\nxs[0] = 2\nprint(len(xs) + sum(ys))",
        28,
    ),
    (
        "recursion",
        "def f(n):\n    if n < 2:\n        return 1\n    return f(n - 1) + 2 * f(n - 2)\nprint(f(6))",
        43,
    ),
    (
        "dict-update",
        "d = {'a': 1, 'b': 2}\nfor k in list(d):\n    d[k + k] = d[k] * 10\nprint(sum(d.values()))",
        33,
    ),
    (
        "while-halving",
        "n = 100\nsteps = 0\nwhile n > 1:\n    n = n // 2 if n % 2 == 0 else n - 1\n    steps += 1\nprint(steps)",
        8,
    ),
];

pub const BUGS: &[(&str, &str, i64, &str)] = &[
    (
        "off-by-one",
        "def last_three(xs):\n    out = []\n    for i in range(len(xs) - 3, len(xs) - 1):\n        out.append(xs[i])\n    return out",
        3,
        "return the last three items of a list",
    ),
    (
        "wrong-comparison",
        "def is_adult(age):\n    if age > 18:\n        return True\n    return False",
        2,
        "return True for anyone aged 18 or over",
    ),
    (
        "lost-accumulator",
        "def total(xs):\n    result = 0\n    for x in xs:\n        result = x\n    return result",
        4,
        "return the sum of a list",
    ),
    (
        "swapped-return",
        "def clamp(x, lo, hi):\n    if x < lo:\n        return hi\n    if x > hi:\n        return hi\n    return x",
        3,
        "keep x between lo and hi inclusive",
    ),
    (
        "integer-division",
        "def average(xs):\n    if not xs:\n        return 0.0\n    return sum(xs) // len(xs)",
        4,
        "return the mean of a list as a float",
    ),
];

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each program a row set, outputs then bugs"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} program(s) whose printed number is to be predicted and {} with one wrong line to \
         name; greedy; both read as numbers",
        OUTPUTS.len(),
        BUGS.len()
    )];
    let ask = |text: &str| -> Result<(String, usize), String> {
        let ids = framed_ids(&engine, text)?;
        let completed = engine
            .complete(
                Prompt::Identifiers(&ids),
                BUDGET,
                Draw::greedy(0),
                false,
                site.timed(),
            )
            .map_err(|failure| failure.detail().to_owned())?;
        Ok((completed.text, completed.predicted))
    };
    let (mut outputs_right, mut bugs_right) = (0_usize, 0_usize);
    for (at, (name, code, output)) in OUTPUTS.iter().enumerate() {
        site.progress(at, OUTPUTS.len().saturating_add(BUGS.len()), name);
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let asked = format!(
            "What does this Python program print? Answer with the number only.\n\n```python\n{code}\n```"
        );
        let (text, tokens) = match ask(&asked) {
            Ok(held) => held,
            Err(why) => return Found::could_not_tell(&why),
        };
        let read = answer_in(&text);
        let right = read == Some(*output);
        outputs_right = outputs_right.saturating_add(usize::from(right));
        let dims = [
            ("kind", Value::text("output")),
            ("program", Value::text(*name)),
        ];
        rows.push(Reading::new(&dims, "right", i64::from(right), "bool"));
        if let Some(read) = read {
            rows.push(Reading::new(&dims, "answer", read, "count"));
        }
        rows.push(Reading::new(&dims, "expected", *output, "count"));
        rows.push(Reading::new(&dims, "tokens", as_integer(tokens), "tokens"));
        lines.push(format!(
            "  output {name:<18} {}",
            if right { "right" } else { "WRONG" }
        ));
    }
    for (name, code, line, meant) in BUGS {
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let numbered: Vec<String> = code
            .lines()
            .enumerate()
            .map(|(at, text)| format!("{}: {text}", at.saturating_add(1)))
            .collect();
        let asked = format!(
            "This Python function is meant to {meant}, and one line is wrong. Which line \
             number? Answer with the line number only.\n\n```python\n{}\n```",
            numbered.join("\n")
        );
        let (text, tokens) = match ask(&asked) {
            Ok(held) => held,
            Err(why) => return Found::could_not_tell(&why),
        };
        let read = answer_in(&text);
        let right = read == Some(*line);
        bugs_right = bugs_right.saturating_add(usize::from(right));
        let dims = [
            ("kind", Value::text("bug")),
            ("program", Value::text(*name)),
        ];
        rows.push(Reading::new(&dims, "right", i64::from(right), "bool"));
        if let Some(read) = read {
            rows.push(Reading::new(&dims, "answer", read, "count"));
        }
        rows.push(Reading::new(&dims, "expected", *line, "count"));
        rows.push(Reading::new(&dims, "tokens", as_integer(tokens), "tokens"));
        lines.push(format!(
            "  bug    {name:<18} {}",
            if right { "right" } else { "WRONG" }
        ));
    }
    lines.push(format!(
        "  outputs {outputs_right} of {} right; bugs {bugs_right} of {} placed",
        OUTPUTS.len(),
        BUGS.len()
    ));
    Found {
        lines,
        fields: vec![
            ("outputs", Value::Integer(as_integer(OUTPUTS.len()))),
            ("outputs_right", Value::Integer(as_integer(outputs_right))),
            ("bugs", Value::Integer(as_integer(BUGS.len()))),
            ("bugs_right", Value::Integer(as_integer(bugs_right))),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{BUGS, OUTPUTS};

    fn f(n: i64) -> i64 {
        if n < 2 { 1 } else { f(n - 1) + 2 * f(n - 2) }
    }

    #[test]
    #[expect(clippy::integer_division, reason = "the program's own halving, redone")]
    fn the_reference_outputs_hold() {
        let loop_sum: i64 = (1..=5).map(|i| if i % 2 == 0 { i * i } else { i }).sum();
        assert_eq!(loop_sum, OUTPUTS[0].2);
        let count = "abracadabra"
            .chars()
            .enumerate()
            .filter(|(i, c)| *c == 'a' && i % 2 == 0)
            .count();
        assert_eq!(i64::try_from(count).unwrap(), OUTPUTS[1].2);
        assert_eq!(6 + 22, OUTPUTS[2].2);
        assert_eq!(f(6), OUTPUTS[3].2);
        assert_eq!(1 + 2 + 10 + 20, OUTPUTS[4].2);
        let (mut n, mut steps) = (100_i64, 0_i64);
        while n > 1 {
            n = if n % 2 == 0 { n / 2 } else { n - 1 };
            steps += 1;
        }
        assert_eq!(steps, OUTPUTS[5].2);
        for (name, code, line, _) in BUGS {
            let lines = i64::try_from(code.lines().count()).unwrap();
            assert!(*line >= 1 && *line <= lines, "{name}");
        }
    }
}
