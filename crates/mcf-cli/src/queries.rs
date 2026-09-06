//! SQL and regular expressions: queries run in the container against a
//! fixed database and compared exactly; patterns run against match and
//! no-match cases (B-551, D55, B-523, B-025).
//!
//! Two small languages a coding model is asked for daily, each checked
//! by running what the model wrote. The database is built in the
//! container from fixed rows; a query's result is compared with the
//! reference row for row. A pattern is run by Python's own engine against
//! texts that must match and texts that must not.

use std::path::Path;

use mcf_record::json::Value;
use mcf_serve::examine::Reading;

/// How many attempts each task.
pub(crate) const ATTEMPTS: usize = 2;

/// The token budget for one answer.
const BUDGET: usize = 200;

/// The database every query runs against, built fresh in the container.
const SCHEMA: &str = "CREATE TABLE orders(id INTEGER, customer TEXT, city TEXT, amount INTEGER, placed TEXT);\n\
INSERT INTO orders VALUES (1,'Ada','Leeds',120,'2024-01-05'),(2,'Ben','York',80,'2024-01-09'),\
(3,'Ada','Leeds',45,'2024-02-02'),(4,'Cy','Hull',300,'2024-02-15'),(5,'Ben','York',60,'2024-03-01'),\
(6,'Dee','Leeds',210,'2024-03-20'),(7,'Ada','Leeds',15,'2024-03-28');";

/// One SQL task: name, what is asked, the reference query whose rows are
/// the answer.
pub(crate) const QUERIES: &[(&str, &str, &str)] = &[
    (
        "total-by-customer",
        "the total amount per customer, as rows of customer and total, ordered by customer name",
        "SELECT customer, SUM(amount) FROM orders GROUP BY customer ORDER BY customer",
    ),
    (
        "largest-order",
        "the id of the order with the largest amount, as one row with one column",
        "SELECT id FROM orders ORDER BY amount DESC LIMIT 1",
    ),
    (
        "leeds-in-march",
        "the ids of orders placed in Leeds during March 2024, ordered by id",
        "SELECT id FROM orders WHERE city='Leeds' AND placed >= '2024-03-01' AND placed < '2024-04-01' ORDER BY id",
    ),
    (
        "customers-over-100",
        "the names of customers whose total amount is over 100, ordered by name",
        "SELECT customer FROM orders GROUP BY customer HAVING SUM(amount) > 100 ORDER BY customer",
    ),
    (
        "orders-per-city",
        "the number of orders per city, as rows of city and count, ordered by count descending then city",
        "SELECT city, COUNT(*) FROM orders GROUP BY city ORDER BY COUNT(*) DESC, city",
    ),
];

/// One regex task: name, what the pattern must do, texts that must match
/// whole, texts that must not.
pub(crate) const PATTERNS: &[(&str, &str, &[&str], &[&str])] = &[
    (
        "uk-postcode-like",
        "match a code of one or two capital letters, one or two digits, a space, a digit and two capital letters, and nothing else",
        &["B33 8TH", "M1 1AE", "LS10 2AB"],
        &["m1 1ae", "M11AE", "M1 1A", "1SW 1AA"],
    ),
    (
        "iso-date",
        "match a date written as four digits, a hyphen, two digits, a hyphen, two digits, and nothing else",
        &["2024-01-05", "1999-12-31"],
        &["2024-1-5", "05-01-2024", "2024-01-05T10:00", "20240105"],
    ),
    (
        "hex-colour",
        "match a hash followed by exactly six hexadecimal digits in either case, and nothing else",
        &["#a1b2c3", "#FFFFFF", "#0f0F0f"],
        &["a1b2c3", "#a1b2c", "#a1b2c3d", "#g1b2c3"],
    ),
    (
        "simple-email",
        "match one or more letters, digits, dots or underscores, an at sign, one or more letters or digits, a dot, and two or more letters, and nothing else",
        &["ada.b@leeds.ac", "ben_9@york.org"],
        &["ada@leeds", "@york.org", "ada b@leeds.ac", "ada@york.o"],
    ),
];

/// What is asked for a query.
fn ask_query(wants: &str) -> String {
    format!(
        "An SQLite table is defined as:\n\n{SCHEMA}\n\nWrite one SQL query that returns {wants}. \
         Reply with only the query."
    )
}

/// What is asked for a pattern.
fn ask_pattern(wants: &str) -> String {
    format!(
        "Write a Python regular expression that will {wants}. It will be used with re.fullmatch. \
         Reply with only the pattern, with no quotes and no explanation."
    )
}

/// The program that runs a query beside the reference and prints `ok`
/// or `no`, or `x` where the model's query would not run.
fn query_checker(written: &str, reference: &str) -> String {
    format!(
        "import sqlite3\nc = sqlite3.connect(':memory:')\nc.executescript({schema})\n\
         want = c.execute({reference}).fetchall()\n\
         try:\n    got = c.execute({written}).fetchall()\n    print('ok' if got == want else 'no')\n\
         except Exception:\n    print('x')\n",
        schema = python_string(SCHEMA),
        reference = python_string(reference),
        written = python_string(written),
    )
}

/// The program that runs a pattern against every case and prints one
/// `ok` or `no` a case, or `x` where the pattern would not compile.
fn pattern_checker(written: &str, yes: &[&str], no: &[&str]) -> String {
    use std::fmt::Write as _;
    let mut out = format!(
        "import re\ntry:\n    p = re.compile({})\nexcept Exception:\n    print('x')\n    raise SystemExit\n",
        python_string(written)
    );
    for text in yes {
        let _wrote = writeln!(
            out,
            "print('ok' if p.fullmatch({}) else 'no')",
            python_string(text)
        );
    }
    for text in no {
        let _wrote = writeln!(
            out,
            "print('ok' if not p.fullmatch({}) else 'no')",
            python_string(text)
        );
    }
    out
}

/// A Python string literal holding the text.
fn python_string(text: &str) -> String {
    format!(
        "'''{}'''",
        text.replace('\\', "\\\\").replace("'''", "\\'\\'\\'")
    )
}

/// The query or pattern in an answer: the code fence stripped, the first
/// non-empty line kept where several came, a trailing semicolon allowed.
pub(crate) fn written_in(said: &str) -> String {
    let inside = crate::eval::code_in(said);
    // Prose with the answer between single backticks: the answer alone.
    let inside = match inside.split_once('`') {
        Some((_, rest)) if !said.contains("```") => rest
            .split_once('`')
            .map_or(rest, |(held, _)| held)
            .to_owned(),
        _ => inside,
    };
    let trimmed = inside.trim();
    if trimmed.contains("SELECT") || trimmed.to_uppercase().starts_with("SELECT") {
        return trimmed.to_owned();
    }
    trimmed
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .trim_matches(|c| c == '`' || c == '"' || c == '\'')
        .trim_start_matches("r'")
        .trim_start_matches("r\"")
        .to_owned()
}

/// Runs every task: the lines said, the rows, and the engine that answered.
#[allow(
    clippy::too_many_lines,
    reason = "one suite read straight through: each query and each pattern asked, run, a row set each"
)]
pub(crate) fn run(
    socket: &Path,
    named: &str,
    podman: &Path,
    scratch: &Path,
) -> (Vec<String>, Vec<Reading>, Option<String>) {
    let mut lines = vec![
        format!(
            "  {} SQL task(s) and {} pattern task(s), {ATTEMPTS} attempt(s) each; a query is \
             run in the container against a fixed table beside the reference, a pattern against \
             match and no-match cases",
            QUERIES.len(),
            PATTERNS.len()
        ),
        String::new(),
    ];
    let mut rows = Vec::new();
    let mut engine_ran = None;
    let ask = |text: &str, engine_ran: &mut Option<String>| {
        let began = std::time::Instant::now();
        let spoken = mcf_serve::probes::spoken(socket, Path::new(named), text, None, BUDGET, None);
        if engine_ran.is_none() {
            engine_ran.clone_from(&spoken.engine_ran);
        }
        (
            written_in(&spoken.text),
            u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX),
        )
    };
    for (name, wants, reference) in QUERIES {
        lines.push(format!("  {name}"));
        for attempt in 0..ATTEMPTS {
            let (written, ask_ns) = ask(&ask_query(wants), &mut engine_ran);
            let said =
                crate::eval::run_python(podman, scratch, &query_checker(&written, reference))
                    .unwrap_or_default();
            let (ran, right) = (
                said.lines().any(|line| line == "ok" || line == "no"),
                said.lines().any(|line| line == "ok"),
            );
            let dims = [
                ("kind", Value::text("sql")),
                ("task", Value::text(*name)),
                (
                    "attempt",
                    Value::Integer(i64::try_from(attempt).unwrap_or(0)),
                ),
            ];
            rows.push(Reading::new(
                &dims,
                "wrote",
                i64::from(!written.is_empty()),
                "bool",
            ));
            rows.push(Reading::new(&dims, "ran", i64::from(ran), "bool"));
            rows.push(Reading::new(&dims, "right", i64::from(right), "bool"));
            rows.push(Reading::new(
                &dims,
                "ask_ns",
                i64::try_from(ask_ns).unwrap_or(i64::MAX),
                "ns",
            ));
            lines.push(format!(
                "      attempt {}: {}",
                attempt + 1,
                if right {
                    "the rows matched"
                } else if ran {
                    "the rows did not match"
                } else {
                    "did not run"
                }
            ));
        }
    }
    for (name, wants, yes, no) in PATTERNS {
        lines.push(format!("  {name}"));
        for attempt in 0..ATTEMPTS {
            let (written, ask_ns) = ask(&ask_pattern(wants), &mut engine_ran);
            let said =
                crate::eval::run_python(podman, scratch, &pattern_checker(&written, yes, no))
                    .unwrap_or_default();
            let cases = yes.len() + no.len();
            let held = said.lines().filter(|line| *line == "ok").count();
            let compiled = !said.lines().any(|line| line == "x")
                && said.lines().filter(|l| *l == "ok" || *l == "no").count() == cases;
            let dims = [
                ("kind", Value::text("regex")),
                ("task", Value::text(*name)),
                (
                    "attempt",
                    Value::Integer(i64::try_from(attempt).unwrap_or(0)),
                ),
            ];
            rows.push(Reading::new(
                &dims,
                "wrote",
                i64::from(!written.is_empty()),
                "bool",
            ));
            rows.push(Reading::new(&dims, "compiled", i64::from(compiled), "bool"));
            rows.push(Reading::new(
                &dims,
                "cases_held",
                i64::try_from(held).unwrap_or(0),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "cases",
                i64::try_from(cases).unwrap_or(0),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "whole",
                i64::from(compiled && held == cases),
                "bool",
            ));
            rows.push(Reading::new(
                &dims,
                "ask_ns",
                i64::try_from(ask_ns).unwrap_or(i64::MAX),
                "ns",
            ));
            lines.push(format!(
                "      attempt {}: {}",
                attempt + 1,
                if compiled {
                    format!("{held} of {cases} case(s) held")
                } else {
                    "did not compile".to_owned()
                }
            ));
        }
    }
    lines.push(String::new());
    (lines, rows, engine_ran)
}

#[cfg(test)]
mod tests {
    use super::{PATTERNS, QUERIES, pattern_checker, query_checker, written_in};

    #[test]
    fn the_answer_is_read_out_of_prose_and_fences() {
        assert_eq!(
            written_in("```sql\nSELECT id FROM orders;\n```"),
            "SELECT id FROM orders;"
        );
        assert_eq!(
            written_in("Here: `^\\d{4}-\\d{2}-\\d{2}$`"),
            "^\\d{4}-\\d{2}-\\d{2}$"
        );
        assert_eq!(written_in("r'^#[0-9a-fA-F]{6}$'"), "^#[0-9a-fA-F]{6}$");
    }

    /// The reference queries and patterns hold in the container: every
    /// reference query matches itself and every reference pattern holds
    /// its cases. Needs podman.
    #[test]
    #[ignore = "needs podman and the pinned image; run with --ignored"]
    fn every_reference_holds_in_the_container() {
        let podman = std::path::Path::new("/usr/bin/podman");
        let scratch = std::env::temp_dir().join(format!("mcf-queries-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        for (name, _, reference) in QUERIES {
            let said =
                crate::eval::run_python(podman, &scratch, &query_checker(reference, reference))
                    .unwrap();
            assert_eq!(said.trim(), "ok", "{name}");
        }
        let references = [
            "[A-Z]{1,2}[0-9]{1,2} [0-9][A-Z]{2}",
            "[0-9]{4}-[0-9]{2}-[0-9]{2}",
            "#[0-9a-fA-F]{6}",
            "[A-Za-z0-9._]+@[A-Za-z0-9]+\\.[A-Za-z]{2,}",
        ];
        for ((name, _, yes, no), pattern) in PATTERNS.iter().zip(references) {
            let said =
                crate::eval::run_python(podman, &scratch, &pattern_checker(pattern, yes, no))
                    .unwrap();
            assert!(said.lines().all(|line| line == "ok"), "{name}: {said}");
        }
        let _gone = std::fs::remove_dir_all(&scratch);
    }
}
