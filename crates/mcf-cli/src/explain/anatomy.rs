use mcf_standin::anatomy::blocks::{Census, ranges};
use mcf_standin::anatomy::vocabulary::{self, Vocabulary};
use mcf_standin::anatomy::work::{self, Cache};
use mcf_standin::anatomy::{self, Agreement, Anatomy, Share};
use mcf_standin::gguf::Model;

const VALUE_WIDTH: usize = 80;

fn row(label: &str, text: &str) -> Vec<String> {
    super::wrapped(text, VALUE_WIDTH)
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let label = if index == 0 { label } else { "" };
            format!("  {label:<38}{line}")
        })
        .collect()
}

pub(crate) fn counted(file: &Model) -> Vec<String> {
    let body = anatomy::of(file);
    let mut lines = Vec::new();
    lines.extend(row(
        "parameters, counted",
        &format!(
            "{} ({}B) across {} tensors in {} blocks",
            with_thousands(body.elements),
            anatomy::billions(body.elements),
            file.tensors.len(),
            body.blocks
        ),
    ));
    if let Some(active) = &body.active {
        lines.extend(row(
            "of which a token activates",
            &format!(
                "{} ({}B): {} of {} experts per block, as the header says the router picks",
                with_thousands(active.elements),
                anatomy::billions(active.elements),
                active.used,
                active.experts
            ),
        ));
    }
    lines.extend(row(
        "weight bytes",
        &match (body.bytes, body.unsized_tensors) {
            (Some(bytes), _) => format!(
                "{} — {} per weight over the whole file",
                with_thousands(bytes),
                bits(&Share {
                    tensors: 0,
                    elements: body.elements,
                    bytes: Some(bytes)
                })
            ),
            (None, without_a_size) => format!(
                "not totalled: {without_a_size} tensor(s) are encoded in a way MCF does not size, and a \
                 total with a hole in it is not one (A7)"
            ),
        },
    ));
    lines.extend(row(
        "output head",
        if body.output_tied {
            "reuses the embedding table — no output.weight in the directory"
        } else {
            "its own table"
        },
    ));
    lines.extend(by_part(&body));
    lines.extend(by_encoding(&body));
    lines.extend(by_block(&body));
    lines
}

fn by_block(body: &Anatomy) -> Vec<String> {
    let census: &Census = &body.census;
    let mut lines = Vec::new();
    let mut label = "by block";
    for family in &census.families {
        let bits = match family.bits {
            Some((least, most)) if least == most => hundredths(least),
            Some((least, most)) => format!("{}–{}", hundredths(least), hundredths(most)),
            None => "unsized".to_owned(),
        };
        lines.push(format!(
            "  {label:<38}{} block(s), {}: {} ({}) at {bits} bits",
            family.blocks.len(),
            ranges(&family.blocks, 6),
            with_thousands(family.share.elements),
            percent(family.share.elements, body.elements),
        ));
        lines.extend(row("", &format!("  {}", family.shape.said())));
        label = "";
    }
    if census.families.len() > 1 {
        lines.extend(row(
            "",
            &format!(
                "{} shapes of block, read off the tensors each holds; the header's block \
                 count says only how many there are",
                census.families.len()
            ),
        ));
    }
    lines
}

fn hundredths(held: u64) -> String {
    #[allow(
        clippy::integer_division,
        reason = "hundredths into a whole and a fraction; nothing is lost"
    )]
    let (whole, fraction) = (held / 100, held % 100);
    format!("{whole}.{fraction:02}")
}

fn by_part(body: &Anatomy) -> Vec<String> {
    let mut lines = Vec::new();
    let mut label = "by part";
    for (role, share) in &body.roles {
        lines.push(format!(
            "  {label:<38}{:<18}{:>16} {:>6}  {}",
            role.as_str(),
            with_thousands(share.elements),
            percent(share.elements, body.elements),
            bits(share),
        ));
        label = "";
    }
    lines
}

fn by_encoding(body: &Anatomy) -> Vec<String> {
    let mut lines = Vec::new();
    let mut label = "by encoding";
    for (kind, share) in &body.kinds {
        lines.push(format!(
            "  {label:<38}{:<18}{:>16} {:>6}  {} in {} tensor(s)",
            kind.to_string(),
            with_thousands(share.elements),
            percent(share.elements, body.elements),
            bits(share),
            share.tensors,
        ));
        label = "";
    }
    lines
}

pub(crate) fn agreed(file: &Model) -> Vec<String> {
    let body = anatomy::of(file);
    let mut lines = vec![
        "WHAT THE HEADER DECLARES, AGAINST WHAT THE DIRECTORY HOLDS  (A21: declared beside \
         observed)"
            .to_owned(),
    ];
    let mut disagreeing = 0_u64;
    let mut unshown = 0_u64;
    for row in &body.agreements {
        lines.push(agreement_line(row));
        match row.agrees {
            Some(false) => disagreeing = disagreeing.saturating_add(1),
            None => unshown = unshown.saturating_add(1),
            Some(true) => {}
        }
    }
    lines.extend(row(
        "",
        &match (disagreeing, unshown) {
            (0, 0) => "every figure the header declares, the directory bears out".to_owned(),
            (0, unshown) => format!(
                "every figure both sides state agrees; {unshown} could not be compared, and that \
                 is not a disagreement"
            ),
            (disagreeing, _) => format!(
                "{disagreeing} figure(s) the header declares are not what its own directory \
                 holds — the shapes are what an engine multiplies, and the header is what it \
                 reads first"
            ),
        },
    ));
    lines
}

fn agreement_line(row: &Agreement) -> String {
    let declared = row.declared.as_deref().map_or_else(
        || "the header does not say".to_owned(),
        |held| format!("declared {held}"),
    );
    let observed = row.observed.as_deref().map_or_else(
        || "not read from the directory".to_owned(),
        |held| format!("found {held}"),
    );
    let verdict = match row.agrees {
        Some(true) => "agree",
        Some(false) => "DISAGREE",
        None => "not compared",
    };
    format!(
        "  {:<38}{:<28}{:<40}{verdict}",
        row.what, declared, observed
    )
}

pub(crate) fn costed(file: &Model) -> Vec<String> {
    let body = anatomy::of(file);
    let work = work::of(file, &body);
    let mut lines = vec![
        "WHAT ONE TOKEN COSTS, IN ARITHMETIC  (A20: computed from the header, measured by \
         nothing here)"
            .to_owned(),
    ];
    lines.extend(row(
        "multiply-adds through the weights",
        &format!(
            "{} ({}B) — the weights it passes through, the embedding lookup excluded",
            with_thousands(work.multiply_adds),
            anatomy::billions(work.multiply_adds)
        ),
    ));
    if let Some(width) = work.head_width {
        lines.extend(row(
            "attention head width",
            &format!(
                "{width}{}",
                work.queries_per_key.map_or_else(String::new, |ratio| {
                    format!(", {ratio} query head(s) to each key/value head")
                })
            ),
        ));
    }
    if let Some(added) = work.attention_at_context {
        lines.extend(row(
            "attention over the context",
            &format!(
                "{} ({}B) more for the last token of a full declared context{}",
                with_thousands(added),
                anatomy::billions(added),
                if body.census.attending == body.blocks {
                    String::new()
                } else {
                    format!(
                        ", in the {} of {} blocks that attend",
                        body.census.attending, body.blocks
                    )
                }
            ),
        ));
    }
    lines.extend(cache_lines(&work.cache));
    lines
}

fn cache_lines(cache: &Cache) -> Vec<String> {
    match cache {
        Cache::Sized {
            per_token,
            key_heads,
            at_context,
            sliding_window,
            attending,
            recurrent,
            ..
        } => {
            let (keeping, blocks) = *attending;
            let mut lines = row(
                "key/value cache",
                &format!(
                    "{} bytes per token at 16 bits an element: {key_heads} head(s) keeping {}{}",
                    with_thousands(*per_token),
                    cache.kept().unwrap_or_default(),
                    if keeping == blocks {
                        String::new()
                    } else {
                        format!(
                            ", across the {keeping} of {blocks} blocks that keep keys{}",
                            if *recurrent > 0 {
                                format!(
                                    " — the other {recurrent} keep a fixed recurrent state, \
                                     which the header's attention widths do not size"
                                )
                            } else {
                                String::new()
                            }
                        )
                    }
                ),
            );
            if let Some((tokens, bytes)) = at_context {
                lines.extend(row(
                    "",
                    &format!(
                        "{} bytes ({}) for the declared {} tokens{}",
                        with_thousands(*bytes),
                        gibibytes(*bytes),
                        with_thousands(*tokens),
                        sliding_window.map_or_else(String::new, |window| {
                            format!(
                                " — at most: a sliding window of {window} bounds what some \
                                 blocks keep"
                            )
                        })
                    ),
                ));
            }
            lines
        }
        Cache::Unsized(why) => row("key/value cache", &format!("not sized: {why}")),
    }
}

pub(crate) fn spoken(file: &Model) -> Vec<String> {
    let held = vocabulary::of(file);
    let mut lines = vec![
        "WHAT THE VOCABULARY IS  (counted from the header's token list; nothing tokenised, \
         nothing rated)"
            .to_owned(),
    ];
    lines.extend(row("segmentation", &held.segmentation()));
    lines.extend(row(
        "tokens",
        &format!(
            "{}{}",
            with_thousands(held.tokens),
            held.merges.map_or_else(String::new, |merges| format!(
                ", from {} merges",
                with_thousands(merges)
            ))
        ),
    ));
    lines.extend(row(
        "by kind",
        &held.kinds.as_ref().map_or_else(
            || "the file does not type its tokens".to_owned(),
            |kinds| {
                kinds
                    .iter()
                    .map(|(kind, count)| format!("{} {}", with_thousands(*count), kind.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ),
    ));
    lines.extend(row(
        "begin a word",
        &format!(
            "{} ({})",
            with_thousands(held.word_starts),
            percent(held.word_starts, held.tokens)
        ),
    ));
    lines.extend(row("digit runs", &held.digits_said()));
    if let Some((token, bytes)) = &held.longest {
        lines.push(format!(
            "  {:<38}{bytes} bytes: {}",
            "longest token",
            clipped(token)
        ));
    }
    lines.extend(named_lines(&held));
    lines.extend(row("beginning token added", held.beginning_said()));
    lines.extend(template_lines(&held));
    lines
}

fn named_lines(held: &Vocabulary) -> Vec<String> {
    let mut lines = Vec::new();
    let mut label = "named tokens";
    for named in &held.named {
        let spelled = held
            .beyond(named)
            .unwrap_or_else(|| named.spelled.as_deref().map_or_else(String::new, clipped));
        lines.push(format!(
            "  {label:<38}{:<20}{:>8}  {spelled}",
            named.what, named.identifier
        ));
        label = "";
    }
    lines
}

fn template_lines(held: &Vocabulary) -> Vec<String> {
    let Some(template) = &held.template else {
        return row("chat template", vocabulary::NO_TEMPLATE);
    };
    let mut lines = row(
        "chat template",
        &format!(
            "{} bytes{}",
            with_thousands(template.bytes),
            if template.mentions.is_empty() {
                String::new()
            } else {
                format!(", mentioning {}", template.mentions.join(", "))
            }
        ),
    );
    lines.extend(row(
        "markers it frames a turn with",
        &template.no_markers().map_or_else(
            || {
                template
                    .markers
                    .iter()
                    .flatten()
                    .map(|marker| clipped(marker))
                    .collect::<Vec<_>>()
                    .join(" ")
            },
            str::to_owned,
        ),
    ));
    lines
}

fn clipped(token: &str) -> String {
    const LONGEST_SHOWN: usize = 48;
    let spelled = format!("{token:?}");
    if spelled.chars().count() <= LONGEST_SHOWN {
        return spelled;
    }
    let kept: String = spelled.chars().take(LONGEST_SHOWN).collect();
    format!("{kept}…")
}

fn gibibytes(bytes: u64) -> String {
    #[allow(
        clippy::integer_division,
        reason = "one decimal is the resolution shown"
    )]
    let tenths = bytes / (1 << 30) * 10 + (bytes % (1 << 30)) * 10 / (1 << 30);
    #[allow(
        clippy::integer_division,
        reason = "split into the whole and the tenth"
    )]
    let (whole, tenth) = (tenths / 10, tenths % 10);
    format!("{whole}.{tenth} GiB")
}

fn bits(share: &Share) -> String {
    share.hundredths_of_a_bit().map_or_else(
        || "unsized".to_owned(),
        |hundredths| {
            #[allow(
                clippy::integer_division,
                reason = "hundredths into a whole and a fraction; nothing is lost"
            )]
            let (whole, fraction) = (hundredths / 100, hundredths % 100);
            format!("{whole}.{fraction:02} bits")
        },
    )
}

fn percent(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "—".to_owned();
    }
    #[allow(
        clippy::integer_division,
        reason = "one decimal is the resolution shown"
    )]
    let tenths = part.saturating_mul(1000) / whole;
    #[allow(
        clippy::integer_division,
        reason = "the same, split into the whole and the tenth"
    )]
    let (whole, tenth) = (tenths / 10, tenths % 10);
    format!("{whole}.{tenth}%")
}

pub(crate) fn with_thousands(number: u64) -> String {
    anatomy::grouped(number)
}

#[cfg(test)]
mod tests {
    use super::{percent, with_thousands};

    #[test]
    fn thousands_are_separated() {
        assert_eq!(with_thousands(0), "0");
        assert_eq!(with_thousands(999), "999");
        assert_eq!(with_thousands(1_000), "1,000");
        assert_eq!(with_thousands(8_250_000_000), "8,250,000,000");
    }

    #[test]
    fn a_share_is_one_decimal_and_never_rounded_up() {
        assert_eq!(percent(1, 3), "33.3%");
        assert_eq!(percent(2, 3), "66.6%");
        assert_eq!(percent(0, 0), "—");
        assert_eq!(percent(5, 5), "100.0%");
    }
}
