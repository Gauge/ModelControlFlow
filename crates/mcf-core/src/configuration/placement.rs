//! Where the operator asked the model to be put.
//!
//! Intent v16 settles the split: **placement is a declared intent in the
//! configuration, and the layout that actually resulted is a condition.**
//! Divergence between them is a finding, and often an informative one — it is
//! how a configuration visibly fails to transfer to another machine.
//!
//! Only the declared half is here. The realized half cannot be: it names
//! accelerators, and this module is the one place in MCF where naming hardware
//! is structurally impossible (B57, B-272). It travels in the condition set
//! instead, as `Floor::realized_placement`.

use core::fmt;

/// What the operator asked for.
///
/// Deliberately coarse. A placement intent that named a device would be
/// hardware in the identity through the back door; what a configuration
/// declares is a *policy*, and which devices satisfied it is what the machine
/// answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Placement {
    /// MCF chooses. B1 makes that a visible, attributed, overridable default.
    Automatic,
    /// Everything on the processor, whatever accelerators are present.
    ProcessorOnly,
    /// As much as fits on accelerators, the remainder on the processor.
    AcceleratorPreferred,
    /// A stated number of layers on accelerators, the rest on the processor.
    ///
    /// A count rather than a device list: *how much* is a property of the
    /// configuration, *where* is a property of the machine.
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
