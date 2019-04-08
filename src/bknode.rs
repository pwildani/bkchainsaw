use crate::Dist;
use std::vec::Vec;

pub trait BkNode {
    type Key;

    fn key(&self) -> &Self::Key;
    fn set_child_node(&mut self, distance: Dist, node: Self);
    fn has_child_at(&self, dist: Dist) -> bool;
    fn child_at_mut(&mut self, dist: Dist) -> Option<&mut Self>;
    fn child_at(&self, dist: Dist) -> Option<&Self>;
    // Needs RFC 1598: GATs: fn children_iter(&self) -> impl Iterator<Item = (Dist, &Self)>;
    fn children_vector(&self) -> Vec<(Dist, &Self)>;
}
