//! Test writing: the model writes tests for a stated function, and the
//! tests are run against a correct implementation and against several
//! deliberately broken ones — how many pass on the good one, and how many
//! of the broken ones are caught (B-524, D55).
//!
//! A test that fails on the correct implementation is a wrong test; a
//! suite that passes on a broken implementation missed its bug. Both
//! counts are rows, task by task, and neither is a mark: a suite of one
//! test that catches every bug and a suite of ten that catch none are
//! both written down as what they are.

use std::path::Path;

use mcf_record::json::Value;
use mcf_serve::examine::Reading;

/// How many times each task is asked for.
pub(crate) const ATTEMPTS: usize = 2;

/// The token budget for a suite of tests.
const BUDGET: usize = 1000;

/// One task: a function in words, its correct implementation, and the
/// broken implementations a good suite would catch.
pub(crate) struct Task {
    /// Its name.
    pub name: &'static str,
    /// The function's name, which the ask states.
    pub function: &'static str,
    /// What the function does, in words, as the model is told.
    pub spec: &'static str,
    /// A correct implementation.
    pub good: &'static str,
    /// Implementations with one bug each, named by the bug.
    pub broken: &'static [(&'static str, &'static str)],
}

/// The tasks.
pub(crate) const TASKS: &[Task] = &[
    Task {
        name: "palindrome",
        function: "is_palindrome",
        spec: "`is_palindrome(s)` returns True if the string s reads the same forwards and \
               backwards when case is ignored and every character that is not a letter or a \
               digit is ignored, and False otherwise. The empty string is a palindrome.",
        good: "def is_palindrome(s):\n    t = [c.lower() for c in s if c.isalnum()]\n    return t == t[::-1]\n",
        broken: &[
            (
                "case-sensitive",
                "def is_palindrome(s):\n    t = [c for c in s if c.isalnum()]\n    return t == t[::-1]\n",
            ),
            (
                "keeps-punctuation",
                "def is_palindrome(s):\n    t = [c.lower() for c in s]\n    return t == t[::-1]\n",
            ),
            (
                "odd-length-wrong",
                "def is_palindrome(s):\n    t = [c.lower() for c in s if c.isalnum()]\n    n = len(t)\n    return t[:n // 2] == t[n // 2:][::-1]\n",
            ),
        ],
    },
    Task {
        name: "clamp",
        function: "clamp",
        spec: "`clamp(x, lo, hi)` returns x if it lies between lo and hi inclusive, lo if x is \
               below lo, and hi if x is above hi.",
        good: "def clamp(x, lo, hi):\n    return min(max(x, lo), hi)\n",
        broken: &[
            (
                "swapped-bounds",
                "def clamp(x, lo, hi):\n    if x > hi:\n        return lo\n    if x < lo:\n        return hi\n    return x\n",
            ),
            (
                "upper-exclusive",
                "def clamp(x, lo, hi):\n    if x >= hi:\n        return hi - 1\n    return max(x, lo)\n",
            ),
            (
                "ignores-lower",
                "def clamp(x, lo, hi):\n    return min(x, hi)\n",
            ),
        ],
    },
    Task {
        name: "chunks",
        function: "chunks",
        spec: "`chunks(xs, n)` splits the list xs into consecutive lists of n elements each, in \
               order, the last one shorter if the elements do not divide evenly; an empty list \
               gives an empty list.",
        good: "def chunks(xs, n):\n    return [xs[i:i + n] for i in range(0, len(xs), n)]\n",
        broken: &[
            (
                "drops-the-remainder",
                "def chunks(xs, n):\n    return [xs[i:i + n] for i in range(0, len(xs) - len(xs) % n, n)]\n",
            ),
            (
                "one-too-many",
                "def chunks(xs, n):\n    return [xs[i:i + n + 1] for i in range(0, len(xs), n)]\n",
            ),
            (
                "overlapping",
                "def chunks(xs, n):\n    step = max(n - 1, 1)\n    return [xs[i:i + n] for i in range(0, len(xs), step)]\n",
            ),
        ],
    },
    Task {
        name: "word-count",
        function: "word_count",
        spec: "`word_count(text)` returns a dict from each word in the text, lowercased, to how \
               many times it appears; words are separated by any whitespace, including tabs and \
               newlines.",
        good: "def word_count(text):\n    counts = {}\n    for w in text.lower().split():\n        counts[w] = counts.get(w, 0) + 1\n    return counts\n",
        broken: &[
            (
                "case-sensitive",
                "def word_count(text):\n    counts = {}\n    for w in text.split():\n        counts[w] = counts.get(w, 0) + 1\n    return counts\n",
            ),
            (
                "counts-once",
                "def word_count(text):\n    return {w: 1 for w in text.lower().split()}\n",
            ),
            (
                "spaces-only",
                "def word_count(text):\n    counts = {}\n    for w in text.lower().split(' '):\n        if w:\n            counts[w] = counts.get(w, 0) + 1\n    return counts\n",
            ),
        ],
    },
    Task {
        name: "fizzbuzz",
        function: "fizzbuzz",
        spec: "`fizzbuzz(n)` returns a list of n strings for the numbers 1 to n: 'FizzBuzz' for \
               a multiple of both 3 and 5, 'Fizz' for a multiple of 3, 'Buzz' for a multiple of \
               5, and the number itself as a string otherwise.",
        good: "def fizzbuzz(n):\n    out = []\n    for i in range(1, n + 1):\n        if i % 15 == 0:\n            out.append('FizzBuzz')\n        elif i % 3 == 0:\n            out.append('Fizz')\n        elif i % 5 == 0:\n            out.append('Buzz')\n        else:\n            out.append(str(i))\n    return out\n",
        broken: &[
            (
                "fifteen-is-fizz",
                "def fizzbuzz(n):\n    out = []\n    for i in range(1, n + 1):\n        if i % 3 == 0:\n            out.append('Fizz')\n        elif i % 5 == 0:\n            out.append('Buzz')\n        else:\n            out.append(str(i))\n    return out\n",
            ),
            (
                "starts-at-zero",
                "def fizzbuzz(n):\n    out = []\n    for i in range(0, n):\n        if i % 15 == 0:\n            out.append('FizzBuzz')\n        elif i % 3 == 0:\n            out.append('Fizz')\n        elif i % 5 == 0:\n            out.append('Buzz')\n        else:\n            out.append(str(i))\n    return out\n",
            ),
            (
                "numbers-not-strings",
                "def fizzbuzz(n):\n    out = []\n    for i in range(1, n + 1):\n        if i % 15 == 0:\n            out.append('FizzBuzz')\n        elif i % 3 == 0:\n            out.append('Fizz')\n        elif i % 5 == 0:\n            out.append('Buzz')\n        else:\n            out.append(i)\n    return out\n",
            ),
        ],
    },
];

/// What the model is asked.
#[must_use]
pub(crate) fn prompt_for(task: &Task) -> String {
    format!(
        "A Python function {} Write tests for it: functions whose names start with `test_`, \
         taking no arguments, each using `assert`. The function `{}` is already defined where \
         the tests run; do not define it and do not import it. Reply with only the tests.",
        task.spec, task.function
    )
}

/// The program the container runs: a small `pytest` stand-in for `raises`,
/// `approx` and `parametrize`, then, for the correct implementation and
/// each broken one, the implementation, the tests, and one line `impl
/// <index> <passed> <of>` — or `impl <index> x` where the tests could not
/// be loaded — and nothing else.
#[must_use]
pub(crate) fn checker(task: &Task, tests: &str) -> String {
    let mut out = String::from(
        "import sys, types, unittest, io\n\n\
         class _Raises:\n    def __init__(self, kind, **_):\n        self.kind = kind\n    \
         def __enter__(self):\n        return self\n    \
         def __exit__(self, kind, value, trace):\n        \
         return kind is not None and issubclass(kind, self.kind)\n\n\
         class _Approx:\n    def __init__(self, value, rel=1e-6, abs=1e-12):\n        \
         self.value, self.rel, self.abs = value, rel, abs\n    \
         def __eq__(self, other):\n        \
         return abs(other - self.value) <= max(self.rel * abs(self.value), self.abs)\n\n\
         def _parametrize(names, values, **_):\n    \
         def wrap(fn):\n        fn._params = (names, values)\n        return fn\n    \
         return wrap\n\n\
         def _fixture(fn=None, **_):\n    \
         return fn if fn is not None else (lambda held: held)\n\n\
         _mark = types.SimpleNamespace(parametrize=_parametrize, skip=lambda *a, **k: (lambda fn: fn))\n\
         sys.modules['pytest'] = types.SimpleNamespace(raises=_Raises, approx=_Approx, mark=_mark, fixture=_fixture)\n\n\
         TESTS = ",
    );
    out.push_str(&python_string(tests));
    out.push_str("\n\nIMPLS = [\n");
    for implementation in
        std::iter::once(task.good).chain(task.broken.iter().map(|(_, code)| *code))
    {
        out.push_str("    ");
        out.push_str(&python_string(implementation));
        out.push_str(",\n");
    }
    out.push_str(
        "]\n\n\
         def _run(index, impl):\n    \
         ns = {'__name__': '__tests__'}\n    \
         try:\n        exec(impl, ns)\n        exec(TESTS, ns)\n    \
         except BaseException as failed:\n        \
         print('impl', index, 'x', type(failed).__name__)\n        return\n    \
         passed = of = 0\n    \
         for name, held in list(ns.items()):\n        \
         if name.startswith('test_') and callable(held) and not isinstance(held, type):\n            \
         params = getattr(held, '_params', None)\n            \
         if params:\n                \
         names, values = params\n                \
         names = [n.strip() for n in names.split(',')] if isinstance(names, str) else list(names)\n                \
         for row in values:\n                    \
         row = row if isinstance(row, (tuple, list)) and len(names) > 1 else (row,)\n                    \
         of += 1\n                    \
         try:\n                        held(**dict(zip(names, row)))\n                        passed += 1\n                    \
         except BaseException:\n                        pass\n            \
         else:\n                \
         of += 1\n                \
         try:\n                    held()\n                    passed += 1\n                \
         except BaseException:\n                    pass\n        \
         elif isinstance(held, type) and issubclass(held, unittest.TestCase):\n            \
         suite = unittest.defaultTestLoader.loadTestsFromTestCase(held)\n            \
         result = unittest.TextTestRunner(stream=io.StringIO(), verbosity=0).run(suite)\n            \
         of += result.testsRun\n            \
         passed += result.testsRun - len(result.failures) - len(result.errors)\n    \
         print('impl', index, passed, of)\n\n\
         for index, impl in enumerate(IMPLS):\n    _run(index, impl)\n",
    );
    out
}

/// A Python string literal holding the text.
fn python_string(text: &str) -> String {
    format!(
        "'''{}'''",
        text.replace('\\', "\\\\").replace("'''", "\\'\\'\\'")
    )
}

/// What one implementation reported: the tests passed of the tests, or
/// that the tests could not be loaded beside it, with the exception's
/// name where the program said one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reported {
    Ran { passed: usize, of: usize },
    NotLoaded(String),
}

/// The lines the program printed, read as one report an implementation,
/// in index order; `None` where an implementation is missing.
#[must_use]
pub(crate) fn reports_in(said: &str, implementations: usize) -> Option<Vec<Reported>> {
    let mut held = vec![None; implementations];
    for line in said.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let (Some(&"impl"), Some(index)) = (words.first(), words.get(1)) else {
            continue;
        };
        let Ok(index) = index.parse::<usize>() else {
            continue;
        };
        let report = match (words.get(2), words.get(3)) {
            (Some(&"x"), reason) => Reported::NotLoaded((*reason.unwrap_or(&"")).to_owned()),
            (Some(passed), Some(of)) => match (passed.parse(), of.parse()) {
                (Ok(passed), Ok(of)) => Reported::Ran { passed, of },
                _ => continue,
            },
            _ => continue,
        };
        if let Some(slot) = held.get_mut(index) {
            *slot = Some(report);
        }
    }
    held.into_iter().collect()
}

/// One attempt's rows: the tests written, what passed on the good
/// implementation, and whether each broken one was caught.
#[must_use]
pub(crate) fn rows_of(
    task: &Task,
    attempt: usize,
    reports: Option<&[Reported]>,
    tests_bytes: usize,
    ask_ns: u64,
) -> Vec<Reading> {
    let whole = |count: usize| i64::try_from(count).unwrap_or(i64::MAX);
    let dims = [
        ("task", Value::text(task.name.to_owned())),
        ("attempt", Value::Integer(whole(attempt))),
    ];
    let mut rows = vec![
        Reading::new(
            &dims,
            "ask_ns",
            i64::try_from(ask_ns).unwrap_or(i64::MAX),
            "ns",
        ),
        Reading::new(&dims, "tests_bytes", whole(tests_bytes), "bytes"),
        Reading::new(&dims, "wrote", i64::from(tests_bytes > 0), "bool"),
        Reading::new(&dims, "broken", whole(task.broken.len()), "count"),
    ];
    let Some(reports) = reports else {
        rows.push(Reading::new(&dims, "ran", 0, "bool"));
        return rows;
    };
    rows.push(Reading::new(&dims, "ran", 1, "bool"));
    let Some(Reported::Ran { passed, of }) = reports.first() else {
        rows.push(Reading::new(&dims, "loaded", 0, "bool"));
        return rows;
    };
    rows.push(Reading::new(&dims, "loaded", 1, "bool"));
    rows.push(Reading::new(&dims, "tests", whole(*of), "count"));
    rows.push(Reading::new(&dims, "pass_on_good", whole(*passed), "count"));
    rows.push(Reading::new(
        &dims,
        "good_all_pass",
        i64::from(passed == of && *of > 0),
        "bool",
    ));
    let mut caught = 0_usize;
    for ((bug, _), report) in task.broken.iter().zip(reports.iter().skip(1)) {
        // Caught: at least one test fails on it that passed on the good
        // one — read as fewer passing than on the good implementation.
        let was = match report {
            Reported::Ran { passed: on_bug, .. } => on_bug < passed,
            Reported::NotLoaded(_) => true,
        };
        caught = caught.saturating_add(usize::from(was));
        rows.push(Reading::new(
            &[
                ("task", Value::text(task.name.to_owned())),
                ("attempt", Value::Integer(whole(attempt))),
                ("bug", Value::text((*bug).to_owned())),
            ],
            "caught",
            i64::from(was),
            "bool",
        ));
    }
    rows.push(Reading::new(&dims, "caught", whole(caught), "count"));
    rows
}

/// Runs every task: the lines said and the rows.
pub(crate) fn run(
    socket: &Path,
    named: &str,
    podman: &Path,
    scratch: &Path,
) -> (Vec<String>, Vec<Reading>) {
    let mut lines = vec![
        format!(
            "  {} test-writing task(s), {ATTEMPTS} attempt(s) each: tests written for a stated \
             function, run against a correct implementation and against broken ones",
            TASKS.len()
        ),
        String::new(),
    ];
    let mut rows = Vec::new();
    for task in TASKS {
        lines.push(format!("  {}", task.name));
        for attempt in 0..ATTEMPTS {
            let began = std::time::Instant::now();
            let spoken = mcf_serve::probes::spoken(
                socket,
                Path::new(named),
                &prompt_for(task),
                None,
                BUDGET,
                None,
            );
            let ask_ns = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
            let tests = crate::eval::code_in(&spoken.text);
            let cut = matches!(spoken.trial, mcf_serve::probes::Trial::RanOut);
            let reports = if tests.trim().is_empty() {
                None
            } else {
                crate::eval::run_python(podman, scratch, &checker(task, &tests))
                    .ok()
                    .and_then(|said| reports_in(&said, task.broken.len().wrapping_add(1)))
            };
            lines.push(format!(
                "      attempt {}: {}{}",
                attempt.wrapping_add(1),
                said_of(task, reports.as_deref()),
                if cut { " — cut at the budget" } else { "" }
            ));
            rows.extend(rows_of(
                task,
                attempt,
                reports.as_deref(),
                tests.len(),
                ask_ns,
            ));
            rows.push(Reading::new(
                &[
                    ("task", Value::text(task.name.to_owned())),
                    (
                        "attempt",
                        Value::Integer(i64::try_from(attempt).unwrap_or(i64::MAX)),
                    ),
                ],
                "cut_at_budget",
                i64::from(cut),
                "bool",
            ));
        }
        lines.push(String::new());
    }
    (lines, rows)
}

/// One attempt in words.
fn said_of(task: &Task, reports: Option<&[Reported]>) -> String {
    let Some(reports) = reports else {
        return "did not run".to_owned();
    };
    let Some(Reported::Ran { passed, of }) = reports.first() else {
        return format!(
            "the tests could not be loaded{}",
            match reports.first() {
                Some(Reported::NotLoaded(reason)) if !reason.is_empty() => format!(" ({reason})"),
                _ => String::new(),
            }
        );
    };
    let caught = task
        .broken
        .iter()
        .zip(reports.iter().skip(1))
        .filter(|(_, report)| match report {
            Reported::Ran { passed: on_bug, .. } => on_bug < passed,
            Reported::NotLoaded(_) => true,
        })
        .count();
    format!(
        "{of} test(s); {passed} pass on the correct implementation; {caught} of {} broken \
         implementation(s) caught",
        task.broken.len()
    )
}

#[cfg(test)]
mod tests {
    use super::{Reported, TASKS, checker, reports_in, rows_of};

    #[test]
    fn the_report_lines_are_read_in_index_order() {
        let said = "impl 0 3 3\nnoise\nimpl 2 x NameError\nimpl 1 2 3\n";
        assert_eq!(
            reports_in(said, 3),
            Some(vec![
                Reported::Ran { passed: 3, of: 3 },
                Reported::Ran { passed: 2, of: 3 },
                Reported::NotLoaded("NameError".to_owned())
            ])
        );
        assert_eq!(reports_in("impl 0 3 3\n", 2), None);
    }

    #[test]
    fn a_broken_implementation_is_caught_when_fewer_tests_pass_on_it() {
        let reports = [
            Reported::Ran { passed: 4, of: 4 },
            Reported::Ran { passed: 3, of: 4 },
            Reported::Ran { passed: 4, of: 4 },
            Reported::NotLoaded(String::new()),
        ];
        let rows = rows_of(&TASKS[0], 0, Some(&reports), 120, 5);
        let caught: Vec<(String, i64)> = rows
            .iter()
            .filter(|row| row.metric == "caught")
            .map(|row| {
                (
                    row.dims
                        .get("bug")
                        .map_or("total".to_owned(), mcf_record::json::Value::to_line),
                    row.value,
                )
            })
            .collect();
        assert_eq!(caught.len(), 4, "{caught:?}");
        assert_eq!(
            caught
                .iter()
                .find(|(bug, _)| bug == "total")
                .map(|(_, n)| *n),
            Some(2)
        );
        assert!(
            rows.iter()
                .any(|row| row.metric == "good_all_pass" && row.value == 1)
        );
    }

    #[test]
    fn the_checker_holds_the_good_and_every_broken_implementation() {
        let program = checker(
            &TASKS[1],
            "def test_inside():\n    assert clamp(5, 1, 10) == 5\n",
        );
        assert!(program.contains("IMPLS = ["));
        assert_eq!(program.matches("def clamp(x, lo, hi):").count(), 4);
        assert!(program.contains("sys.modules['pytest']"));
        assert!(program.contains("def test_inside():"));
    }

    /// A good suite for every task — one written three ways, so the
    /// `pytest` stand-in, `parametrize` and `unittest` are all exercised —
    /// run in the container: every test passes on the correct
    /// implementation and every broken one is caught. Needs podman.
    const REFERENCE: &[(&str, &str)] = &[
        (
            "palindrome",
            "import pytest\n\n@pytest.mark.parametrize('s, want', [('A man, a plan, a canal: Panama', True), ('abc', False), ('', True), ('aba', True), ('ab', False)])\ndef test_cases(s, want):\n    assert is_palindrome(s) == want\n\ndef test_case_ignored():\n    assert is_palindrome('Aba')\n\ndef test_odd_length():\n    assert is_palindrome('racecar')\n    assert not is_palindrome('racecax')\n",
        ),
        (
            "clamp",
            "import unittest\n\nclass ClampTests(unittest.TestCase):\n    def test_inside(self):\n        self.assertEqual(clamp(5, 1, 10), 5)\n    def test_below(self):\n        self.assertEqual(clamp(-3, 1, 10), 1)\n    def test_above(self):\n        self.assertEqual(clamp(50, 1, 10), 10)\n    def test_edges(self):\n        self.assertEqual(clamp(10, 1, 10), 10)\n        self.assertEqual(clamp(1, 1, 10), 1)\n",
        ),
        (
            "chunks",
            "import unittest\n\ndef test_even():\n    assert chunks([1, 2, 3, 4], 2) == [[1, 2], [3, 4]]\n\ndef test_remainder():\n    assert chunks([1, 2, 3, 4, 5], 2) == [[1, 2], [3, 4], [5]]\n\ndef test_empty():\n    assert chunks([], 3) == []\n\ndef test_sizes():\n    assert all(len(c) == 3 for c in chunks(list(range(9)), 3))\n\nif __name__ == '__main__':\n    unittest.main()\n",
        ),
        (
            "word-count",
            "def test_counts():\n    assert word_count('a b a') == {'a': 2, 'b': 1}\n\ndef test_lowercases():\n    assert word_count('The the') == {'the': 2}\n\ndef test_any_whitespace():\n    assert word_count('a\\tb\\nc') == {'a': 1, 'b': 1, 'c': 1}\n\ndef test_empty():\n    assert word_count('') == {}\n",
        ),
        (
            "fizzbuzz",
            "import pytest\n\ndef test_first_fifteen():\n    assert fizzbuzz(15) == ['1', '2', 'Fizz', '4', 'Buzz', 'Fizz', '7', '8', 'Fizz', 'Buzz', '11', 'Fizz', '13', '14', 'FizzBuzz']\n\ndef test_length():\n    assert len(fizzbuzz(7)) == 7\n\ndef test_strings():\n    assert all(isinstance(x, str) for x in fizzbuzz(5))\n\ndef test_raises_stand_in():\n    with pytest.raises(TypeError):\n        fizzbuzz('x')\n",
        ),
    ];

    #[test]
    #[ignore = "needs podman and the pinned image; run with --ignored"]
    fn a_good_suite_passes_on_the_correct_implementation_and_catches_every_bug() {
        let podman = std::path::Path::new("/usr/bin/podman");
        let scratch = std::env::temp_dir().join(format!("mcf-testing-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let mut wrong = Vec::new();
        for task in TASKS {
            let Some((_, tests)) = REFERENCE.iter().find(|(name, _)| *name == task.name) else {
                wrong.push(format!("{}: no reference", task.name));
                continue;
            };
            let said = crate::eval::run_python(podman, &scratch, &checker(task, tests)).unwrap();
            let Some(reports) = reports_in(&said, task.broken.len() + 1) else {
                wrong.push(format!("{}: {said:?}", task.name));
                continue;
            };
            let rows = rows_of(task, 0, Some(&reports), tests.len(), 0);
            let value = |metric: &str| {
                rows.iter()
                    .find(|row| row.metric == metric && !row.dims.contains_key("bug"))
                    .map(|row| row.value)
            };
            if value("good_all_pass") != Some(1) || value("caught") != Some(3) {
                wrong.push(format!("{}: {reports:?}", task.name));
            }
        }
        let _gone = std::fs::remove_dir_all(&scratch);
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
