#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub language: &'static str,
    pub text: &'static str,
}

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

pub const SOURCE: &str = "the first clause of Article 1 of the Universal Declaration of Human \
                          Rights, in the United Nations' own translations";
