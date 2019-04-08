pub mod array_storage;
mod bknode;
mod bktree;
pub mod keyquery;
pub mod keys;
mod metric;

pub use bknode::BkNode;
pub use bktree::BkInRamTree;
pub use bktree::BkTree;
pub use metric::hamming::HammingMetric;

#[macro_use]
extern crate derivative;
extern crate byteorder;

/// The concrete distance type shared across this crate. This is the result of all metric
/// comparisons.
pub type Dist = usize;
