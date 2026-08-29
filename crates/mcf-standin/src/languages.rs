//! One sentence, in several scripts, for asking a vocabulary what it costs
//! (B-379, §3.15, DEC-002).
//!
//! **What this is and is not.** It is a fixed set of strings that MCF declares
//! and carries. It is *not* a claim about the languages: a vocabulary that
//! spends fifty tokens here has spent fifty tokens on **this string**, and
//! that is the only thing measured. A different sentence would give different
//! numbers, which is why the sentence travels with the result rather than
//! sitting in a footnote (§3.4).
//!
//! **Nor is it a claim about the model's fluency.** A model can be excellent
//! at a language its vocabulary spells expensively, and hopeless at one it
//! spells cheaply. The cost is a property of the vocabulary — of what pieces
//! somebody put in a file — and the wording everywhere must keep those apart,
//! because *expensive* reads as *bad* to a reader who is not being careful.
//!
//! **Why it is worth showing at all** (DEC-002: *expose the meaningful values
//! so people learn to read them*). The cost is real and compounding: a
//! vocabulary that spends two and a half times as many tokens on the same
//! meaning spends two and a half times the context, and two and a half times
//! whatever a token costs in money or in time. It is invisible in every
//! surface a person ordinarily sees.
//!
//! **The first clause of Article 1 of the Universal Declaration of Human
//! Rights**, which is the most widely and most carefully translated sentence
//! there is — the United Nations publishes it in five hundred languages and
//! places no restriction on reproducing it. Using a sentence somebody else
//! translated with care is better than using one MCF translated itself.

/// A language, and the sentence in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// What the language is called, in English.
    pub language: &'static str,
    /// The sentence.
    pub text: &'static str,
}

/// The declared set.
///
/// Thirteen, chosen to span the scripts that behave differently under a
/// subword vocabulary — Latin, Cyrillic, Greek, Han, Kana, Hangul, Arabic and
/// Devanagari — rather than to rank the world's languages. A set is a choice
/// and this one is stated here in one place so that it is one line to find and
/// one line to change.
pub const DECLARED: [Sample; 13] = [
    Sample {
        language: "English",
        text: "All human beings are born free and equal in dignity and rights.",
    },
    Sample {
        language: "French",
        text: "Tous les êtres humains naissent libres et égaux en dignité et en droits.",
    },
    Sample {
        language: "Spanish",
        text: "Todos los seres humanos nacen libres e iguales en dignidad y derechos.",
    },
    Sample {
        language: "German",
        text: "Alle Menschen sind frei und gleich an Würde und Rechten geboren.",
    },
    Sample {
        language: "Italian",
        text: "Tutti gli esseri umani nascono liberi ed eguali in dignità e diritti.",
    },
    Sample {
        language: "Portuguese",
        text: "Todos os seres humanos nascem livres e iguais em dignidade e direitos.",
    },
    Sample {
        language: "Russian",
        text: "Все люди рождаются свободными и равными в своем достоинстве и правах.",
    },
    Sample {
        language: "Greek",
        text: "Όλοι οι άνθρωποι γεννιούνται ελεύθεροι και ίσοι στην αξιοπρέπεια και τα δικαιώματα.",
    },
    Sample {
        language: "Arabic",
        text: "يولد جميع الناس أحراراً متساوين في الكرامة والحقوق.",
    },
    Sample {
        language: "Hindi",
        text: "सभी मनुष्यों को गौरव और अधिकारों के मामले में जन्मजात स्वतन्त्रता और समानता प्राप्त है।",
    },
    Sample {
        language: "Chinese (Simplified)",
        text: "人人生而自由，在尊严和权利上一律平等。",
    },
    Sample {
        language: "Japanese",
        text: "すべての人間は、生まれながらにして自由であり、かつ、尊厳と権利とについて平等である。",
    },
    Sample {
        language: "Korean",
        text: "모든 인간은 태어날 때부터 자유로우며 그 존엄과 권리에 있어 동등하다.",
    },
];

/// Where the sentence comes from, in one line, so that a surface can say it.
pub const SOURCE: &str = "the first clause of Article 1 of the Universal Declaration of Human \
                          Rights, in the United Nations' own translations";
