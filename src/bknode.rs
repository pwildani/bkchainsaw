pub trait BkNode {
    type Key;
    type Dist;
    fn key(&self) -> &Self::Key;
    fn set_child_node(&mut self, distance: Self::Dist, node: Self);
    fn has_child_at(&self, dist: Self::Dist) -> bool;
    fn child_at_mut(&mut self, dist: Self::Dist) -> Option<&mut Self>;
}
