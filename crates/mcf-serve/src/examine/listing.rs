use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "listing";

pub const ASKS: &[(&str, &str, usize)] = &[
    ("animals-10", "animals", 10),
    ("animals-25", "animals", 25),
    ("animals-50", "animals", 50),
    ("countries-20", "countries", 20),
    ("countries-40", "countries", 40),
    ("fruits-15", "kinds of fruit", 15),
    ("fruits-30", "kinds of fruit", 30),
    ("tools-20", "hand tools", 20),
];

const BUDGET: usize = 700;

#[must_use]
pub fn ask_for(what: &str, count: usize) -> String {
    format!(
        "List exactly {count} different {what}, one per line, with no numbering, no repeats and \
         nothing else."
    )
}

#[must_use]
pub fn item_of(line: &str) -> String {
    let trimmed = line.trim();
    let without_number = trimmed
        .trim_start_matches(|c: char| {
            c.is_ascii_digit() || c == '.' || c == ')' || c == '-' || c == '*' || c == '•'
        })
        .trim();
    without_number
        .trim_end_matches(['.', ',', ';'])
        .to_lowercase()
}

#[must_use]
pub fn counted(said: &str) -> (usize, usize, usize) {
    let items: Vec<String> = said
        .lines()
        .map(item_of)
        .filter(|item| !item.is_empty())
        .collect();
    let distinct: std::collections::BTreeSet<&String> = items.iter().collect();
    (
        items.len(),
        distinct.len(),
        items.len().saturating_sub(distinct.len()),
    )
}

#[must_use]
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
        "  {} ask(s) for so many distinct items one per line, greedy; the lines counted and the \
         repeats found by a parser",
        ASKS.len()
    )];
    let (mut exact, mut clean) = (0_usize, 0_usize);
    for (at, (name, what, count)) in ASKS.iter().enumerate() {
        site.progress(at, ASKS.len(), name);
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let ids = match framed_ids(&engine, &ask_for(what, *count)) {
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
        let (items, distinct, repeats) = counted(&completed.text);
        let count_held = items == *count;
        let none_repeated = repeats == 0;
        exact = exact.saturating_add(usize::from(count_held));
        clean = clean.saturating_add(usize::from(none_repeated));
        let dims = [("ask", Value::text(*name))];
        rows.push(Reading::new(&dims, "asked", as_integer(*count), "count"));
        rows.push(Reading::new(&dims, "items", as_integer(items), "count"));
        rows.push(Reading::new(
            &dims,
            "distinct",
            as_integer(distinct),
            "count",
        ));
        rows.push(Reading::new(&dims, "repeats", as_integer(repeats), "count"));
        rows.push(Reading::new(
            &dims,
            "count_held",
            i64::from(count_held),
            "bool",
        ));
        rows.push(Reading::new(
            &dims,
            "none_repeated",
            i64::from(none_repeated),
            "bool",
        ));
        rows.push(Reading::new(
            &dims,
            "tokens",
            as_integer(completed.predicted),
            "tokens",
        ));
        let cut = completed.predicted >= BUDGET;
        rows.push(Reading::new(&dims, "cut_at_budget", i64::from(cut), "bool"));
        lines.push(format!(
            "  {name:<14} asked {count:>2}   gave {items:>3} line(s), {distinct:>3} distinct, \
             {repeats} repeat(s){}",
            if cut { " — cut at the budget" } else { "" }
        ));
    }
    lines.push(format!(
        "  the count held in {exact} of {} ask(s); nothing repeated in {clean}",
        ASKS.len()
    ));
    Found {
        lines,
        fields: vec![
            ("asks", Value::Integer(as_integer(ASKS.len()))),
            ("count_held", Value::Integer(as_integer(exact))),
            ("none_repeated", Value::Integer(as_integer(clean))),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{ask_for, counted, item_of};

    #[test]
    fn lines_are_counted_and_repeats_found_case_aside() {
        assert_eq!(counted("Cat\ndog\n\nCAT\n3. Horse."), (4, 3, 1));
        assert_eq!(item_of("12) Zebra,"), "zebra");
        assert_eq!(item_of("- Lion"), "lion");
        assert!(ask_for("animals", 10).contains("exactly 10"));
    }
}
