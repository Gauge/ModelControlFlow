use core::fmt;

use crate::failure::Failure;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabId(String);

impl LabId {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LabId {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Score {
    lab: LabId,
    parts_per_million: u64,
}

impl Score {
    #[must_use]
    pub const fn new(lab: LabId, parts_per_million: u64) -> Self {
        Self {
            lab,
            parts_per_million,
        }
    }

    #[must_use]
    pub const fn lab(&self) -> &LabId {
        &self.lab
    }

    #[must_use]
    pub const fn parts_per_million(&self) -> u64 {
        self.parts_per_million
    }

    #[must_use]
    pub fn against(&self, other: &Self) -> Option<core::cmp::Ordering> {
        (self.lab == other.lab).then(|| self.parts_per_million.cmp(&other.parts_per_million))
    }
}

impl fmt::Display for Score {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}.{}% on {}",
            self.parts_per_million.wrapping_div(10_000),
            self.parts_per_million.wrapping_div(1_000).wrapping_rem(10),
            self.lab
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Graded {
    Measured(Score),
    NotApplicable { capability: String },
    Unknown { why: String },
    Failed(Box<Failure>),
}

impl Graded {
    #[must_use]
    pub const fn score(&self) -> Option<&Score> {
        match self {
            Self::Measured(score) => Some(score),
            Self::NotApplicable { .. } | Self::Unknown { .. } | Self::Failed(_) => None,
        }
    }

    #[must_use]
    pub const fn is_measured(&self) -> bool {
        matches!(self, Self::Measured(_))
    }
}

impl fmt::Display for Graded {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Measured(score) => write!(form, "{score}"),
            Self::NotApplicable { capability } => write!(
                form,
                "not applicable: {capability} was verified absent, so this was not measured \
                 badly — it was not measured (B40)"
            ),
            Self::Unknown { why } => write!(
                form,
                "unknown: {why} — which is not the same as absent, and not a low score"
            ),
            Self::Failed(failure) => write!(form, "failed: {failure}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    outcomes: Vec<(LabId, Graded)>,
}

impl Profile {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            outcomes: Vec::new(),
        }
    }

    #[must_use]
    pub fn and(mut self, lab: LabId, graded: Graded) -> Self {
        self.outcomes.push((lab, graded));
        self
    }

    #[must_use]
    pub fn all(&self) -> &[(LabId, Graded)] {
        &self.outcomes
    }

    #[must_use]
    pub fn measured(&self) -> Vec<&LabId> {
        self.outcomes
            .iter()
            .filter(|(_, graded)| graded.is_measured())
            .map(|(lab, _)| lab)
            .collect()
    }

    #[must_use]
    pub fn inapplicable(&self) -> Vec<&LabId> {
        self.outcomes
            .iter()
            .filter(|(_, graded)| matches!(graded, Graded::NotApplicable { .. }))
            .map(|(lab, _)| lab)
            .collect()
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.outcomes.is_empty() {
            return form.write_str("no laboratory has reported on this");
        }
        write!(
            form,
            "measured by {} of {} laboratory(ies)",
            self.measured().len(),
            self.outcomes.len()
        )?;
        let absent = self.inapplicable();
        if !absent.is_empty() {
            write!(
                form,
                "; inapplicable to {}",
                absent
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(", ")
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    name: String,
    profile: crate::origin::LocallyMeasured<Profile>,
}

impl Candidate {
    #[must_use]
    pub fn new(name: impl Into<String>, profile: crate::origin::LocallyMeasured<Profile>) -> Self {
        Self {
            name: name.into(),
            profile,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn profile(&self) -> &Profile {
        self.profile.value()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoRecommendation {
    AFieldOfNone,
    AFieldOfOne { only: String },
    TooFewMeasured { lab: LabId, measured: usize },
}

impl fmt::Display for NoRecommendation {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AFieldOfNone => form.write_str(
                "nothing was considered, so there is nothing to recommend and nothing to \
                 compare",
            ),
            Self::AFieldOfOne { only } => write!(
                form,
                "one candidate ({only}) is not a field: a frontier with a single point is not a \
                 frontier, and *the best of one* recommends whatever it was handed (§6.23)"
            ),
            Self::TooFewMeasured { lab, measured } => write!(
                form,
                "{lab} has a reading for {measured} of the candidates, which is not a \
                 comparison: the others were not measured badly, they were not measured (B40)"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Field {
    candidates: Vec<Candidate>,
}

impl Field {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            candidates: Vec::new(),
        }
    }

    #[must_use]
    pub fn and(mut self, candidate: Candidate) -> Self {
        self.candidates.push(candidate);
        self
    }

    #[must_use]
    pub fn all(&self) -> &[Candidate] {
        &self.candidates
    }

    pub fn ordered_by(&self, lab: &LabId) -> Result<Vec<&Candidate>, NoRecommendation> {
        match self.candidates.as_slice() {
            [] => return Err(NoRecommendation::AFieldOfNone),
            [only] => {
                return Err(NoRecommendation::AFieldOfOne {
                    only: only.name.clone(),
                });
            }
            _ => {}
        }
        let mut measured: Vec<(&Candidate, u64)> = self
            .candidates
            .iter()
            .filter_map(|candidate| {
                candidate
                    .profile()
                    .all()
                    .iter()
                    .find(|(named, _)| named == lab)
                    .and_then(|(_, graded)| graded.score())
                    .filter(|score| score.lab() == lab)
                    .map(|score| (candidate, score.parts_per_million()))
            })
            .collect();
        if measured.len() < 2 {
            return Err(NoRecommendation::TooFewMeasured {
                lab: lab.clone(),
                measured: measured.len(),
            });
        }
        measured.sort_by_key(|(_, parts)| core::cmp::Reverse(*parts));
        Ok(measured
            .into_iter()
            .map(|(candidate, _)| candidate)
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub on: LabId,
    pub measured: usize,
    pub considered: usize,
    pub inapplicable: Vec<LabId>,
    pub silent: Vec<LabId>,
}

impl fmt::Display for Coverage {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "on {} alone, which measured {} of {} candidate(s)",
            self.on, self.measured, self.considered
        )?;
        let named = |held: &[LabId]| {
            held.iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(", ")
        };
        if !self.inapplicable.is_empty() {
            write!(
                form,
                "; inapplicable to {} (verified absent, which is not a low score)",
                named(&self.inapplicable)
            )?;
        }
        if !self.silent.is_empty() {
            write!(
                form,
                "; {} informed nothing here, which is *not run* rather than *no difference* \
                 (A7)",
                named(&self.silent)
            )?;
        }
        Ok(())
    }
}

impl Field {
    #[must_use]
    pub fn coverage(&self, lab: &LabId) -> Coverage {
        let mut inapplicable = Vec::new();
        let mut silent = Vec::new();
        let mut measured = 0_usize;
        for candidate in &self.candidates {
            for (named, graded) in candidate.profile().all() {
                match graded {
                    Graded::Measured(_) if named == lab => measured = measured.saturating_add(1),
                    Graded::NotApplicable { .. } => inapplicable.push(named.clone()),
                    Graded::Unknown { .. } | Graded::Failed(_) => silent.push(named.clone()),
                    Graded::Measured(_) => {}
                }
            }
        }
        for held in [&mut inapplicable, &mut silent] {
            held.sort();
            held.dedup();
        }
        let reading: Vec<LabId> = self
            .candidates
            .iter()
            .flat_map(|candidate| candidate.profile().measured())
            .cloned()
            .collect();
        silent.retain(|named| !reading.contains(named));
        inapplicable.retain(|named| !reading.contains(named));
        Coverage {
            on: lab.clone(),
            measured,
            considered: self.candidates.len(),
            inapplicable,
            silent,
        }
    }
}
