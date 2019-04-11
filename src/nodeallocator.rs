use crate::bknode::BkNode;

pub trait NodeAllocator<'a> {
    type Key: Clone;
    type Node;
    fn new_root(&'a self, key: Self::Key) -> Self::Node;
    fn new_child(&'a self, key: Self::Key) -> Self::Node;
}
