use crate::Dist;
use std::vec::Vec;

pub trait BkNode {
    type Key;

    fn key(&self) -> &Self::Key;
    fn has_child_at(&self, dist: Dist) -> bool;
    fn child_at(&self, dist: Dist) -> Option<&Self>;
    fn children_vector(&self) -> Vec<(Dist, &Self)>;

    // Needs RFC 1598: GATs: because the child is not copyable and is owned by this code (or
    // rather, by its allocator)
    // fn children_iter(&self) -> impl Iterator<Item = (Dist, &Self)>;
}

pub trait BkNodeMut: BkNode
where
    Self: Sized,
{
    fn child_at_mut(&mut self, dist: Dist) -> Option<&mut Self>;
    fn ref_child_at_mut(&mut self, dist: Dist) -> &mut Option<Self>;
}
