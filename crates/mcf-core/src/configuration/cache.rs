use core::fmt;

pub const BLOCK: u64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum CacheType {
    F32,
    #[default]
    F16,
    Bf16,
    Q8_0,
    Q5_1,
    Q5_0,
    Q4_1,
    Q4_0,
    Iq4Nl,
}

impl CacheType {
    pub const ALL: [Self; 9] = [
        Self::F32,
        Self::F16,
        Self::Bf16,
        Self::Q8_0,
        Self::Q5_1,
        Self::Q5_0,
        Self::Q4_1,
        Self::Q4_0,
        Self::Iq4Nl,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F16 => "f16",
            Self::Bf16 => "bf16",
            Self::Q8_0 => "q8_0",
            Self::Q5_1 => "q5_1",
            Self::Q5_0 => "q5_0",
            Self::Q4_1 => "q4_1",
            Self::Q4_0 => "q4_0",
            Self::Iq4Nl => "iq4_nl",
        }
    }

    #[must_use]
    pub fn parse(said: &str) -> Option<Self> {
        let wanted = said.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|held| held.as_str() == wanted)
    }

    /// What [`BLOCK`] cached elements cost, in bytes.
    ///
    /// The quantized types carry their scales alongside the values, so a rate
    /// per element is not a whole number for them. The rate is given over a
    /// block so the arithmetic stays exact where it matters, which is a cache
    /// of millions of elements rather than a handful.
    #[must_use]
    pub const fn bytes_per_block(self) -> u64 {
        match self {
            Self::F32 => 128,
            Self::F16 | Self::Bf16 => 64,
            Self::Q8_0 => 34,
            Self::Q5_1 => 24,
            Self::Q5_0 => 22,
            Self::Q4_1 => 20,
            Self::Q4_0 | Self::Iq4Nl => 18,
        }
    }

    #[must_use]
    pub const fn is_quantized(self) -> bool {
        !matches!(self, Self::F32 | Self::F16 | Self::Bf16)
    }

    /// What a cache of `elements` costs, rounded up to the byte.
    ///
    /// The wide types are exact at every count. A quantized one is charged its
    /// share of a block rather than a whole block, because a latent cache can
    /// hold fewer elements than a block and charging it a whole one would
    /// report a reserve twice the size of the thing being reserved.
    #[must_use]
    pub fn bytes_for(self, elements: u64) -> Option<u64> {
        Some(
            elements
                .checked_mul(self.bytes_per_block())?
                .div_ceil(BLOCK),
        )
    }

    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::F32 => "the widest, and twice what an engine holds by default",
            Self::F16 => "what an engine holds by default",
            Self::Bf16 => "as wide as the default, with the range of a larger float",
            Self::Q8_0 => "about half the default, and the smallest step most models do not notice",
            Self::Q5_1 | Self::Q5_0 => "about a third of the default",
            Self::Q4_1 | Self::Q4_0 | Self::Iq4Nl => {
                "under a third of the default, and far enough down that answers change"
            }
        }
    }
}

impl fmt::Display for CacheType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::{BLOCK, CacheType};

    #[test]
    fn the_default_is_what_an_engine_holds_without_being_asked() {
        assert_eq!(CacheType::default(), CacheType::F16);
        assert_eq!(CacheType::F16.bytes_per_block(), BLOCK * 2);
        assert!(!CacheType::F16.is_quantized());
    }

    #[test]
    fn every_type_round_trips_through_the_name_the_engine_takes() {
        for held in CacheType::ALL {
            assert_eq!(CacheType::parse(held.as_str()), Some(held), "{held}");
            assert_eq!(
                CacheType::parse(&held.as_str().to_ascii_uppercase()),
                Some(held),
                "{held} is not read back when it is shouted"
            );
        }
        assert_eq!(CacheType::parse("q3_k"), None);
        assert_eq!(CacheType::parse(""), None);
    }

    #[test]
    fn a_narrower_cache_costs_less_for_the_same_conversation() {
        // A token of the 27B this was read against: 68 KiB held as f16.
        let elements = 34_816;
        let wide = CacheType::F16.bytes_for(elements).expect("f16 is sized");
        let narrow = CacheType::Q8_0.bytes_for(elements).expect("q8_0 is sized");
        assert_eq!(wide, 68 * 1024);
        assert!(
            narrow.saturating_mul(2) > wide && narrow.saturating_mul(16) < wide.saturating_mul(9),
            "q8_0 should be a little over half of f16: {narrow} against {wide}"
        );
        for held in CacheType::ALL {
            let cost = held.bytes_for(elements).expect("every type is sized");
            assert_eq!(cost > wide, held == CacheType::F32, "{held}");
        }
    }

    #[test]
    fn a_wide_cache_is_exact_and_a_narrow_one_is_charged_its_share() {
        assert_eq!(CacheType::F16.bytes_for(0), Some(0));
        assert_eq!(CacheType::F16.bytes_for(1), Some(2));
        assert_eq!(CacheType::F16.bytes_for(16), Some(32));
        assert_eq!(CacheType::F32.bytes_for(16), Some(64));

        assert_eq!(CacheType::Q8_0.bytes_for(BLOCK), Some(34));
        assert_eq!(CacheType::Q8_0.bytes_for(BLOCK * 2), Some(68));
        assert_eq!(
            CacheType::Q8_0.bytes_for(1),
            Some(2),
            "a single element is charged its share of a block, not a whole one"
        );
    }

    #[test]
    fn the_quantized_ones_are_the_ones_that_need_flash_attention() {
        for held in CacheType::ALL {
            assert_eq!(
                held.is_quantized(),
                held.bytes_per_block() < CacheType::F16.bytes_per_block(),
                "{held}"
            );
        }
    }
}
