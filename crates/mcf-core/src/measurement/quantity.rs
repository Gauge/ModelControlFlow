use core::fmt;

pub trait Quantity: Copy + Ord + fmt::Debug + fmt::Display {
    const UNIT: &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(pub u64);

impl Quantity for Bytes {
    const UNIT: &'static str = "B";
}

impl Bytes {
    #[must_use]
    #[allow(clippy::integer_division)]
    pub fn human(self) -> String {
        const UNITS: [(&str, u64); 4] = [
            ("GiB", 1024 * 1024 * 1024),
            ("MiB", 1024 * 1024),
            ("KiB", 1024),
            ("B", 1),
        ];
        for (name, size) in UNITS {
            if self.0 >= size && size > 1 {
                let whole = self.0 / size;
                let tenths = (self.0 % size).saturating_mul(10) / size;
                return format!("{whole}.{tenths} {name}");
            }
        }
        format!("{} B", self.0)
    }
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1024 {
            write!(f, "{} {} ({})", self.0, Self::UNIT, self.human())
        } else {
            write!(f, "{} {}", self.0, Self::UNIT)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartsPerMillion(pub u64);

impl Quantity for PartsPerMillion {
    const UNIT: &'static str = "ppm";
}

impl fmt::Display for PartsPerMillion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, Self::UNIT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Count(pub u64);

impl Quantity for Count {
    const UNIT: &'static str = "";
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
