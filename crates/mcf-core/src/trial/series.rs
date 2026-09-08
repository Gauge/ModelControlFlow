use core::fmt;

use crate::measurement::Quantity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Thinning(u32);

impl Thinning {
    pub const FULL: Self = Self(1);

    #[must_use]
    pub const fn every(nth: u32) -> Option<Self> {
        if nth == 0 { None } else { Some(Self(nth)) }
    }

    #[must_use]
    pub const fn factor(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn is_full_resolution(self) -> bool {
        self.0 == 1
    }
}

impl fmt::Display for Thinning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_full_resolution() {
            f.write_str("full resolution")
        } else {
            write!(f, "thinned, every {}th point", self.0)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series<Q: Quantity> {
    points: Vec<Q>,
    thinning: Thinning,
}

impl<Q: Quantity> Series<Q> {
    #[must_use]
    pub fn new(points: impl IntoIterator<Item = Q>, thinning: Thinning) -> Self {
        Self {
            points: points.into_iter().collect(),
            thinning,
        }
    }

    #[must_use]
    pub fn thinned(&self, nth: u32) -> Option<Self> {
        let further = Thinning::every(nth)?;
        let factor = self.thinning.factor().checked_mul(further.factor())?;
        Some(Self {
            points: self
                .points
                .iter()
                .copied()
                .step_by(further.factor() as usize)
                .collect(),
            thinning: Thinning(factor),
        })
    }

    #[must_use]
    pub fn points(&self) -> (&[Q], Thinning) {
        (&self.points, self.thinning)
    }

    #[must_use]
    pub fn kept(&self) -> usize {
        self.points.len()
    }

    #[must_use]
    pub const fn thinning(&self) -> Thinning {
        self.thinning
    }

    #[must_use]
    pub const fn is_full_resolution(&self) -> bool {
        self.thinning.is_full_resolution()
    }
}

impl<Q: Quantity> fmt::Display for Series<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} points, {}", self.points.len(), self.thinning)
    }
}
