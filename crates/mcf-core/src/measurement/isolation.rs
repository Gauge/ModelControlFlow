use core::fmt;

use super::Conditions;

pub const INSTRUMENT: &str = "mcf_build";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Isolation {
    SameConfiguration,
    Isolated {
        variable: &'static str,
    },
    Confounded {
        differ: Vec<&'static str>,
    },
    Undetermined {
        differ: Vec<&'static str>,
        unread: Vec<&'static str>,
    },
}

impl Isolation {
    #[must_use]
    pub fn between(one: &Conditions, other: &Conditions) -> Self {
        let mut differ = Vec::new();
        let mut unread = Vec::new();

        if one.mcf() != other.mcf() {
            differ.push(INSTRUMENT);
        }
        for ((question, mine), (_, theirs)) in one
            .floor()
            .entries()
            .into_iter()
            .zip(other.floor().entries())
        {
            match (mine.known(), theirs.known()) {
                (Some(mine), Some(theirs)) if mine != theirs => differ.push(question),
                (Some(_), Some(_)) => {}
                _ => unread.push(question),
            }
        }

        if differ.len() > 1 {
            return Self::Confounded { differ };
        }
        if !unread.is_empty() {
            return Self::Undetermined { differ, unread };
        }
        match differ.first() {
            Some(variable) => Self::Isolated { variable },
            None => Self::SameConfiguration,
        }
    }

    #[must_use]
    pub const fn isolates_a_variable(&self) -> bool {
        matches!(*self, Self::Isolated { .. })
    }

    #[must_use]
    pub const fn is_confounded(&self) -> bool {
        matches!(*self, Self::Confounded { .. })
    }

    #[must_use]
    pub fn differing(&self) -> &[&'static str] {
        match self {
            Self::SameConfiguration => &[],
            Self::Isolated { variable } => core::slice::from_ref(variable),
            Self::Confounded { differ } | Self::Undetermined { differ, .. } => differ,
        }
    }
}

impl fmt::Display for Isolation {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameConfiguration => form.write_str(
                "the two arms are one configuration: nothing differs, so what this measures is \
                 the machine rather than a difference between them",
            ),
            Self::Isolated { variable } => {
                write!(form, "one variable differs, and it is {variable}")
            }
            Self::Confounded { differ } => write!(
                form,
                "these are not comparable: {} conditions differ ({}), so a delta between them \
                 says nothing about which one produced it (A8)",
                differ.len(),
                differ.join(", ")
            ),
            Self::Undetermined { differ, unread } => {
                if differ.is_empty() {
                    write!(
                        form,
                        "isolation is undetermined: nothing known differs, and {} condition(s) \
                         could not be compared ({})",
                        unread.len(),
                        unread.join(", ")
                    )
                } else {
                    write!(
                        form,
                        "isolation is undetermined: {} differs, and {} condition(s) could not be \
                         compared ({})",
                        differ.join(", "),
                        unread.len(),
                        unread.join(", ")
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
