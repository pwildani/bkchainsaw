use crate::Dist;

pub trait BkNode {
    type Key;

    fn key(&self) -> &Self::Key;
    fn set_child_node(&mut self, distance: Dist, node: Self);
    fn has_child_at(&self, dist: Dist) -> bool;
    fn child_at_mut(&mut self, dist: Dist) -> Option<&mut Self>;
    fn child_at(&self, dist: Dist) -> Option<&Self>;
    fn children_iter<'a>(&'a self) -> Box<'a + Iterator<Item = (Dist, &'a Self)>>;
}
