/***
 * let metric = ...  // e.g. metric : HammingMetric<KeyType> = Default::default();
 * let tree = BkTree::new(metric);
 * tree.add(key1);
*/

use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::option::Option;
use std::vec::Vec;

use crate::bknode::BkNode;
use crate::keyquery::KeyQuery;
use crate::metric::Metric;

use crate::Dist;

pub trait NodeAllocator<'a> {
    type Key: Clone;
    type Node: BkNode<Key = Self::Key>;
    fn new_root(&'a self, key: Self::Key) -> Self::Node;
    fn new_child(&'a self, key: Self::Key) -> Self::Node;
}

/// BK tree node optimised for small distances.
/// TODO: consider feature(const_generics) to drop the vec overhead, once that's stable.
/// (https://github.com/rust-lang/rust/issues/44580)
pub struct BkInRam<K> {
    key: K,
    children: Vec<Option<Self>>,
}

impl<K> BkInRam<K> {
    pub fn new(key: K) -> BkInRam<K> {
        BkInRam {
            key: key,
            children: Vec::with_capacity(16),
        }
    }

    fn children_iter(&self) -> impl Iterator<Item = (Dist, &Self)> {
        self.children
            .iter()
            .enumerate()
            .rev() // Find here looks at the last child first, and things play nicer if the closest is first.
            .filter(|(_, child)| child.is_some())
            .map(|(dist, child)| (dist.into(), child.as_ref().unwrap()))
    }
}

impl<'a, K> BkNode for BkInRam<K> {
    type Key = K;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn has_child_at(&self, dist: Dist) -> bool {
        let child: Option<&Option<Self>> = self.children.get(dist);
        match child {
            None | Some(None) => false,
            Some(_) => true,
        }
    }

    fn child_at_mut(&mut self, dist: Dist) -> Option<&mut Self> {
        match self.children.get_mut(dist) {
            None | Some(None) => None,
            Some(child @ Some(_)) => child.as_mut(),
        }
    }

    fn child_at(&self, dist: Dist) -> Option<&Self> {
        match self.children.get(dist) {
            None | Some(None) => None,
            Some(child @ Some(_)) => child.as_ref(),
        }
    }

    fn set_child_node(&mut self, dist: Dist, node: Self) {
        if self.children.len() <= dist {
            self.children.resize_with(dist + 1, || None);
        }
        assert!(!self.has_child_at(dist));
        self.children[dist] = Some(node);
    }

    fn children_vector(&self) -> Vec<(Dist, &Self)> {
        self.children_iter().collect()
    }
}

impl<K> Debug for BkInRam<K>
where
    K: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let children: Vec<_> = self.children.iter().filter(|&x| x.is_some()).collect();
        f.debug_map().entry(&self.key, &children).finish()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct BkInRamAllocator<'a, K>(#[derivative(Debug = "ignore")] PhantomData<&'a K>);
// The PhantomData above is misrepresenting 'a. It's the lifetime of the nodes, not the lifetime
// of the keys of the nodes.

impl<'a, K: Clone> NodeAllocator<'a> for BkInRamAllocator<'a, K> {
    type Key = K;
    type Node = BkInRam<K>;

    fn new_root(&'a self, key: K) -> Self::Node {
        BkInRam::new(key)
    }
    fn new_child(&'a self, key: K) -> Self::Node {
        BkInRam::new(key)
    }
}

#[derive(Debug)]
pub struct BkTree<'nodes, KQ, M, A>
where
    KQ: KeyQuery,
    M: Metric<<KQ as KeyQuery>::Query>,
    A: 'nodes + NodeAllocator<'nodes>,
{
    root: Option<A::Node>,
    max_depth: usize,
    metric: M,
    node_allocator: &'nodes A,
    kq: KQ,
}

pub type BkInRamTree<'a, KQ, M> = BkTree<'a, KQ, M, BkInRamAllocator<'a, <KQ as KeyQuery>::Key>>;

impl<'a, K, KQ, M, N, Alloc> BkTree<'a, KQ, M, Alloc>
where
    K: Clone,
    KQ: KeyQuery<Key = K> + Default,
    M: Metric<<KQ as KeyQuery>::Query>,
    N: 'a + BkNode<Key = K>,
    Alloc: NodeAllocator<'a, Key = K, Node = N>,
{
    pub fn new(metric: M, alloc: &'a Alloc) -> Self {
        BkTree {
            root: None,
            max_depth: 0,
            metric: metric,
            node_allocator: alloc,
            kq: Default::default(),
        }
    }
}

impl<'a, K: 'a + Clone, KQ, M, N, Alloc> BkTree<'a, KQ, M, Alloc>
where
    K: 'a + Clone,
    KQ: KeyQuery<Key = K> + Default,
    M: Metric<<KQ as KeyQuery>::Query>,
    N: 'a + BkNode<Key = K>,
    Alloc: NodeAllocator<'a, Key = K, Node = N>,
{
    /// Add keys to a tree.
    ///
    /// Currently only implemented if the root node type is the same as the child node type.
    ///
    /// Example:
    ///   let mut tree = BkTree::new(Metric, BkInRamAllocator());
    ///
    ///   tree.add(1);
    ///   tree.add(2);
    ///   tree.add(3);
    ///
    pub fn add(&mut self, query: &KQ::Query) {
        let mut root = self.root.take();
        match root {
            None => {
                root = Some(self.node_allocator.new_root(self.kq.to_key(query)));
            }
            Some(ref mut root) => {
                let mut insert_depth = 0;
                let mut cur = root;
                let mut dist = self.kq.distance(&self.metric, cur.key(), query);

                // Find an empty child slot where the slot's distance from its node is the same as the
                // query's distance from the same node, or that this query is already present in
                // the tree.
                while cur.has_child_at(dist) && (dist == 0 || !self.kq.eq(cur.key(), query)) {
                    cur = cur.child_at_mut(dist).unwrap();
                    dist = self.kq.distance(&self.metric, cur.key(), query);
                    insert_depth += 1;
                }

                assert!(!cur.has_child_at(dist) || self.kq.eq(cur.key(), query));
                if !self.kq.eq(cur.key(), query) {
                    let child = self.node_allocator.new_child(self.kq.to_key(query));
                    cur.set_child_node(dist, child);
                }
                if self.max_depth < insert_depth {
                    self.max_depth = insert_depth;
                }
            }
        }
        self.root = root;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::StringKey;
    use crate::keys::U64Key;
    use crate::metric::hamming::HammingMetric;
    use crate::metric::strlen::StrLenMetric;

    const U64_ALLOC: BkInRamAllocator<'static, u64> = BkInRamAllocator(PhantomData);
    const STRING_ALLOC: BkInRamAllocator<'static, String> = BkInRamAllocator(PhantomData);

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
}
