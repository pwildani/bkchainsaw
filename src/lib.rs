mod bknode;
mod bktree;
mod metric;
pub mod keys;
pub mod keyquery;

pub use bknode::BkNode;
pub use bktree::BkInRamTree;
pub use bktree::BkTree;
pub use metric::hamming::HammingMetric;

#[macro_use]
extern crate derivative;

/// The concrete distance type shared across this crate. This is the result of all metric
/// comparisons.
pub type Dist = usize;

