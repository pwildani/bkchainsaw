use std::option::Option;

use crate::bknode::BkNode;
use crate::metric::Metric;
use crate::keyquery::KeyQuery;

use crate::Dist;
#[derive(Debug, Clone)]

struct BkFindEntry<'a, N: 'a + BkNode> {
    dist: Dist,
    node: &'a N,
}

pub struct BkFind<'a, KQ, N: 'a, M>
where
    KQ: KeyQuery + Default,
    N: 'a + BkNode<Key = <KQ as KeyQuery>::Key>,
    M: Metric<<KQ as KeyQuery>::Query>,
{
    kq: &'a KQ,
    metric: &'a M,
    needle: &'a KQ::Query,
    tolerance: Dist,
    stack: Vec<BkFindEntry<'a, N>>,
}

impl<'a, KQ, N, M> BkFind<'a, KQ, N, M>
where
    KQ: 'a + KeyQuery + Default,
    N: 'a + BkNode<Key = <KQ as KeyQuery>::Key>,
    M: 'a + Metric<<KQ as KeyQuery>::Query>,
{
    pub fn new (kq: &'a KQ, metric: &'a M, max_depth: usize, root: Option<&'a N>, tolerance: Dist, needle: &'a KQ::Query) -> Self {
        // Initial setup. Push the root node onto the stack
        let mut stack: Vec<BkFindEntry<'a, N>> = Vec::with_capacity(max_depth);
        if let Some(ref root) = root {
            let cur = kq.distance(metric, &root.key(), needle);
            stack.push(BkFindEntry {
                dist: cur,
                node: root,
            });
        }
        BkFind {
            kq: kq,
            metric: metric,
            needle: needle,
            tolerance: tolerance,
            stack: stack,
        }
    }

    pub fn each<F: FnMut(Dist, &'a KQ::Key)>(&'a mut self, mut callback: F)
        where F: FnMut(Dist, &'a KQ::Key)
    {
        while let Some(candidate) = self.stack.pop() {
            // Enqueue the children.
            let min: usize = candidate.dist.saturating_sub(self.tolerance);
            let max: usize = candidate.dist.saturating_add(self.tolerance);
            for (dist, ref child) in candidate.node.children_iter() {
                if min <= dist && dist <= max {
                    let child_dist = self.kq.distance(self.metric, &child.key(), self.needle);
                    self.stack.push(BkFindEntry {
                        dist: child_dist,
                        node: child,
                    })
                }
            }

            // And maybe yield this node.
            if candidate.dist <= self.tolerance {
                callback(candidate.dist, candidate.node.key());
            }
        }
    }
}

impl<'a, KQ, N, M> Iterator for BkFind<'a, KQ, N, M>
where
    KQ: 'a + KeyQuery + Default,
    N: 'a + BkNode<Key = <KQ as KeyQuery>::Key>,
    M: 'a + Metric<<KQ as KeyQuery>::Query>,
{
    type Item = (Dist, KQ::Key);

    fn next(&mut self) -> Option<(Dist, KQ::Key)> {
        while let Some(candidate) = self.stack.pop() {
            // Enqueue the children.
            let min: usize = candidate.dist.saturating_sub(self.tolerance);
            let max: usize = candidate.dist.saturating_add(self.tolerance);
            for (dist, ref child) in candidate.node.children_iter() {
                if min <= dist && dist <= max {
                    let child_dist = self.kq.distance(self.metric, &child.key(), self.needle);
                    self.stack.push(BkFindEntry {
                        dist: child_dist,
                        node: child,
                    })
                }
            }

            // And maybe yield this node.
            if candidate.dist <= self.tolerance {
                return Some((candidate.dist, candidate.node.key().clone()));
            }
        }
        return None;
    }
}
