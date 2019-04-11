use std::option::Option;
use std::vec::Vec;

use crate::bknode::BkNode;
use crate::Dist;

struct BkPreOrderEntry<'a, N: 'a + BkNode> {
    dist: Dist,
    node: &'a N,
}

pub struct BkPreOrder<'a, N>
where
    N: 'a + BkNode,
{
    root: Option<(Dist, &'a N)>,
    stack: Vec<BkPreOrderEntry<'a, N>>,
}

impl<'a, N> BkPreOrder<'a, N>
where
    N: 'a + BkNode,
{
    pub fn new(root: Option<&'a N>) -> BkPreOrder<'a, N> {
        Self::inner_new(0, root)
    }
    fn inner_new(rdist: Dist, root: Option<&'a N>) -> BkPreOrder<'a, N> {
        BkPreOrder {
            root: root.map(|r| (rdist, r)),
            stack: Vec::new(),
        }
    }
}

impl<'a, N, K> BkPreOrder<'a, N>
where
    K: 'a,
    N: 'a + BkNode<Key = K>,
{
    pub fn each<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Dist, usize, &K),
    {
        if let Some((dist, root)) = self.root.take() {
            let children = root.children_vector();
            callback(dist, children.len(), root.key().into());
            for (dist, node) in children.iter() {
                BkPreOrder::inner_new(*dist, Some(*node)).each_stacksafe(callback);
            }
        }
    }
}

// Can only be stack safe when the node child type is the same as the top level type.
impl<'a, N, K> BkPreOrder<'a, N>
where
    K: 'a,
    N: 'a + BkNode<Key = K>,
{
    pub fn each_stacksafe<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Dist, usize, &K),
    {
        if let Some((dist, node)) = self.root.take() {
            self.stack.push(BkPreOrderEntry { dist, node });
        }

        while let Some(entry) = self.stack.pop() {
            let children = entry.node.children_vector();
            callback(entry.dist, children.len(), entry.node.key());

            for (dist, node) in children.iter() {
                self.stack.push(BkPreOrderEntry {
                    dist: *dist,
                    node: *node,
                });
            }
        }
    }
}

impl<'a, N> Iterator for BkPreOrder<'a, N>
where
    N: 'a + BkNode,
{
    type Item = (Dist, N::Key);

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
