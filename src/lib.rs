#[macro_use] extern crate derivative;
#[macro_use] extern crate serde_derive;
extern crate byteorder;
extern crate serde_cbor;
extern crate sha2;

pub mod metric;
pub mod array_storage;
pub mod bkfile;

/*
mod bknode;
mod bktree;
pub mod keyquery;
pub mod keys;

pub use bknode::BkNode;
pub use bktree::BkInRamTree;
pub use bktree::BkTree;

*/
pub use metric::hamming::HammingMetric;


/// The concrete distance type shared across this crate. This is the result of all metric
/// comparisons.
pub type Dist = usize;
