#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declined {
    pub modality: &'static str,
    pub looked_for: &'static str,
    pub because: &'static str,
    pub needs: &'static str,
    pub until: &'static str,
}

pub const DECLINED: [Declined; 1] = [Declined {
    modality: "multilingual fluency",
    looked_for: "nothing — the question is asked of a model's answers rather than of its \
                     file, and grading an answer needs a rater",
    because: "what a language *costs* this vocabulary is answerable without an engine and \
                  is probed (`language-cost`); how well the model speaks it is a graded task. \
                  Reporting the first as the second is exactly the reading F81 was written to \
                  prevent",
    needs: "a laboratory with a graded task in each language, and the workload to grade \
                against",
    until: "B-110",
}];
