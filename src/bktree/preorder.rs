use std::vec::Vec;
use std::option::Option;

use crate::bknode::BkNode;
use crate::Dist;

#[derive(Debug)]
struct BkPreOrderEntry<'a, N: 'a + BkNode> {
    dist: Dist,
    node: &'a N,
}

#[derive(Debug)]
pub struct BkPreOrder<'a, N: 'a + BkNode> {
    stack: Vec<BkPreOrderEntry<'a, N>>,
}

impl<'b, 'a: 'b, N: 'a + BkNode> BkPreOrder<'b, N> {
    pub fn new(root: Option<&'a N>) -> BkPreOrder<'b, N> {
        BkPreOrder {
            stack: root.iter().map(|x| BkPreOrderEntry{dist: 0, node: *x}).collect(),
        }
    }

    pub fn each<F>(&mut self, mut callback: F)
    where
        F: FnMut(Dist, usize, &N::Key),
    {
        while let Some(entry) = self.stack.pop() {
            let num_children = entry.node.children_iter().count();
            callback(entry.dist, num_children, entry.node.key());

            for (dist, node) in entry.node.children_iter() {
                self.stack.push(BkPreOrderEntry{dist, node});
            }
        }
    }
}

impl<'a, N: 'a + BkNode> Iterator for BkPreOrder<'a, N> {
    type Item = (Dist, N::Key);

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
