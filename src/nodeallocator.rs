use std::error::Error;
use std::result::Result;

//use crate::bknode::BkNode;

pub trait NodeAllocator<'a> {
    type Key: Clone;
    type Node;
    // TODO: type AllocationError: Error;

    fn new_child(&'a self, key: Self::Key) -> Result<Self::Node, Box<dyn Error>>;
}
