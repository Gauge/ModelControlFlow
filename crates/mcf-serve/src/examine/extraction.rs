//! Structured extraction: fixed texts with dates, amounts, names and
//! lists pulled into JSON, checked field by field by exact match
//! (B-525, D55, A19).
//!
//! The grammar measurement asks whether JSON comes out; this asks
//! whether the right JSON does. Five short texts — an invoice line, a
//! meeting note, a shipping note, a job listing, a weather line — each
//! with the keys wanted and the form each value must take: dates as
//! `YYYY-MM-DD`, amounts as whole cents or grams, lists as arrays of
//! strings. The answer is read by a parser and each field compared with
//! what the text says, exactly; a field is a row, and nothing here judges
//! whether a wrong answer was close.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, timed};
use crate::generation::{Draw, Truncation};
use crate::served::{Prompt, Startup};
use mcf_core::configuration::Thousandths;

/// The measurement's name.
pub const NAME: &str = "extraction";

/// How many trials a text: the first greedy, the rest drawn.
pub const TRIALS: usize = 3;

/// The temperature of the drawn trials, in thousandths.
const TEMPERATURE: u32 = 700;

/// How many tokens an answer may take.
const BUDGET: usize = 220;

/// One text and what is to be pulled out of it.
#[derive(Debug)]
pub struct Text {
    /// Its name.
    pub name: &'static str,
    /// The text.
    pub text: &'static str,
    /// The keys wanted, each with the form its value must take, in words.
    pub wants: &'static str,
    /// The fields as they should come back.
    pub fields: &'static [(&'static str, Expected)],
}

/// What a field should hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// A whole number.
    Number(i64),
    /// Text, matched exactly.
    Words(&'static str),
    /// A list of texts, in order, matched exactly.
    List(&'static [&'static str]),
}

/// The texts.
pub const TEXTS: &[Text] = &[
    Text {
        name: "invoice",
        text: "Invoice 4471 from Northwind Traders, dated 12 March 2024, comes to 1,284.50 EUR \
               and is due in 30 days. It lists 3 desk lamps and 2 cable trays.",
        wants: "invoice (whole number), vendor (text), date (YYYY-MM-DD), total_cents (whole \
                number), currency (text), due_days (whole number), quantities (array of whole \
                numbers in the order listed)",
        fields: &[
            ("invoice", Expected::Number(4471)),
            ("vendor", Expected::Words("Northwind Traders")),
            ("date", Expected::Words("2024-03-12")),
            ("total_cents", Expected::Number(128_450)),
            ("currency", Expected::Words("EUR")),
            ("due_days", Expected::Number(30)),
            ("quantities", Expected::List(&["3", "2"])),
        ],
    },
    Text {
        name: "meeting",
        text: "The design review is on Thursday 7 November 2024 at 14:30 in room B12. Priya \
               Natarajan, Tom Okafor and Lena Vogt are attending; bring the third-quarter \
               figures.",
        wants: "date (YYYY-MM-DD), time (HH:MM), room (text), attendees (array of full names in \
                the order given), attendee_count (whole number)",
        fields: &[
            ("date", Expected::Words("2024-11-07")),
            ("time", Expected::Words("14:30")),
            ("room", Expected::Words("B12")),
            (
                "attendees",
                Expected::List(&["Priya Natarajan", "Tom Okafor", "Lena Vogt"]),
            ),
            ("attendee_count", Expected::Number(3)),
        ],
    },
    Text {
        name: "shipping",
        text: "Order AX2210 shipped on 03/08/2025 (day/month/year) with DHL, tracking \
               JD014600003; 3 parcels weighing 2.4 kg in all, to 14 Rue de Lyon, 75012 Paris.",
        wants: "order (text), shipped (YYYY-MM-DD), carrier (text), tracking (text), parcels \
                (whole number), weight_grams (whole number), postcode (text)",
        fields: &[
            ("order", Expected::Words("AX2210")),
            ("shipped", Expected::Words("2025-08-03")),
            ("carrier", Expected::Words("DHL")),
            ("tracking", Expected::Words("JD014600003")),
            ("parcels", Expected::Number(3)),
            ("weight_grams", Expected::Number(2400)),
            ("postcode", Expected::Words("75012")),
        ],
    },
    Text {
        name: "listing",
        text: "Senior data engineer, remote within the EU. Salary 85,000 to 95,000 GBP. Starts \
               1 January 2026; apply by 15 December 2025. Skills: Python, SQL, Spark and \
               Airflow.",
        wants: "title (text), salary_min (whole number), salary_max (whole number), currency \
                (text), start (YYYY-MM-DD), deadline (YYYY-MM-DD), skills (array of text in the \
                order given)",
        fields: &[
            ("title", Expected::Words("Senior data engineer")),
            ("salary_min", Expected::Number(85_000)),
            ("salary_max", Expected::Number(95_000)),
            ("currency", Expected::Words("GBP")),
            ("start", Expected::Words("2026-01-01")),
            ("deadline", Expected::Words("2025-12-15")),
            (
                "skills",
                Expected::List(&["Python", "SQL", "Spark", "Airflow"]),
            ),
        ],
    },
    Text {
        name: "weather",
        text: "Tuesday: a high of 23 °C and a low of 14 °C, a 60% chance of rain, wind 18 km/h \
               from the southwest, sunrise at 06:12.",
        wants: "day (text), high_c (whole number), low_c (whole number), rain_percent (whole \
                number), wind_kmh (whole number), wind_from (text, lowercase), sunrise (HH:MM)",
        fields: &[
            ("day", Expected::Words("Tuesday")),
            ("high_c", Expected::Number(23)),
            ("low_c", Expected::Number(14)),
            ("rain_percent", Expected::Number(60)),
            ("wind_kmh", Expected::Number(18)),
            ("wind_from", Expected::Words("southwest")),
            ("sunrise", Expected::Words("06:12")),
        ],
    },
];

/// What one text is asked.
#[must_use]
pub fn ask_for(text: &Text) -> String {
    format!(
        "Read this text:\n\n{}\n\nAnswer with one JSON object with exactly these keys: {}. Use \
         only what the text says. Answer with the JSON only.",
        text.text, text.wants
    )
}

/// The JSON object in an answer, if any.
#[must_use]
pub fn object_in(said: &str) -> Option<Value> {
    let (open, close) = (said.find('{')?, said.rfind('}')?);
    let held = mcf_record::json::parse(said.get(open..=close)?).ok()?;
    matches!(held, Value::Map(_)).then_some(held)
}

/// Whether a field holds what was expected, exactly.
#[must_use]
pub fn field_matches(held: Option<&Value>, expected: Expected) -> bool {
    match (held, expected) {
        (Some(Value::Integer(number)), Expected::Number(want)) => *number == want,
        (Some(Value::Text(words)), Expected::Words(want)) => words == want,
        (Some(Value::List(items)), Expected::List(want)) => {
            items.len() == want.len()
                && items.iter().zip(want).all(|(item, want)| match item {
                    Value::Text(words) => words == want,
                    Value::Integer(number) => number.to_string() == *want,
                    _ => false,
                })
        }
        _ => false,
    }
}

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each text, each trial, each field a row"
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
        "  {} text(s), {TRIALS} trial(s) each (the first greedy, the rest at temperature {}), \
         every field compared exactly",
        TEXTS.len(),
        TEMPERATURE
    )];
    let (mut fields_right, mut fields_asked, mut parsed, mut trials) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    for text in TEXTS {
        let ids = match framed_ids(&engine, &ask_for(text)) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        let mut said = Vec::with_capacity(TRIALS);
        for trial in 0..TRIALS {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let seed = u64::try_from(trial).unwrap_or(0);
            let draw = if trial == 0 {
                Draw::greedy(0)
            } else {
                Draw {
                    seed,
                    temperature: Thousandths(TEMPERATURE),
                    truncation: Truncation::OFF,
                }
            };
            let (done, ns) = timed(|| {
                engine.complete(Prompt::Identifiers(&ids), BUDGET, draw, false, site.timed())
            });
            let completed = match done {
                Ok(completed) => completed,
                Err(failure) => return Found::could_not_tell(failure.detail()),
            };
            let object = object_in(&completed.text);
            let dims = [
                ("text", Value::text(text.name)),
                ("trial", Value::Integer(as_integer(trial))),
            ];
            trials = trials.saturating_add(1);
            parsed = parsed.saturating_add(usize::from(object.is_some()));
            rows.push(Reading::new(
                &dims,
                "parsed",
                i64::from(object.is_some()),
                "bool",
            ));
            rows.push(Reading::new(
                &dims,
                "tokens",
                as_integer(completed.predicted),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "ns",
                i64::try_from(ns).unwrap_or(i64::MAX),
                "ns",
            ));
            let mut right = 0_usize;
            for (key, expected) in text.fields {
                let held = object.as_ref().and_then(|object| object.get(key));
                let matched = field_matches(held, *expected);
                right = right.saturating_add(usize::from(matched));
                rows.push(Reading::new(
                    &[
                        ("text", Value::text(text.name)),
                        ("trial", Value::Integer(as_integer(trial))),
                        ("field", Value::text(*key)),
                    ],
                    "right",
                    i64::from(matched),
                    "bool",
                ));
                rows.push(Reading::new(
                    &[
                        ("text", Value::text(text.name)),
                        ("trial", Value::Integer(as_integer(trial))),
                        ("field", Value::text(*key)),
                    ],
                    "present",
                    i64::from(held.is_some()),
                    "bool",
                ));
            }
            fields_right = fields_right.saturating_add(right);
            fields_asked = fields_asked.saturating_add(text.fields.len());
            rows.push(Reading::new(
                &dims,
                "fields_right",
                as_integer(right),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "fields",
                as_integer(text.fields.len()),
                "count",
            ));
            said.push(if object.is_some() {
                format!("{right}/{}", text.fields.len())
            } else {
                "no JSON".to_owned()
            });
        }
        lines.push(format!("  {:<10} {}", text.name, said.join("   ")));
    }
    lines.push(format!(
        "  {fields_right} of {fields_asked} field(s) exact over {trials} trial(s); JSON parsed in {parsed}"
    ));
    Found {
        lines,
        fields: vec![
            ("texts", Value::Integer(as_integer(TEXTS.len()))),
            ("trials", Value::Integer(as_integer(trials))),
            ("parsed", Value::Integer(as_integer(parsed))),
            ("fields_right", Value::Integer(as_integer(fields_right))),
            ("fields_asked", Value::Integer(as_integer(fields_asked))),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{Expected, TEXTS, ask_for, field_matches, object_in};
    use mcf_record::json::Value;

    #[test]
    fn the_object_is_found_inside_prose_and_fields_compare_exactly() {
        let said = "Here you go:\n```json\n{\"invoice\": 4471, \"vendor\": \"Northwind Traders\", \"quantities\": [3, 2]}\n```";
        let object = object_in(said).unwrap();
        assert!(field_matches(object.get("invoice"), Expected::Number(4471)));
        assert!(!field_matches(
            object.get("invoice"),
            Expected::Number(4472)
        ));
        assert!(field_matches(
            object.get("vendor"),
            Expected::Words("Northwind Traders")
        ));
        assert!(!field_matches(
            object.get("vendor"),
            Expected::Words("northwind traders")
        ));
        assert!(field_matches(
            object.get("quantities"),
            Expected::List(&["3", "2"])
        ));
        assert!(!field_matches(
            object.get("quantities"),
            Expected::List(&["3"])
        ));
        assert!(!field_matches(None, Expected::Number(1)));
        assert!(!field_matches(
            Some(&Value::text("4471")),
            Expected::Number(4471)
        ));
        assert!(object_in("no braces here").is_none());
    }

    #[test]
    fn every_text_asks_for_every_field_it_checks() {
        for text in TEXTS {
            let asked = ask_for(text);
            for (key, _) in text.fields {
                assert!(asked.contains(key), "{} lacks {key}", text.name);
            }
        }
    }
}
