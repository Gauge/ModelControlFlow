//! What a prompt does to a model, measured rather than judged (§3.8, A19, A7).
//!
//! **Not *is this a good prompt*.** That is a judgement, it needs a rater, and
//! §XIII says whose work that is. What is observable without one is narrower
//! and more useful than it sounds: which parts of a prompt the model actually
//! used, and whether the prompt settles the answer at all.
//!
//! **Ablation is the load-bearing measurement, and what it can carry was
//! measured rather than assumed.** Take the prompt apart into the sentences a
//! person wrote, remove one, and ask again with the seed held still; how much
//! of the answer moves is a fact about this prompt on this model, checkable by
//! anyone.
//!
//! **What it will not tell you is that a sentence was irrelevant.** Under
//! greedy decoding, removing any sentence from the middle of a prompt shifts
//! everything after it, and the answer is written differently whether or not
//! the sentence steered it. Measured here: an instruction scored 94.8%, an
//! obviously irrelevant sentence in the same prompt scored 50.6%, and a
//! control sentence carrying no instruction — inserted and then removed, the
//! same operation — put the floor at 14.0%. The irrelevant sentence sat far
//! above its own floor. Continuation drift dominates, and no threshold
//! recovers the distinction.
//!
//! So what this reports is an **ordering**, and it says so: the sentence that
//! steered the answer stands out from the ones that did not, which is the
//! question people actually ask of their own prompts. Reading the figure as
//! *relevance* would be reading drift as meaning. Separating the two needs a
//! comparison that is not text — whether the answer still satisfies what the
//! prompt asked for, which is the laboratory's mechanic, or the model's own
//! distribution over the prompt (B-373).
//!
//! **So a second measurement asks the model directly, without generating.**
//! The baseline's own opening is put after the shortened prompt, token by
//! token, and at each position the model is asked where it ranks the token the
//! baseline actually had there. Nothing is generated and nothing drifts: what
//! comes back is whether the model would still have *begun* the same answer
//! without the sentence, and how far its first token fell if not. On a
//! one-word answer, where every removal that changes the word scores the same
//! hundred per cent, this is what orders them — a sentence whose absence
//! dropped the answer's first token to its seventeenth choice did more than
//! one that dropped it to its fourth (measured: B-429).
//!
//! **What is taken apart is one prompt, whole.** A persona or an instruction
//! sheet is what a person crafts, and every part of it is a part the report
//! weighs — a closing line that says *begin the adventure* is as much a
//! sentence of the prompt as the persona above it, and nothing is held out
//! of the ablation. The text is taken apart by paragraph where it has
//! paragraphs, by sentence where it does not, or as the caller says, and
//! every variant goes in the one turn the addressing probe found: MCF has not
//! probed for a system turn (D43) and does not assume one here. The cap on
//! how many parts are removed is the caller's, because forty paragraphs is
//! forty generations and whether that is worth it is a choice about their
//! time, not a constant (§3.15, B-430).
//!
//! **Settledness is the other half, and it is asked only at a temperature the
//! caller states.** The same prompt under several seeds either produces the
//! same answer or does not. Several different answers means the prompt
//! underdetermines the answer *for this model* — again a fact, and again not a
//! verdict: an open question deserves several answers, and a specification
//! does not. But every other generation here is greedy, and under a greedy
//! sampler the seed changes nothing: two more generations at temperature 0
//! measured nothing and cost two generations, and the line that said so was
//! honest and wasteful (F147, B-431). So the seeds are drawn at a temperature
//! only when the caller has stated one — B60 forbids MCF a house temperature —
//! and when none is stated no generation is spent on the question, and the
//! report says it was not asked rather than that the answer settled (A7).
//!
//! **Cross-checked by:** the two measurements are counting, and what needs
//! checking is what is counted. `prompt/tests` pins that an unchanged answer is
//! reported as *not used* rather than as *wrong*, that a clause list is the
//! sentences a person wrote and not the tokenizer's pieces, and that a prompt
//! of one sentence is refused as having nothing to ablate rather than reported
//! as fully used. There is no arithmetic here beyond a count of distinct
//! strings (A19).
//!
//! **What this costs.** One generation for the baseline, one per clause, and
//! one per extra seed; the forced reading is one request a token of the
//! opening for each clause, cached, and generates nothing. A prompt of six sentences under three seeds is nine
//! generations, which is why this is a thing somebody asks for rather than
//! something that happens on the way past (§3.8).

use mcf_core::configuration::Thousandths;

pub use crate::generation::{Draw, Stated, Truncation, Whose};

/// How many seeds the settledness question is asked under, when it is asked.
///
/// Three, because two cannot tell *the same twice* from *a coincidence* and
/// the cost is a generation each.
pub const SEEDS: usize = 3;

/// A sentence that carries no instruction, used to find the floor.
///
/// **Removing anything moves the answer.** Under greedy decoding a prompt one
/// sentence shorter is a different context, and what follows is written
/// differently whether or not the sentence mattered — so every clause reads as
/// influential and the measurement says nothing. What separates signal from
/// that is knowing how much the answer moves for a sentence that could not
/// have steered it.
///
/// So one is added and the answer measured against the baseline. It is flat,
/// carries nothing to act on, and is about the room rather than the task —
/// a sentence a model has nothing to do with (F: an irrelevant sentence in a
/// real prompt scored 50.6% against a real instruction's 94.8%, and only the
/// ordering of the two was informative).
pub const NO_INSTRUCTION: &str = "The room is quiet.";

/// How many tokens of the baseline's opening are put to the model under each
/// shortened prompt.
///
/// Twelve is an opening: a word or a short sentence, enough to see whether the
/// answer set off the same way and short enough that a report of eight clauses
/// is under a hundred requests, each one cached and generating nothing.
pub const MOST_FORCED: usize = 12;

/// The most parts that will be ablated unless the caller says otherwise.
///
/// A long prompt is a long run of generations, and a report that took an hour
/// is one nobody asked for. What is over the cap is said rather than silently
/// dropped (A7), and the cap itself is the caller's to raise ([`Taken::most`]).
pub const MOST_CLAUSES: usize = 8;

/// What a document is taken apart into.
///
/// A person asking *which words to use* wants the answer by word or by
/// phrase; a persona ablated by word is hundreds of generations about text
/// whose units are its paragraphs. Which is which is decided from the text
/// where the caller does not say ([`Unit::for_text`]), and the report says
/// which it was and who decided (§3.15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// A word: ended by whitespace. Punctuation stays with the word it is
    /// attached to, as a sentence keeps its full stop.
    Word,
    /// A phrase: a sentence, or the part of one ended by `,`, `;` or `:`
    /// before whitespace — *in one word*, *as a senior engineer*. A cut
    /// that would leave fewer than three words on either side is not made:
    /// *a cold, dripping cave* is one phrase, not a phrase and a fragment.
    Phrase,
    /// A sentence: ended by `.`, `?`, `!` before whitespace, or a line break.
    Sentence,
    /// A paragraph: ended by a blank line.
    Paragraph,
}

impl Unit {
    /// Every unit, finest first.
    pub const ALL: [Self; 4] = [Self::Word, Self::Phrase, Self::Sentence, Self::Paragraph];

    /// The unit a text is written in: paragraphs where a blank line separates
    /// two of them, phrases otherwise — the unit an instruction is written
    /// in, and the one a question about *which words* is answered at.
    #[must_use]
    pub fn for_text(text: &str) -> Self {
        if parts_of(text, Self::Paragraph).len() > 1 {
            Self::Paragraph
        } else {
            Self::Phrase
        }
    }

    /// The word on the wire and in a report.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Phrase => "phrase",
            Self::Sentence => "sentence",
            Self::Paragraph => "paragraph",
        }
    }

    /// The unit a word names, if it names one.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "word" | "words" => Some(Self::Word),
            "phrase" | "phrases" => Some(Self::Phrase),
            "sentence" | "sentences" => Some(Self::Sentence),
            "paragraph" | "paragraphs" => Some(Self::Paragraph),
            _ => None,
        }
    }
}

/// One part of a document, and the whitespace that followed it.
///
/// **The separator travels with the part** so that a document with one part
/// removed is the document as written, less that part — a bullet list stays a
/// bullet list, paragraphs stay paragraphs. An earlier cut rejoined sentences
/// with a space, so every ablation of a multi-line prompt was also a
/// reformatting of it, and the baseline was the only variant with its line
/// breaks (F: found when the field took a document, B-430).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The part, as written, trimmed at both ends.
    pub text: String,
    /// What lay between it and the next part: whitespace, and any run of
    /// punctuation that was not a part by itself. Empty after the last.
    pub after: String,
}

/// What is taken apart, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Taken<'a> {
    /// The prompt: the text whose parts are removed in turn.
    pub text: &'a str,
    /// What to take the document apart into; `None` lets the text decide.
    pub by: Option<Unit>,
    /// How many parts to remove at most; `None` is [`MOST_CLAUSES`].
    pub most: Option<usize>,
    /// The further readings asked for, each costing generations; none
    /// unless asked (§3.15).
    pub extras: Extras,
}

/// A further reading of the prompt, asked for by name: each costs
/// generations, so none is taken unasked (§3.15), and each is served as
/// null where it was not (A7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Extra {
    /// The inert sentence put at every position rather than one, so that
    /// the floor is a spread and not a draw (B-434). A generation a
    /// position.
    Floors,
    /// Each part asked as the whole prompt in turn, to say which of them
    /// carries the answer on its own (B-435). A generation a part and one
    /// for the control.
    Alone,
    /// The prompt grown a part at a time from the front — the first part,
    /// the first two, and on to one short of the whole — to say where the
    /// answer becomes the answer (B-436). A generation a prefix.
    Prefixes,
    /// Each pair of neighbouring parts swapped in turn, to say whether
    /// the answer is carried by what the parts say or by where they sit
    /// (B-437). A generation a swap.
    Swaps,
    /// The same parts in another form: on one line, as bullets, as a
    /// numbered list, under headings, in tags and in capitals — the words
    /// kept and only their dress changed, to say whether the model follows
    /// the formatting or the words (B-444). A generation a form, less any
    /// the prompt is already in.
    Forms,
}

impl Extra {
    /// Every reading there is, in the order they are asked and reported.
    pub const ALL: [Self; 5] = [
        Self::Floors,
        Self::Alone,
        Self::Prefixes,
        Self::Swaps,
        Self::Forms,
    ];

    /// The name a flag and the wire use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Floors => "floors",
            Self::Alone => "alone",
            Self::Prefixes => "prefixes",
            Self::Swaps => "swaps",
            Self::Forms => "forms",
        }
    }

    /// The reading a name means, or `None` where it names nothing.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|extra| extra.name() == name)
    }

    /// What the reading costs, in generations, on a prompt of `parts` parts
    /// of which the first `removed` are removed in turn. The one place this
    /// is counted, so a forecast and the bill cannot disagree (B-072).
    #[must_use]
    pub const fn generations(self, parts: usize, removed: usize) -> usize {
        match self {
            // One a position, less the one the single draw already takes.
            Self::Floors => parts,
            // One a part removed, and the control alone.
            Self::Alone => removed.saturating_add(1),
            // One a prefix short of the whole, as many as parts removed:
            // the whole is the baseline, already drawn. One a neighbouring
            // pair, likewise: the part removed and the one after it change
            // places, and the last part has nothing after it.
            Self::Prefixes | Self::Swaps => strict_prefixes(parts, removed),
            // One a form, and this is the most: a form the prompt is
            // already in is not asked, and the page says which. One part
            // is nothing to list, so only the two forms of a whole are
            // left to it.
            Self::Forms => {
                if parts > 1 {
                    Form::ALL.len()
                } else {
                    Form::OF_A_WHOLE
                }
            }
        }
    }

    /// Whether the count is a ceiling rather than the bill: a form the
    /// prompt is already written in costs nothing, so the forecast for the
    /// forms is *at most* (B-444).
    #[must_use]
    pub const fn at_most(self) -> bool {
        matches!(self, Self::Forms)
    }

    /// What the reading is, for a button or a line: what it asks, in a
    /// sentence a reader chooses by.
    #[must_use]
    pub const fn asks(self) -> &'static str {
        match self {
            Self::Floors => {
                "the control sentence at every position, so the floor is a spread rather than \
                 one draw"
            }
            Self::Alone => {
                "each part as the whole prompt in turn, to say which carries the answer on its \
                 own"
            }
            Self::Prefixes => {
                "the prompt grown a part at a time from the front, to say where the answer \
                 becomes the answer"
            }
            Self::Swaps => {
                "each part swapped with the one after it, to say whether the answer follows \
                 what the parts say or where they sit"
            }
            Self::Forms => {
                "the same parts on one line, as bullets, numbered, under headings, in tags \
                 and in capitals, to say whether the model follows the form or the words"
            }
        }
    }

    /// The reading's name on a button: a few words a reader picks it by.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Floors => "floor at every position",
            Self::Alone => "each part alone",
            Self::Prefixes => "prompt grown from the front",
            Self::Swaps => "neighbours swapped",
            Self::Forms => "the same words in six forms",
        }
    }

    /// What its generations are spent on, for a cost line that reads "N
    /// more" and then this.
    #[must_use]
    pub const fn spent_on(self) -> &'static str {
        match self {
            Self::Floors => "for the control at every other position",
            Self::Alone => "for each part alone and the control alone",
            Self::Prefixes => "for the prompt grown a part at a time, short of the whole",
            Self::Swaps => "for each part swapped with the one after it",
            Self::Forms => "for the parts in each form the prompt is not already in",
        }
    }

    const fn bit(self) -> u8 {
        match self {
            Self::Floors => 1,
            Self::Alone => 2,
            Self::Prefixes => 4,
            Self::Swaps => 8,
            Self::Forms => 16,
        }
    }
}

/// How many prefixes short of the whole are read: one a part removed, and
/// never the whole itself, whose answer is the baseline. The count of
/// neighbouring pairs is the same figure: a part and the one after it, for
/// every part but the last.
const fn strict_prefixes(parts: usize, removed: usize) -> usize {
    let short = parts.saturating_sub(1);
    if removed < short { removed } else { short }
}

/// Which further readings were asked for: a set of [`Extra`], small enough
/// to copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extras(u8);

impl Extras {
    /// None asked for.
    pub const NONE: Self = Self(0);

    /// Whether this reading was asked for.
    #[must_use]
    pub const fn has(self, extra: Extra) -> bool {
        self.0 & extra.bit() != 0
    }

    /// This set with the reading asked for, or not.
    #[must_use]
    pub const fn with(self, extra: Extra, asked: bool) -> Self {
        if asked {
            Self(self.0 | extra.bit())
        } else {
            Self(self.0 & !extra.bit())
        }
    }

    /// What the readings asked for cost together, in generations.
    #[must_use]
    pub fn generations(self, parts: usize, removed: usize) -> usize {
        self.asked().fold(0, |sum, extra| {
            sum.saturating_add(extra.generations(parts, removed))
        })
    }

    /// The readings asked for, in order.
    pub fn asked(self) -> impl Iterator<Item = Extra> {
        Extra::ALL.into_iter().filter(move |extra| self.has(*extra))
    }

    /// The set the names mean; a name that means nothing is ignored, so
    /// the caller that reads a wire says what it could not read.
    pub fn named<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        names
            .into_iter()
            .filter_map(Extra::named)
            .fold(Self::NONE, |set, extra| set.with(extra, true))
    }
}

impl Taken<'_> {
    /// The unit this will be taken apart by, and whether the caller chose it.
    #[must_use]
    pub fn unit(&self) -> (Unit, bool) {
        self.by
            .map_or_else(|| (Unit::for_text(self.text), false), |by| (by, true))
    }

    /// The cap on parts removed.
    #[must_use]
    pub fn cap(&self) -> usize {
        self.most.unwrap_or(MOST_CLAUSES).max(1)
    }

    /// The parts, in the unit this is taken apart by.
    #[must_use]
    pub fn parts(&self) -> Vec<Part> {
        parts_of(self.text, self.unit().0)
    }
}

/// Whether the model would still have begun the same answer.
///
/// **Teacher-forced, so nothing drifts.** The ablation compares two answers,
/// and under greedy decoding two answers part at their first difference and
/// are written differently from there on — which is why that column is an
/// ordering. This puts the baseline's own opening after the shortened prompt
/// and asks, at each token, where the model ranks the token the baseline had
/// there. The answer is a rank, not a comparison of texts, and it does not
/// depend on what was written after the parting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Held {
    /// Where the model ranked the baseline's first token under this prompt,
    /// counting from one. `None` where it was outside the depth read — a
    /// bound rather than an absence (A7); the report says the depth.
    pub first: Option<usize>,
    /// How many of the opening's `of` tokens were still the model's first
    /// choice: the same answer would have begun the same way for this many
    /// tokens.
    pub kept: usize,
    /// How many tokens of the opening were asked about.
    pub of: usize,
}

/// One sentence of the prompt, and what happened without it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    /// The sentence, as the person wrote it.
    pub text: String,
    /// The answer when this sentence was left out.
    pub without: String,
    /// Whether leaving it out changed the answer at all.
    pub changed: bool,
    /// How much of the answer moved without it, in parts per million of the
    /// longer of the two answers.
    ///
    /// **Whether it changed is too blunt to be the measurement.** Removing any
    /// text shifts what follows it, so under greedy decoding almost every
    /// removal changes the answer *somehow* — and a report that said *used* for
    /// every sentence would tell a person nothing. What separates a sentence
    /// that steered the answer from one that merely perturbed it is how much
    /// moved: a word or two against half the reply.
    ///
    /// Word-level, because a person writes words and a difference counted in
    /// tokens would be a difference about the vocabulary (F: found by putting
    /// an irrelevant sentence into a prompt and watching it report as used).
    pub moved: u64,
    /// Whether the model would still have begun the baseline's answer without
    /// this sentence. `None` where the reading could not be taken, which the
    /// report says separately from a rank (A7).
    pub held: Option<Held>,
    /// How many tokens the model spent thinking before its answer, without
    /// this sentence — so that a part which costs a long thought is visible
    /// beside one that changes what is said (B-455).
    pub thought: Option<usize>,
}

/// The condition the settledness seeds are drawn under: the caller's
/// temperature, and the cut the file recommends or none (B-440).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settle {
    /// The temperature the caller stated.
    pub temperature: Thousandths,
    /// How the distribution is cut before each draw.
    pub truncation: Truncation,
}

/// What several seeds made of the same prompt, at a stated temperature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    /// The temperature every seed was drawn at: the caller's, never MCF's
    /// (B60), and the condition every figure below is under (§3.4).
    pub temperature: Thousandths,
    /// How every seed's draw was cut: as the file declared, or not at all —
    /// stated on every request so the engine filled nothing in (B-440).
    pub truncation: Truncation,
    /// How many seeds were asked.
    pub asked: usize,
    /// How many distinct answers came back.
    ///
    /// One means the prompt settles the answer for this model under these
    /// conditions. More than one does not mean the prompt is bad.
    pub distinct: usize,
    /// How far apart the two farthest of the sampled answers are, in parts
    /// per million of their words — [`moved_by`], over every pair.
    ///
    /// The count says *how many*; this says *how different*. Three answers
    /// that differ in a word each are three distinct answers and a settled
    /// prompt; three that share nothing are three distinct answers and are not.
    pub spread: u64,
    /// How far the farthest sample sits from the greedy answer, the same way.
    ///
    /// The greedy answer is what every ablation was compared against, so this
    /// is what sampling does to the thing the rest of the report is about.
    pub from_greedy: u64,
}

/// The floor taken at one position: the inert sentence put in before the
/// part at `position` (or at the end, where `position` is the part count),
/// and what its removal did (B-434).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorAt {
    /// Where the inert sentence was put: before this part, or at the end.
    pub position: usize,
    /// How much the answer moved for it, in parts per million.
    pub moved: u64,
    /// The forced reading with it in.
    pub held: Option<Held>,
}

/// The least, the middle and the most of the floor across positions.
///
/// **A floor is a draw, and a draw has a spread.** One inert sentence at one
/// position gave 84.9% on a persona; whether a part at 83.6% sits under the
/// floor or under that draw of it is what this answers. The middle is the
/// upper median, so it is one of the draws and not a number between two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    /// The smallest floor any position gave.
    pub least: u64,
    /// The median draw.
    pub middle: u64,
    /// The largest.
    pub most: u64,
}

/// One variant of the prompt, asked and read against the answer as written:
/// how far the answer moved, whether it still began the same way, and what
/// it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// How much of the answer moved, in parts per million of the longer.
    pub moved: u64,
    /// The forced reading under this prompt.
    pub held: Option<Held>,
    /// The answer, as the model wrote it.
    pub answer: String,
    /// What the model spent thinking before it, where that was counted.
    pub thought: Option<usize>,
}

/// A form the same parts can be written in: the words kept, their dress
/// changed (B-444).
///
/// A writer choosing between a paragraph and a list wants to know whether
/// this model reads the list *as* a list. Every form here keeps every word
/// of every part and changes only what is around them, so what moves under
/// a form is the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Every part on one line, one space between them, no line breaks.
    OneLine,
    /// A bullet a part: `- part`.
    Bullets,
    /// A numbered item a part: `1. part`.
    Numbered,
    /// A numbered heading over each part: `## 1`, then the part.
    Headings,
    /// Each part inside an `<instruction>` tag, one a line.
    Tags,
    /// The prompt as written, every letter a capital.
    Capitals,
}

impl Form {
    /// Every form, in the order they are read and reported.
    pub const ALL: [Self; 6] = [
        Self::OneLine,
        Self::Bullets,
        Self::Numbered,
        Self::Headings,
        Self::Tags,
        Self::Capitals,
    ];

    /// How many of the forms a document of one part can still take: one
    /// line and capitals, which dress the whole rather than list its parts.
    pub const OF_A_WHOLE: usize = 2;

    /// The name on the wire and in a report.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::OneLine => "one line",
            Self::Bullets => "bullets",
            Self::Numbered => "numbered",
            Self::Headings => "headings",
            Self::Tags => "tags",
            Self::Capitals => "capitals",
        }
    }

    /// The form a name means, or `None` where it names nothing.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|form| form.name() == name)
    }

    /// Whether the form lists the parts, which takes two of them, or
    /// dresses the whole, which takes one.
    #[must_use]
    pub const fn lists(self) -> bool {
        !matches!(self, Self::OneLine | Self::Capitals)
    }

    /// The parts in this form, or `None` where they cannot be put in it:
    /// one part is nothing to list.
    ///
    /// A part's own line breaks are folded to spaces in every form but
    /// capitals, because a bullet with a paragraph break inside it is not a
    /// bullet; capitals keep the document as written, because the case is
    /// all that form changes.
    #[must_use]
    pub fn render(self, parts: &[Part]) -> Option<String> {
        if parts.is_empty() || (self.lists() && parts.len() < 2) {
            return None;
        }
        let flat = |part: &Part| part.text.split_whitespace().collect::<Vec<_>>().join(" ");
        let listed = |dressed: &dyn Fn(usize, String) -> String, between: &str| {
            parts
                .iter()
                .enumerate()
                .map(|(at, part)| dressed(at.saturating_add(1), flat(part)))
                .collect::<Vec<_>>()
                .join(between)
        };
        Some(match self {
            Self::OneLine => parts.iter().map(flat).collect::<Vec<_>>().join(" "),
            Self::Bullets => listed(&|_, text| format!("- {text}"), "\n"),
            Self::Numbered => listed(&|number, text| format!("{number}. {text}"), "\n"),
            Self::Headings => listed(&|number, text| format!("## {number}\n\n{text}"), "\n\n"),
            Self::Tags => listed(
                &|_, text| format!("<instruction>{text}</instruction>"),
                "\n",
            ),
            Self::Capitals => joined(parts).to_uppercase(),
        })
    }
}

/// How many of the forms these parts would be asked in: every form they
/// can be put in that is not the prompt as written. The bill, where the
/// text is in hand; [`Extra::generations`] is the ceiling where it is not.
#[must_use]
pub fn forms_asked(parts: &[Part]) -> usize {
    let written = joined(parts);
    Form::ALL
        .into_iter()
        .filter(|form| {
            form.render(parts)
                .is_some_and(|rendered| rendered != written)
        })
        .count()
}

/// What one form did: read against the answer as written, or not rendered
/// and why not — never drawn as the same answer (A7, B-444).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formed {
    /// The form.
    pub form: Form,
    /// The reading, or why none was taken.
    pub outcome: Rendering,
}

/// Whether a form was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rendering {
    /// The parts in this form were asked, and the answer read against the
    /// answer as written.
    Read(Reading),
    /// No generation was spent: the parts cannot be put in this form, or
    /// the prompt is written in it already.
    NotRendered(&'static str),
}

/// Why a form was not rendered: the prompt is in it as written, and asking
/// it again would draw the baseline a second time.
pub const AS_WRITTEN: &str = "the prompt is written this way";

/// Why a listing form was not rendered: there is one part.
pub const ONE_PART: &str = "one part is nothing to list";

/// What every variant is read against: the answer as written and its
/// opening, the seed held still, and the two ways of asking. One place
/// that asks and compares, so every figure in a report is the same
/// operation on a different prompt (B-072, §3.4).
struct Bench<'a, 'b> {
    baseline: &'a str,
    opening: &'a [usize],
    seed: u64,
    ask: Ask<'b>,
    force: Force<'b>,
}

impl Bench<'_, '_> {
    /// Asks the prompt, greedy at the held seed, and reads the answer.
    ///
    /// The opening is an empty list where the baseline said nothing, and a
    /// rank over nothing is not taken rather than read as kept.
    fn read(&mut self, prompt: &str) -> Reading {
        let said = (self.ask)(prompt, Draw::greedy(self.seed));
        let held = if self.opening.is_empty() {
            None
        } else {
            (self.force)(prompt, self.opening)
        };
        Reading {
            moved: moved_by(self.baseline.trim(), said.text.trim()),
            held,
            answer: said.text,
            thought: said.thought,
        }
    }
}

/// What a prompt did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// How much the answer moves for a sentence that carries no instruction.
    ///
    /// The floor every clause is read against: a clause that moved this much
    /// or less moved the answer no more than an inert sentence would have, and
    /// what it did beyond perturbing the context is not visible here.
    pub floor: u64,
    /// The same forced reading for the inert sentence: how the baseline's
    /// opening ranks with a sentence carrying no instruction put in. The
    /// floor of `held`, taken by the same operation.
    pub floor_held: Option<Held>,
    /// What the model spent thinking under the control, by the same
    /// operation: the floor of the thought's cost, so that a part which
    /// raises it is told from one that merely perturbs it (B-455).
    pub floor_thought: Option<usize>,
    /// The floor at every position, in position order, where the caller
    /// asked for it; `None` where one draw was taken (B-434).
    pub floors: Option<Vec<FloorAt>>,
    /// Each part asked as the whole prompt in turn, the first `most` of
    /// them, where asked (B-435): how far the answer to it alone sat from
    /// the answer as written. `None` where not asked, which is not *every
    /// part alone gives the same answer* (A7).
    pub alone: Option<Vec<Reading>>,
    /// The inert sentence asked alone, where `alone` was: what the model
    /// says with nothing from the writer, read against the answer as
    /// written. A part whose answer alone sits as far off as this carries
    /// nothing of the whole on its own.
    pub alone_floor: Option<Reading>,
    /// The prompt grown a part at a time from the front, where asked
    /// (B-436): the first entry is the first part alone, the next the first
    /// two, and so on to one short of the whole — how far each sat from the
    /// answer as written, so the run of them says where the answer became
    /// the answer. Capped at `most` prefixes. `None` where not asked (A7).
    pub prefixes: Option<Vec<Reading>>,
    /// Each part swapped with the one after it, where asked (B-437): the
    /// first entry is the first two parts in the other order, the next the
    /// second and third, and so on — how far each sat from the answer as
    /// written. A swap that moves the answer says the parts' order carries
    /// it, not their words alone. Capped at `most` swaps. `None` where not
    /// asked (A7).
    pub swaps: Option<Vec<Reading>>,
    /// The same parts in each form, where asked (B-444): the words kept
    /// and the dress changed, so what moves is the form. `None` where not
    /// asked (A7).
    pub forms: Option<Vec<Formed>>,
    /// The answer to the prompt as written, which every ablation is compared
    /// against.
    pub baseline: String,
    /// What the model spent thinking before that answer, where the turn had
    /// a marker it thinks inside and the account counted it: the cost of the
    /// prompt as written, which a persona can raise as surely as it can
    /// change what is said (B-455).
    pub baseline_thought: Option<usize>,
    /// Each sentence, and what happened without it.
    pub clauses: Vec<Clause>,
    /// How many parts the document had beyond the ones ablated.
    pub clauses_over_the_cap: usize,
    /// What the document was taken apart into.
    pub unit: Unit,
    /// Whether the caller chose the unit, or the text did.
    pub unit_chosen: bool,
    /// The cap the run was under.
    pub most: usize,
    /// What several seeds made of it at a stated temperature, or `None`
    /// where no temperature was stated and the question was not asked.
    pub settled: Option<Settled>,
}

impl Report {
    /// The least, middle and most of the floor across positions, where the
    /// floor was taken at every one.
    #[must_use]
    pub fn floor_spread(&self) -> Option<Spread> {
        let mut drawn: Vec<u64> = self.floors.as_ref()?.iter().map(|at| at.moved).collect();
        drawn.sort_unstable();
        #[allow(
            clippy::integer_division,
            reason = "the upper median's index: a position in a list, not a figure"
        )]
        let middle = *drawn.get(drawn.len() / 2)?;
        Some(Spread {
            least: *drawn.first()?,
            middle,
            most: *drawn.last()?,
        })
    }

    /// The sentences whose removal changed nothing at all.
    #[must_use]
    pub fn unused(&self) -> Vec<&Clause> {
        self.clauses.iter().filter(|held| !held.changed).collect()
    }

    /// The sentences whose removal moved less of the answer than `most`.
    ///
    /// What counts as *barely* is the reader's, which is why it is an argument:
    /// MCF has not measured what size of change a person notices, and a
    /// threshold baked in here would be a judgement wearing a number's clothes
    /// (§3.15, A7).
    #[must_use]
    pub fn barely_moved(&self, most: u64) -> Vec<&Clause> {
        self.clauses
            .iter()
            .filter(|held| held.moved <= most)
            .collect()
    }

    /// The sentences that moved the answer no more than an inert one would.
    ///
    /// Measured against this run's own floor rather than a number chosen here:
    /// how much a removal perturbs an answer is a property of the model and
    /// the prompt together, and a threshold that travelled between them would
    /// be a constant standing in for a measurement (A7, §3.4).
    #[must_use]
    pub fn at_the_floor(&self) -> Vec<&Clause> {
        self.clauses
            .iter()
            .filter(|held| held.moved <= self.floor)
            .collect()
    }
}

/// Splits a prompt into the sentences a person wrote.
///
/// **The person's units, not the tokenizer's.** A report about pieces would be
/// a report about the vocabulary — which `mcf segment` already gives — and the
/// question here is about the writing. Sentence ends are `.`, `?`, `!` and a
/// line break; a run of them is one end.
#[must_use]
pub fn clauses_of(prompt: &str) -> Vec<String> {
    parts_of(prompt, Unit::Sentence)
        .into_iter()
        .map(|part| part.text)
        .collect()
}

/// Whether a character ends a sentence here: `.`, `?` or `!` before
/// whitespace or the end, or a line break.
///
/// **A full stop ends a sentence only where a space follows it.** Splitting
/// on every `.` cut `0.001` into two clauses, `arr.Length` into two, and
/// `System.Numerics` into two — so a prompt about precision, or one naming
/// any dotted identifier, was ablated on fragments that were never sentences.
/// A newline is a break whatever follows it (F147).
///
/// What this does not fix is `e.g.`, which ends in a full stop and a space
/// and is not the end of a sentence. Telling that from a sentence ending in
/// the letter g needs a list of abbreviations, which is a fact about a
/// language rather than about this prompt, and getting it wrong in the other
/// direction would silently join two real sentences. The failure that remains
/// splits one clause into two; the one removed split a number in half.
fn ends_a_sentence(character: char, next: Option<char>) -> bool {
    character == '\n'
        || (matches!(character, '.' | '?' | '!') && next.is_none_or(char::is_whitespace))
}

/// Whether a character ends a phrase here: a sentence end, or `,`, `;`, `:`
/// before whitespace or the end.
///
/// The same rule as the full stop's, for the same reason: `1,000` and
/// `a::b` are not two phrases.
fn ends_a_phrase(character: char, next: Option<char>) -> bool {
    ends_a_sentence(character, next)
        || (matches!(character, ',' | ';' | ':') && next.is_none_or(char::is_whitespace))
}

/// Splits a document into its parts, each with the whitespace that followed.
///
/// A paragraph ends at a blank line — a run of whitespace with two line
/// breaks in it. A piece with nothing alphanumeric in it (a lone `...`, a
/// rule of dashes) is not a part: it is folded into the separator before it,
/// so that removing the part before it removes it too.
#[must_use]
pub fn parts_of(text: &str, by: Unit) -> Vec<Part> {
    let mut found: Vec<Part> = Vec::new();
    let mut held = String::new();
    let mut characters = text.chars().peekable();
    // What ended the last part and lay after it, kept until the next part
    // begins so that the last part's trailing whitespace is not its `after`.
    let mut between = String::new();
    while let Some(character) = characters.next() {
        let ends = match by {
            Unit::Word => {
                held.push(character);
                characters.peek().is_none_or(|next| next.is_whitespace())
            }
            Unit::Phrase => {
                held.push(character);
                ends_a_phrase(character, characters.peek().copied())
            }
            Unit::Sentence => {
                held.push(character);
                ends_a_sentence(character, characters.peek().copied())
            }
            Unit::Paragraph => {
                if character == '\n' {
                    // A blank line: this break, then only whitespace up to
                    // another one.
                    let mut ahead = characters.clone();
                    let blank = std::iter::from_fn(|| ahead.next())
                        .take_while(|next| next.is_whitespace())
                        .any(|next| next == '\n');
                    if !blank {
                        held.push(character);
                    }
                    blank
                } else {
                    held.push(character);
                    false
                }
            }
        };
        if !ends {
            continue;
        }
        let mut after = String::new();
        if by == Unit::Paragraph {
            after.push(character);
        }
        while let Some(next) = characters
            .peek()
            .copied()
            .filter(|next| next.is_whitespace())
        {
            after.push(next);
            characters.next();
        }
        // The separator kept from before this piece, and the piece's own
        // trailing whitespace, which a sentence keeps inside `held`.
        let trimmed = held.trim().to_owned();
        let trailing: String = held
            .chars()
            .rev()
            .take_while(|held| held.is_whitespace())
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();
        held.clear();
        if trimmed.chars().any(char::is_alphanumeric) {
            if let Some(last) = found.last_mut() {
                last.after.push_str(&between);
            }
            between.clear();
            found.push(Part {
                text: trimmed,
                after: String::new(),
            });
            between.push_str(&trailing);
            between.push_str(&after);
        } else {
            between.push_str(&trimmed);
            between.push_str(&trailing);
            between.push_str(&after);
        }
    }
    let trimmed = held.trim().to_owned();
    if trimmed.chars().any(char::is_alphanumeric) {
        if let Some(last) = found.last_mut() {
            last.after.push_str(&between);
        }
        found.push(Part {
            text: trimmed,
            after: String::new(),
        });
    }
    if by == Unit::Phrase {
        with_short_phrases_joined(found)
    } else {
        found
    }
}

/// The fewest words a comma, semicolon or colon may leave on either side
/// of its cut. *The party steps into a cold, dripping cave.* has a comma
/// in it and is one phrase: cut there, removing *dripping cave.* leaves a
/// prompt nobody wrote, and the reading is of that prompt, not of theirs.
const FEWEST_WORDS_IN_A_PHRASE: usize = 3;

/// Phrases cut by `,`, `;` or `:` with fewer than [`FEWEST_WORDS_IN_A_PHRASE`]
/// words on either side, joined back to their neighbour. A sentence end
/// always cuts.
fn with_short_phrases_joined(found: Vec<Part>) -> Vec<Part> {
    let words_in = |text: &str| text.split_whitespace().count();
    let mut joined: Vec<Part> = Vec::new();
    for part in found {
        if let Some(last) = joined.last_mut()
            && last.text.ends_with([',', ';', ':'])
            && (words_in(&last.text) < FEWEST_WORDS_IN_A_PHRASE
                || words_in(&part.text) < FEWEST_WORDS_IN_A_PHRASE)
        {
            last.text.push_str(&last.after);
            last.text.push_str(&part.text);
            last.after = part.after;
            continue;
        }
        joined.push(part);
    }
    joined
}

/// The parts put back together as they were written.
#[must_use]
pub fn joined(parts: &[Part]) -> String {
    let mut text = String::new();
    for (at, part) in parts.iter().enumerate() {
        text.push_str(&part.text);
        // What followed the last part followed nothing that is kept: a
        // separator, or a rule of dashes folded into one, goes with the part
        // it led to.
        if at.saturating_add(1) < parts.len() {
            text.push_str(&part.after);
        }
    }
    text
}

/// How expected one part of the prompt was to the model, read from the rank
/// of each of its tokens: a second ordering of the parts that spends no
/// generation (B-433).
///
/// Read against the ablation rather than instead of it. A part that
/// surprised the model and changed nothing when removed is noise to the
/// model; one it fully expected and yet needed is doing structural work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Surprise {
    /// How many of the part's tokens the reading placed in it.
    pub tokens: usize,
    /// How many of them were the model's own first choice.
    pub first_choice: usize,
    /// How many ranked past the depth read, where the rank is a bound.
    pub past_depth: usize,
    /// How many were not ranked at all: the first piece of a turn with
    /// nothing before it has no position to be ranked at.
    pub no_context: usize,
}

/// Where the model put one piece of the prompt.
///
/// Three states a reader must not confuse: a rank, a rank past the depth
/// read — a bound, not an absence — and no reading at all, which is the
/// first piece of a turn nothing precedes. The last was a null rank like
/// the second, and a prompt that went with no turn markers had its first
/// piece counted as past the depth and every piece after it placed
/// nowhere (A7, F160).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    /// Where the model put it, counting from one.
    At(usize),
    /// Past the depth read.
    PastDepth,
    /// Not read: nothing preceded it to rank it against.
    NoContext,
}

/// The rank reading grouped by part: one [`Surprise`] a part, in order, and
/// how many ranked tokens fell nowhere.
///
/// The tokens are walked against the joined prompt with a cursor: a token
/// whose text is the prompt's next text belongs to the part the cursor is
/// in, and one that is not — a template piece, a piece the reading cut off
/// mid-way — is placed nowhere and counted as such, never guessed into a
/// part (A19, A7). Whitespace is skipped on both sides, since a separator
/// belongs to no part.
#[must_use]
pub fn surprise_by_part(parts: &[Part], ranked: &[(String, Rank)]) -> (Vec<Surprise>, usize) {
    let prompt = joined(parts);
    let mut ends = Vec::with_capacity(parts.len());
    let mut at = 0_usize;
    for (index, part) in parts.iter().enumerate() {
        at = at.saturating_add(part.text.len());
        ends.push(at);
        if index.saturating_add(1) < parts.len() {
            at = at.saturating_add(part.after.len());
        }
    }
    let mut found = vec![Surprise::default(); parts.len()];
    let mut nowhere = 0_usize;
    let mut cursor = 0_usize;
    for (text, rank) in ranked {
        let piece = text.trim();
        if piece.is_empty() {
            continue;
        }
        let rest = prompt.get(cursor..).unwrap_or_default();
        let skipped = rest.len().saturating_sub(rest.trim_start().len());
        cursor = cursor.saturating_add(skipped);
        let rest = prompt.get(cursor..).unwrap_or_default();
        if !rest.starts_with(piece) {
            nowhere = nowhere.saturating_add(1);
            continue;
        }
        let index = ends.iter().position(|end| cursor < *end);
        if let Some(surprise) = index.and_then(|index| found.get_mut(index)) {
            surprise.tokens = surprise.tokens.saturating_add(1);
            match rank {
                Rank::At(1) => surprise.first_choice = surprise.first_choice.saturating_add(1),
                Rank::At(_) => {}
                Rank::PastDepth => surprise.past_depth = surprise.past_depth.saturating_add(1),
                Rank::NoContext => surprise.no_context = surprise.no_context.saturating_add(1),
            }
        } else {
            nowhere = nowhere.saturating_add(1);
        }
        cursor = cursor.saturating_add(piece.len());
    }
    (found, nowhere)
}

/// One word of the prompt as the model received it: how many pieces the
/// tokenizer split it into, and where the model ranked the first of them.
///
/// Two readings of *how the model receives a word* that spend no
/// generation. A word in many pieces is one the vocabulary was not built
/// around — it was rare where the vocabulary was learned; a first piece the
/// model ranked far down is one it did not expect there. Neither is
/// comprehension, which nothing here observes; they are what is observable,
/// and are named as what they are (A7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    /// The word, as written.
    pub text: String,
    /// How many of the reading's tokens fell in it.
    pub pieces: usize,
    /// Where the model ranked its first ranked piece, counting from one;
    /// `None` where that was past the depth read — a bound, not an absence.
    pub rank: Option<usize>,
    /// How many of its pieces were the model's own first choice.
    pub first_choice: usize,
    /// How many of its pieces were not ranked at all, nothing preceding
    /// them: a word that is all of these was not read, and is not a first
    /// choice.
    pub unread: usize,
    /// Which part of the document — in the unit the report is by, counting
    /// from one — the word begins in. `None` for a word placed in no part.
    pub part: Option<usize>,
}

/// The rank reading by word: one [`Expected`] a word, in the order written,
/// and how many ranked tokens fell in no word.
///
/// The same cursor walk as [`surprise_by_part`], over the words rather than
/// the parts; a word's part is the part it begins in.
#[must_use]
pub fn expected_by_word(parts: &[Part], ranked: &[(String, Rank)]) -> (Vec<Expected>, usize) {
    let prompt = joined(parts);
    let words = parts_of(&prompt, Unit::Word);
    let ends_of = |held: &[Part]| {
        let mut ends = Vec::with_capacity(held.len());
        let mut at = 0_usize;
        for (index, part) in held.iter().enumerate() {
            at = at.saturating_add(part.text.len());
            ends.push(at);
            if index.saturating_add(1) < held.len() {
                at = at.saturating_add(part.after.len());
            }
        }
        ends
    };
    let word_ends = ends_of(&words);
    let part_ends = ends_of(parts);
    let mut found: Vec<Expected> = words
        .iter()
        .zip(&word_ends)
        .map(|(word, end)| Expected {
            text: word.text.clone(),
            pieces: 0,
            rank: None,
            first_choice: 0,
            unread: 0,
            part: part_ends
                .iter()
                .position(|part_end| end.saturating_sub(word.text.len()) < *part_end)
                .map(|index| index.saturating_add(1)),
        })
        .collect();
    let mut nowhere = 0_usize;
    let mut cursor = 0_usize;
    for (text, rank) in ranked {
        let piece = text.trim();
        if piece.is_empty() {
            continue;
        }
        let rest = prompt.get(cursor..).unwrap_or_default();
        let skipped = rest.len().saturating_sub(rest.trim_start().len());
        cursor = cursor.saturating_add(skipped);
        let rest = prompt.get(cursor..).unwrap_or_default();
        if !rest.starts_with(piece) {
            nowhere = nowhere.saturating_add(1);
            continue;
        }
        let index = word_ends.iter().position(|end| cursor < *end);
        if let Some(word) = index.and_then(|index| found.get_mut(index)) {
            if word.pieces == word.unread
                && let Rank::At(at) = rank
            {
                word.rank = Some(*at);
            }
            word.pieces = word.pieces.saturating_add(1);
            match rank {
                Rank::At(1) => word.first_choice = word.first_choice.saturating_add(1),
                Rank::At(_) | Rank::PastDepth => {}
                Rank::NoContext => word.unread = word.unread.saturating_add(1),
            }
        } else {
            nowhere = nowhere.saturating_add(1);
        }
        cursor = cursor.saturating_add(piece.len());
    }
    (found, nowhere)
}

/// The document with an inert sentence put into it.
///
/// Second from the end rather than appended, so that removing it disturbs what
/// follows — which is what removing a clause does, and the whole point of the
/// control is that the two operations match. It takes the separator of the
/// part it follows, so that a paragraph gets a paragraph's break and a
/// sentence a sentence's.
#[must_use]
pub fn with_inert(parts: &[Part]) -> String {
    with_inert_at(parts, parts.len().saturating_sub(1))
}

/// The document with an inert sentence put in before the part at `at`, or
/// at the end where `at` is the part count (B-434).
///
/// The separator is the one of the part before the insertion, or the first
/// the document has where there is none before it, so that a paragraph gets
/// a paragraph's break wherever the sentence lands; at the end, the part
/// that was last takes that separator and the inert sentence takes none.
#[must_use]
pub fn with_inert_at(parts: &[Part], at: usize) -> String {
    let mut all = parts.to_vec();
    let at = at.min(all.len());
    let separator = at
        .checked_sub(1)
        .and_then(|before| all.get(before))
        .map(|part| part.after.clone())
        .filter(|after| !after.is_empty())
        .or_else(|| {
            all.iter()
                .map(|part| part.after.clone())
                .find(|after| !after.is_empty())
        })
        .unwrap_or_else(|| " ".to_owned());
    if at == all.len() {
        if let Some(last) = all.last_mut() {
            last.after.clone_from(&separator);
        }
        all.push(Part {
            text: NO_INSTRUCTION.to_owned(),
            after: String::new(),
        });
    } else {
        all.insert(
            at,
            Part {
                text: NO_INSTRUCTION.to_owned(),
                after: separator,
            },
        );
    }
    joined(&all)
}

/// The document with one part left out, as written.
#[must_use]
pub fn without(parts: &[Part], at: usize) -> String {
    let kept: Vec<Part> = parts
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != at)
        .map(|(_, held)| held.clone())
        .collect();
    joined(&kept)
}

/// How much two answers differ, in parts per million of the longer.
///
/// The Levenshtein distance over words rather than characters: what is being
/// compared is what the model said, and a difference of one letter inside a
/// word is not a different word. Nought means the two are the same run of
/// words; a million means nothing survived.
///
/// The punctuation at a word's edges is set aside before the words are
/// compared (B-464): `Nile` and `Nile.` are one word, and on a one-word
/// answer the alternative was a full stop read as the whole answer moving.
/// A word that is nothing but punctuation is not a word. What is inside a
/// word stays — `L'Indus`, `don't`, `x.y` — and so does anything that is
/// not sentence punctuation, so `a + b` against `a - b` is a word of three.
#[must_use]
pub fn moved_by(one: &str, other: &str) -> u64 {
    let a = words_of(one);
    let b = words_of(other);
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 0;
    }
    // One row at a time: the whole table is not needed and an answer can be
    // long.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.iter().enumerate() {
        let mut previous = row.first().copied().unwrap_or(0);
        if let Some(first) = row.first_mut() {
            *first = i.saturating_add(1);
        }
        for (j, right) in b.iter().enumerate() {
            let held = row.get(j.saturating_add(1)).copied().unwrap_or(0);
            let cost = usize::from(left != right);
            let candidate = previous
                .saturating_add(cost)
                .min(held.saturating_add(1))
                .min(row.get(j).copied().unwrap_or(0).saturating_add(1));
            if let Some(cell) = row.get_mut(j.saturating_add(1)) {
                *cell = candidate;
            }
            previous = held;
        }
    }
    let distance = row.last().copied().unwrap_or(0);
    u64::try_from(distance)
        .unwrap_or(0)
        .saturating_mul(1_000_000)
        .wrapping_div(u64::try_from(longest).unwrap_or(1).max(1))
}

/// The words of an answer with the sentence punctuation at their edges set
/// aside, and no word that was only punctuation.
fn words_of(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(is_sentence_punctuation))
        .filter(|word| !word.is_empty())
        .collect()
}

/// The marks a sentence is punctuated with, and the quotes and brackets that
/// close around a word: what a model puts at the edge of a word without
/// changing which word it is.
fn is_sentence_punctuation(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '«'
            | '»'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '…'
            | '*'
            | '`'
    )
}

/// What the model said to one question: the text, and the identifiers it
/// said it in.
///
/// Both, because the forced reading puts the *identifiers* back to the model
/// — re-encoding the text could segment it differently from the way the model
/// produced it, and a rank read at a token the model never wrote would be a
/// rank of nothing (A21).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answered {
    /// The text.
    pub text: String,
    /// The same thing as identifiers.
    pub tokens: Vec<usize>,
    /// How many tokens the model spent before its answer began, where the
    /// turn it was asked under has a marker it thinks inside and the
    /// account counted them (B-455, B-451). `None` is *not counted*, which
    /// is not nought (A7).
    pub thought: Option<usize>,
}

/// How a caller asks the model one question.
///
/// The prompt and the draw in, what it said out. This module starts nothing
/// and reaches no socket: what generates is the daemon's, and holding that at
/// the boundary is what lets the whole measurement be tested without a model.
pub type Ask<'a> = &'a mut dyn FnMut(&str, Draw) -> Answered;

/// How a caller asks where the model ranks an opening after a prompt.
///
/// The prompt and the opening's identifiers in; where each ranked out, or
/// `None` where the reading could not be taken at all. Nothing is generated.
pub type Force<'a> = &'a mut dyn FnMut(&str, &[usize]) -> Option<Held>;

/// Measures what a prompt does.
///
/// **The seed is held still across every ablation, and every ablation is
/// greedy.** What must differ between the baseline and a clause left out is
/// the prompt and nothing else; a seed that moved would make every comparison
/// a comparison of two draws (D19). `settle` is the temperature the
/// settledness seeds are drawn at, and `None` asks them nothing.
#[must_use]
pub fn measure(
    taken: &Taken<'_>,
    seed: u64,
    settle: Option<Settle>,
    ask: Ask<'_>,
    force: Force<'_>,
) -> Report {
    let (unit, unit_chosen) = taken.unit();
    let all = parts_of(taken.text, unit);
    // The baseline is the parts put back together, not the text as pasted:
    // what differs between it and a variant must be the part removed and
    // nothing else, and a variant is always the joined parts (§3.4).
    let prompt = joined(&all);
    let Answered {
        text: baseline,
        tokens: opening,
        thought: baseline_thought,
    } = ask(&prompt, Draw::greedy(seed));
    let opening: Vec<usize> = opening.into_iter().take(MOST_FORCED).collect();
    let mut bench = Bench {
        baseline: &baseline,
        opening: &opening,
        seed,
        ask,
        force,
    };

    let most = taken.cap();
    let ablated = all.len().min(most);
    let mut clauses = Vec::with_capacity(ablated);
    // One clause is nothing to ablate: removing it leaves no prompt, and an
    // empty prompt's answer says nothing about the sentence.
    if all.len() > 1 {
        for at in 0..ablated {
            let shortened = without(&all, at);
            let read = bench.read(&shortened);
            clauses.push(Clause {
                changed: read.answer.trim() != baseline.trim(),
                moved: read.moved,
                text: all
                    .get(at)
                    .map(|part| part.text.clone())
                    .unwrap_or_default(),
                without: read.answer,
                held: read.held,
                thought: read.thought,
            });
        }
    }

    // The floor, measured by the SAME operation the clauses are.
    //
    // An earlier cut of this appended an inert sentence and measured what
    // adding it did — but every clause above is *removed*, and removing text
    // from the middle of a prompt disturbs what follows it far more than
    // appending to the end does. The two were not comparable, and the floor
    // came out at 12.9% while an irrelevant sentence in a real prompt scored
    // 50.6%: a floor that separated nothing.
    //
    // So the inert sentence is put in and then taken out. The prompt with it
    // is asked, and the answer compared against the baseline — which is that
    // same prompt with the inert sentence removed. One removal against
    // another, which is the comparison the numbers above need.
    let read = bench.read(&with_inert(&all));
    let floor = read.moved;
    let floor_thought = read.thought;
    let floor_held = if all.len() <= 1 { None } else { read.held };
    let floors = (taken.extras.has(Extra::Floors) && all.len() > 1)
        .then(|| floors_of(&mut bench, &all, (floor, floor_held)));
    let (alone, alone_floor) = if taken.extras.has(Extra::Alone) && all.len() > 1 {
        let (alone, floor) = alone_of(&mut bench, &all, ablated);
        (Some(alone), Some(floor))
    } else {
        (None, None)
    };
    let prefixes = (taken.extras.has(Extra::Prefixes) && all.len() > 1)
        .then(|| prefixes_of(&mut bench, &all, ablated));
    let swaps = (taken.extras.has(Extra::Swaps) && all.len() > 1)
        .then(|| swaps_of(&mut bench, &all, ablated));
    let forms = taken
        .extras
        .has(Extra::Forms)
        .then(|| forms_of(&mut bench, &all, &prompt));

    let settled = settle.map(|under| settled(&prompt, seed, under, &baseline, bench.ask));

    Report {
        floor,
        floor_held,
        floor_thought,
        floors,
        alone,
        alone_floor,
        prefixes,
        swaps,
        forms,
        baseline,
        baseline_thought,
        clauses,
        clauses_over_the_cap: all.len().saturating_sub(ablated),
        unit,
        unit_chosen,
        most,
        settled,
    }
}

/// The floor at every position (B-434).
///
/// One draw of the floor is one number, and a part a few points under it
/// may be under the floor or under that draw. The position already drawn
/// is not drawn again: its figure is the one passed in.
fn floors_of(bench: &mut Bench<'_, '_>, all: &[Part], drawn: (u64, Option<Held>)) -> Vec<FloorAt> {
    let drawn_at = all.len().saturating_sub(1);
    (0..=all.len())
        .map(|position| {
            if position == drawn_at {
                return FloorAt {
                    position,
                    moved: drawn.0,
                    held: drawn.1,
                };
            }
            let read = bench.read(&with_inert_at(all, position));
            FloorAt {
                position,
                moved: read.moved,
                held: read.held,
            }
        })
        .collect()
}

/// Each part alone, and the inert sentence alone (B-435).
///
/// **Sufficiency, beside necessity.** Removing a part says what the answer
/// loses without it; asking the part on its own says how much of the answer
/// it carries by itself. A part can be both, either or neither, and the two
/// readings disagree in ways that are the point of having both. The control
/// is the inert sentence alone — the answer to nothing from the writer —
/// which is as far from the answer as written as a part alone can be
/// expected to sit while carrying nothing of it.
fn alone_of(bench: &mut Bench<'_, '_>, all: &[Part], first: usize) -> (Vec<Reading>, Reading) {
    let alone = all
        .iter()
        .take(first)
        .map(|part| bench.read(part.text.trim()))
        .collect();
    (alone, bench.read(NO_INSTRUCTION))
}

/// The prompt grown from the front (B-436).
///
/// Removing one part at a time says what each is needed for; growing the
/// prompt says when the answer arrived. A persona whose answer is in place
/// after two of five paragraphs has three the model reads as elaboration —
/// which is not the same as three it ignores, and the removal column says
/// which. The whole prompt is not read again: its answer is the baseline,
/// at no distance from itself.
fn prefixes_of(bench: &mut Bench<'_, '_>, all: &[Part], most: usize) -> Vec<Reading> {
    (1..=strict_prefixes(all.len(), most))
        .map(|kept| bench.read(&joined(all.get(..kept).unwrap_or_default())))
        .collect()
}

/// Neighbouring parts swapped (B-437).
///
/// Removing a part and growing the prompt both keep the parts in the
/// order written. Swapping two neighbours keeps every word and changes
/// only where two of them sit, so what moves is position alone: a swap
/// within the floor says the answer is carried by what the parts say; a
/// swap that moves it says the model reads their order, which is the
/// recency or lost-in-the-middle effect with no judgement in the reading.
/// Each part's separator stays with its place, so a paragraph break is
/// still a paragraph break after the swap.
fn swaps_of(bench: &mut Bench<'_, '_>, all: &[Part], most: usize) -> Vec<Reading> {
    (0..strict_prefixes(all.len(), most))
        .map(|at| bench.read(&swapped(all, at)))
        .collect()
}

/// The document with the part at `at` and the one after it in each
/// other's places, the separators left where they were.
#[must_use]
pub fn swapped(parts: &[Part], at: usize) -> String {
    let mut all = parts.to_vec();
    let after = at.saturating_add(1);
    if after < all.len() {
        let separators: Vec<String> = all.iter().map(|part| part.after.clone()).collect();
        all.swap(at, after);
        for (part, separator) in all.iter_mut().zip(separators) {
            part.after = separator;
        }
    }
    joined(&all)
}

/// The same parts in each form (B-444).
///
/// Removing, growing and swapping all keep the prompt's dress and change
/// its words or their places. This keeps every word in its place and
/// changes the dress alone, so what moves is the form: a list that moves
/// the answer past the floor is a model that reads lists as lists. A form
/// the prompt is written in already is not asked — the answer would be the
/// baseline, drawn twice — and one part is not a list; each is said as not
/// rendered rather than drawn as no movement (A7).
fn forms_of(bench: &mut Bench<'_, '_>, all: &[Part], written: &str) -> Vec<Formed> {
    Form::ALL
        .into_iter()
        .map(|form| Formed {
            form,
            outcome: match form.render(all) {
                None => Rendering::NotRendered(ONE_PART),
                Some(rendered) if rendered == written => Rendering::NotRendered(AS_WRITTEN),
                Some(rendered) => Rendering::Read(bench.read(&rendered)),
            },
        })
        .collect()
}

/// The settledness question: the same prompt, `SEEDS` seeds, one temperature.
///
/// The greedy baseline is not one of the samples — it was drawn under another
/// condition, and counting it among them would make the count a count of two
/// things (§3.4). It is what `from_greedy` is measured from.
fn settled(prompt: &str, seed: u64, under: Settle, baseline: &str, ask: Ask<'_>) -> Settled {
    let answers: Vec<String> = (0..SEEDS)
        .map(|extra| {
            let draw = Draw {
                seed: seed.wrapping_add(u64::try_from(extra).unwrap_or(u64::MAX)),
                temperature: under.temperature,
                truncation: under.truncation,
            };
            ask(prompt, draw).text.trim().to_owned()
        })
        .collect();
    let mut distinct = answers.clone();
    distinct.sort();
    distinct.dedup();
    let mut spread = 0_u64;
    for (at, one) in answers.iter().enumerate() {
        for other in answers.iter().skip(at.saturating_add(1)) {
            spread = spread.max(moved_by(one, other));
        }
    }
    let from_greedy = answers
        .iter()
        .map(|answer| moved_by(baseline.trim(), answer))
        .max()
        .unwrap_or(0);
    Settled {
        temperature: under.temperature,
        truncation: under.truncation,
        asked: SEEDS,
        distinct: distinct.len(),
        spread,
        from_greedy,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod splitting_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use super::clauses_of;

    /// A number is not two sentences (F147).
    #[test]
    fn a_decimal_does_not_end_a_sentence() {
        let held = clauses_of("Results must be accurate to 0.001 tolerance. Use doubles.");
        assert_eq!(
            held,
            vec![
                "Results must be accurate to 0.001 tolerance.".to_owned(),
                "Use doubles.".to_owned()
            ],
            "a prompt about precision was ablated on half a number"
        );
    }

    /// Nor is a dotted identifier, which is most of what a coding prompt says.
    #[test]
    fn a_dotted_name_does_not_end_a_sentence() {
        assert_eq!(
            clauses_of("Call arr.Length to get the size. Return an int."),
            vec![
                "Call arr.Length to get the size.".to_owned(),
                "Return an int.".to_owned()
            ]
        );
        assert_eq!(clauses_of("Use System.Numerics.").len(), 1);
    }

    /// A newline is a break whatever follows it.
    #[test]
    fn a_line_ending_is_a_break() {
        assert_eq!(clauses_of("One line.\nAnother line.").len(), 2);
        assert_eq!(clauses_of("no punctuation\nand more").len(), 2);
    }

    /// And what is still wrong, held so that it is a known limit rather than a
    /// surprise: an abbreviation ends in a full stop and a space.
    #[test]
    fn an_abbreviation_is_still_read_as_an_ending() {
        // Two sentences, read as three. Recorded because a check that passes
        // on what is fixed and says nothing about what is not is a check that
        // reads as a guarantee (A7).
        assert_eq!(
            clauses_of("Use a library, e.g. System.Numerics. Keep it simple.").len(),
            3
        );
    }

    /// The whole point: the prompt is the sentences a person wrote.
    #[test]
    fn a_prompt_is_split_where_a_person_would_split_it() {
        let held = clauses_of(
            "write a class in c#. the class handles all the basic math functions. each \
             function should output extremely accurate results.",
        );
        assert_eq!(held.len(), 3, "{held:?}");
        assert!(held[0].ends_with("c#."), "{held:?}");
    }
}
