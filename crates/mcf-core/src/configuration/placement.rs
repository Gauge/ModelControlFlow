use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Placement {
    Automatic,
    ProcessorOnly,
    AcceleratorPreferred,
    LayersOnAccelerator(u32),
}

impl fmt::Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Automatic => f.write_str("automatic"),
            Self::ProcessorOnly => f.write_str("processor only"),
            Self::AcceleratorPreferred => f.write_str("accelerator preferred"),
            Self::LayersOnAccelerator(layers) => write!(f, "{layers} layers on accelerator"),
        }
    }
}
