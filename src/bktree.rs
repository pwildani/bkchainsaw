/***
 * let metric = ...  // e.g. metric : HammingMetric<KeyType> = Default::default();
 * let tree = BkTree::new(metric);
 * tree.add(key1);
*/

use std::error::Error;
//use std::fmt;
//use std::fmt::Debug;
//use std::fmt::Formatter;
//use std::marker::PhantomData;
use std::option::Option;
// use std::vec::Vec;
use std::result::Result;

use crate::bknode::{BkNode, BkNodeMut};
use crate::keyquery::KeyQuery;
use crate::metric::Metric as MetricTrait;

use crate::nodeallocator::NodeAllocator;
use crate::Dist;

pub trait BkTree<Key: Clone> {
    type KQ: KeyQuery<Key = Key>;
    type Metric: MetricTrait<<Self::KQ as KeyQuery>::Query>;
    type Node: BkNode<Key = Key>;

    fn find_each<'a, F>(
        &'a self,
        needle: &'a <Self::KQ as KeyQuery>::Query,
        tolerance: Dist,
        callback: F,
    ) where
        F: FnMut(Dist, &<Self::KQ as KeyQuery>::Key);
}

pub trait BkTreeRootMut<'a, Key: Clone>: BkTree<Key>
where
    <Self as BkTree<Key>>::Node: BkNodeMut<Key = Key>,
{
    // TODO: return an error of Alloc::AllocationError;
    type Alloc: 'a + NodeAllocator<'a, Node = <Self as BkTree<Key>>::Node>;

    fn node_allocator(&mut self) -> &'a Self::Alloc;
    fn root_mut(&mut self) -> &mut Option<<Self as BkTree<Key>>::Node>;
    fn max_depth_mut(&mut self) -> &mut usize;
    fn incr_node_count(&mut self);
}

pub trait BkTreeAdd<'a, Key: Clone>: BkTreeRootMut<'a, Key> + BkTree<Key>
where
    <Self as BkTree<Key>>::Node: BkNodeMut<Key = Key>,
{
    fn add(&mut self, key: <Self::KQ as KeyQuery>::Query) -> Result<(), Box<dyn Error>>;
}

impl<
        'a,
        Q: Sized,
        Key: Clone,
        KQ,
        M,
        N: BkNodeMut<Key = Key>,
        Alloc: 'a,
        T: BkTreeRootMut<'a, Key, Metric = M, Node = N, Alloc = Alloc, KQ = KQ>,
    > BkTreeAdd<'a, Key> for T
where
    Alloc: NodeAllocator<'a, Node = N, Key = Key>,
    KQ: KeyQuery<Key = Key, Query = Q>,
    M: MetricTrait<Q>,
{
    /// Add keys to a tree.
    ///
    /// Example:
    ///   let mut tree = BkTree::new(Metric, BkInRamAllocator());
    ///
    ///   tree.add(1);
    ///   tree.add(2);
    ///   tree.add(3);
    fn add(
        &mut self,
        query: <<Self as BkTree<Key>>::KQ as KeyQuery>::Query,
    ) -> Result<(), Box<dyn Error>> {
        let mut root = self.root_mut().take();
        let mut insert_depth: usize = 0;
        let query_as_key: Key = <Self as BkTree<Key>>::KQ::to_key_static(&query);
        match root {
            None => {
                root = Some(self.node_allocator().new_root(query_as_key)?);
                self.incr_node_count();
            }
            Some(ref mut root) => {
                let mut cur = root;
                let mut dist = <Self as BkTree<Key>>::Metric::distance_static(
                    <Self as BkTree<Key>>::KQ::to_query_static(cur.key()),
                    &query,
                );

                // Find an empty child slot where the slot's distance from its node is the same as the
                // query's distance from the same node, or that this query is already present in
                // the tree.
                while cur.has_child_at(dist)
                    && (dist == 0 || !<Self as BkTree<Key>>::KQ::eq_static(cur.key(), &query))
                {
                    cur = cur.child_at_mut(dist).unwrap();
                    dist = <Self as BkTree<Key>>::Metric::distance_static(
                        <Self as BkTree<Key>>::KQ::to_query_static(cur.key()),
                        &query,
                    );
                    insert_depth += 1;
                }

                assert!(
                    !cur.has_child_at(dist)
                        || <Self as BkTree<Key>>::KQ::eq_static(cur.key(), &query)
                );
                if !<Self as BkTree<Key>>::KQ::eq_static(cur.key(), &query) {
                    let child = self.node_allocator().new_child(query_as_key)?;
                    cur.set_child_node(dist, child);
                    self.incr_node_count();
                }
            }
        }
        if let Some(root2) = root.take() {
            self.root_mut().replace(root2);
        }
        if *self.max_depth_mut() < insert_depth {
            *self.max_depth_mut() = insert_depth;
        }
        return Ok(());
    }
}

/*

    // E0309: Needs GAT with lifetimes to express that the BkFind iterator's innards should not
    // live longer than the tree itself.
    pub fn find<'a, 'b: 'a>(
        &'b self,
        needle: &'b KQ::Query,
        tolerance: Dist,
    ) -> impl 'a + Iterator<Item = (Dist, K)> {
        use super::find::BkFind;
        BkFind::new(&self.kq, &self.metric, self.max_depth, self.root.as_ref(), tolerance, needle)
    }

    pub fn in_order<'a, 'b: 'a>(&'b self) -> impl 'a + Iterator<Item=(Dist, K)> {
         use super::inorder::BkInOrder;
         BkInOrder::new(self.root.as_ref())
    }
*/
/*
impl<'a, K: 'a + Clone, KQ, M, N, Alloc> BkTree<'a, KQ, M, Alloc>
where
    K: 'a + Clone,
    KQ: KeyQuery<Key = K> + Default,
    M: Metric<<KQ as KeyQuery>::Query>,
    N: 'a + BkNode<Key = K>,
    Alloc: NodeAllocator<'a, Key = K, Node = N>,
{
    pub fn find_each<F>(&'a self, needle: &'a KQ::Query, tolerance: Dist, callback: F)
    where
        F: FnMut(Dist, &KQ::Key),
    {
        use super::find::BkFind;
        BkFind::new(
            &self.kq,
            &self.metric,
            self.max_depth,
            self.root.as_ref(),
            tolerance,
            needle,
        )
        .each(callback)
    }
}
*/

/*
impl<'a, K: 'a + Clone, KQ, M, N, Alloc> BkTree<'a, KQ, M, Alloc>
where
    K: 'a + Clone,
    KQ: KeyQuery<Key = K> + Default,
    M: Metric<<KQ as KeyQuery>::Query>,
    N: 'a + BkNode<Key = K>,
    Alloc: NodeAllocator<'a, Key = K, Node = N>,
{
    /// Traverse the tree, calling callback for each key. Parents are passed before children.
    ///
    /// Callback args:
    ///    * distance from parent
    ///    * number of children of the node on which key was found
    ///    * key
    pub fn preorder_each<F>(&'a self, callback: &mut F)
    where
        F: FnMut(Dist, usize, &K),
    {
        use super::preorder::BkPreOrder;
        BkPreOrder::new(self.root.as_ref()).each(callback);
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::StringKey;
    use crate::keys::U64Key;
    use crate::metric::hamming::HammingMetric;
    use crate::metric::strlen::StrLenMetric;

    fn hamming_tree<'a>() -> BkInRamTree<'a, U64Key, HammingMetric<u64>> {
        BkTree::new(Default::default(), &U64_ALLOC)
    }

    fn strlen_tree<'a>() -> BkInRamTree<'a, StringKey, StrLenMetric> {
        BkTree::new(Default::default(), &STRING_ALLOC)
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
