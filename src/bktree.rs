/***
 * let metric = ...  // e.g. metric : HammingMetric<KeyType> = Default::default();
 * let tree = BkTree::new(metric);
 * tree.add(key1);
*/

use std::error::Error;
use std::option::Option;
use std::result::Result;

use crate::bknode::{BkNode, BkNodeMut};
use crate::keyquery::AsQuery;
use crate::metric::Metric as MetricTrait;

use crate::nodeallocator::NodeAllocator;
use crate::Dist;

pub trait BkTree<'n, Key: Clone> {
    type Metric: MetricTrait;
    type Node: 'n + BkNode<Key = Key>;

    fn root(&self) -> &Option<Self::Node>;
    fn max_depth_hint(&self) -> usize;

    fn find_each<F, Q>(&self, needle: &Q, tolerance: Dist, mut callback: F)
    where
        F: FnMut(Dist, &Key),
        Key: AsQuery<Q>,
        Self::Metric: MetricTrait<Query = Q>,
    {
        let mut stack: Vec<BkFindEntry<'_, Self::Node>> = Vec::with_capacity(self.max_depth_hint());
        let root = self.root();
        if let Some(ref root) = root {
            let dist = Self::Metric::distance(&root.key().as_query(), needle);
            stack.push(BkFindEntry { dist, node: root })
        }

        while let Some(candidate) = stack.pop() {
            // Enqueue the children.
            let min: Dist = candidate.dist.saturating_sub(tolerance);
            let max: Dist = candidate.dist.saturating_add(tolerance);
            let children = candidate.node.children_vector();
            for (dist, child) in children.iter() {
                if min <= *dist && *dist <= max {
                    let child_dist = Self::Metric::distance(&child.key().as_query(), needle);
                    stack.push(BkFindEntry {
                        dist: child_dist,
                        node: *child,
                    })
                }
            }

            // And maybe yield this node.
            if candidate.dist <= tolerance {
                callback(candidate.dist, candidate.node.key());
            }
        }
    }
}

#[derive(Debug, Clone)]
struct BkFindEntry<'n, N: 'n + BkNode> {
    dist: Dist,
    node: &'n N,
}

pub trait BkTreeRootMut<'n, Key: Clone>: BkTree<'n, Key>
where
    <Self as BkTree<'n, Key>>::Node: BkNodeMut<Key = Key>,
{
    // TODO: return an error of Alloc::AllocationError;
    type Alloc: 'n + NodeAllocator<'n, Node = <Self as BkTree<'n, Key>>::Node>;

    fn node_allocator(&self) -> &'n Self::Alloc;
    fn root_mut(&mut self) -> &mut Option<<Self as BkTree<'n, Key>>::Node>;
    fn max_depth_mut(&mut self) -> &mut usize;
    fn incr_node_count(&mut self);
}

pub trait BkTreeAdd<'n, Key: Clone>: BkTreeRootMut<'n, Key> + BkTree<'n, Key>
where
    <Self as BkTree<'n, Key>>::Node: 'n + BkNodeMut<Key = Key>,
{
    fn add(&mut self, key: Key) -> Result<(), Box<dyn Error>>;
}

impl<'n, Q, Metric, Key, N, Alloc: 'n, T> BkTreeAdd<'n, Key> for T
where
    N: 'n + BkNodeMut<Key = Key>,
    T: BkTreeRootMut<'n, Key, Metric = Metric, Node = N, Alloc = Alloc>,
    Key: Clone + AsQuery<Q> + PartialEq,
    Alloc: NodeAllocator<'n, Node = N, Key = Key>,
    Q: PartialEq,
    Metric: MetricTrait<Query = Q>,
{
    /// Add keys to a tree.
    ///
    /// Example:
    ///   let mut tree = BkTree::new(Metric, BkInRamAllocator());
    ///
    ///   tree.add(1);
    ///   tree.add(2);
    ///   tree.add(3);
    fn add(&mut self, key: Key) -> Result<(), Box<dyn Error>> {
        let mut insert_depth: usize = 0;
        let mut insert_cursor = None;
        let alloc = self.node_allocator();
        match self.root_mut() {
            root @ None => {
                insert_cursor = Some(root);
            }
            Some(ref mut root) => {
                let mut cur = root;
                let mut dist =
                    <Self as BkTree<Key>>::Metric::distance(&cur.key().as_query(), &key.as_query());

                // Find an empty child slot where the slot's distance from its node is the same as the
                // query's distance from the same node, or that this query is already present in
                // the tree.
                while cur.has_child_at(dist) && (dist == 0 || *cur.key() != key) {
                    cur = cur.child_at_mut(dist).unwrap();
                    dist = <Self as BkTree<Key>>::Metric::distance(
                        &cur.key().as_query(),
                        &key.as_query(),
                    );
                    insert_depth += 1;
                }

                assert!(!cur.has_child_at(dist) || *cur.key() == key);
                if *cur.key() != key {
                    insert_cursor = Some(cur.ref_child_at_mut(dist));
                }
            }
        }
        if let Some(ref mut position) = insert_cursor {
            assert!(position.is_none());
            let child = alloc.new_child(key)?;
            position.replace(child);
            self.incr_node_count();
        }
        if *self.max_depth_mut() < insert_depth {
            *self.max_depth_mut() = insert_depth;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metric::hamming::HammingMetric;
    use crate::metric::strlen::StrLenMetric;

    fn hamming_tree<'a>() -> BkInRamTree<'a, HammingMetric<u64>, BkInRamAllocator<'a>> {
        BkTree::new(&U64_ALLOC)
    }

    fn strlen_tree<'a>() -> BkInRamTree<'a, StrLenMetric, BkInRamAllocator<'a>> {
        BkTree::new(&STRING_ALLOC)
    }

    #[test]
    fn can_construct_empty_tree() {
        let tree = hamming_tree();
        println!("Empty Tree: {:?}", tree)
    }

    #[test]
    fn can_add_one_value() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        println!("Zero Tree: {:?}", tree)
    }

    #[test]
    fn can_add_repeated_roots() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        tree.add(&0u64);
        tree.add(&0u64);
        println!("Zeros Tree: {:?}", tree)
    }

    #[test]
    fn can_add_repeated_children() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        tree.add(&1u64);
        tree.add(&1u64);
        tree.add(&1u64);
        println!("Ones Tree: {:?}", tree)
    }

    #[test]
    fn can_add_distinct_values() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        tree.add(&1u64);
        tree.add(&2u64);
        tree.add(&3u64);
        println!("Many Tree: {:?}", tree)
    }

    #[test]
    fn can_add_distinct_values_in_reverse() {
        let mut tree = hamming_tree();
        tree.add(&3u64);
        tree.add(&2u64);
        tree.add(&1u64);
        tree.add(&0u64);
        println!("Many Tree Reversed: {:?}", tree)
    }

    #[test]
    fn can_construct_empty_string_tree() {
        let tree = strlen_tree();
        println!("Empty string tree: {:?}", tree);
    }

    #[test]
    fn can_add_empty_string() {
        let mut tree = strlen_tree();
        tree.add("");
        println!("empty string tree: {:?}", tree);
    }

    #[test]
    fn can_add_string() {
        let mut tree = strlen_tree();
        tree.add("foo");
        println!("foo string tree: {:?}", tree);
    }

    #[test]
    fn can_add_many_strings() {
        let mut tree = strlen_tree();
        tree.add("foo");
        tree.add("foo");
        tree.add("bar");
        tree.add("baz");
        tree.add("left");
        tree.add("ship");
        println!("many string tree: {:?}", tree);
    }

    /*
    #[test]
    fn can_add_find_exact_match() {
        let mut tree = strlen_tree();
        tree.add("foo");
        tree.add("bar");
        tree.add("baz");
        tree.add("left");
        tree.add("ship");
        println!("exact_match tree: {:?}", tree);
        let mut results = Vec::new();
        tree.find_each("foo", 0, |_, k| results.push(k.clone()));
        assert_eq!(vec!["foo", "bar", "baz"], results);
    }

    #[test]
    fn can_add_find_distant_match() {
        let mut tree = strlen_tree();
        tree.add("quux");
        tree.add("foo");
        tree.add("bar");
        tree.add("baz");
        tree.add("left");
        tree.add("ship");
        println!("distant_match tree: {:?}", tree);
        let mut results = Vec::new();
        tree.find_each("foo", 1, |_, k| results.push(k.clone()));
        assert_eq!(vec!["quux", "left", "ship", "foo", "bar", "baz"], results);
    }
    */
}
