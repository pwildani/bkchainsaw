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
use crate::metric::Metric;

use crate::Dist;

pub trait NodeAllocator {
    type Node: BkNode;
    fn new(&mut self, key: <Self::Node as BkNode>::Key) -> Self::Node;
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
}

impl<K> BkNode for BkInRam<K> {
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

    fn children_iter<'a>(&'a self) -> Box<'a + Iterator<Item = (Dist, &'a Self)>> {
        Box::new(
            self.children
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, child)| child.is_some())
                .map(|(dist, child)| (dist.into(), child.as_ref().unwrap())),
        )
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
pub struct BkInRamAllocator<K>(#[derivative(Debug = "ignore")] PhantomData<K>);

impl<K: Clone> NodeAllocator for BkInRamAllocator<K> {
    type Node = BkInRam<K>;

    fn new(&mut self, key: K) -> Self::Node {
        BkInRam::new(key.clone())
    }
}

impl<K> Default for BkInRamAllocator<K> {
    fn default() -> BkInRamAllocator<K> {
        BkInRamAllocator(PhantomData)
    }
}

pub trait KeyQuery: Default {
    type Key: Clone;
    type Query: ?Sized;

    fn distance<M: Metric<Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> Dist;
    fn to_key(&self, query: &Self::Query) -> Self::Key;
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct U64Key;

impl KeyQuery for U64Key {
    type Key = u64;
    type Query = u64;

    #[inline]
    fn distance<M: Metric<Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> Dist {
        metric.distance(key, query)
    }

    #[inline]
    fn to_key(&self, query: &Self::Query) -> Self::Key {
        *query
    }

    #[inline]
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool {
        key == query
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StringKey;

impl KeyQuery for StringKey {
    type Key = String;
    type Query = str;

    #[inline]
    fn distance<M: Metric<Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> Dist {
        metric.distance(key.as_str(), &query)
    }

    #[inline]
    fn to_key(&self, query: &Self::Query) -> String {
        query.to_string()
    }

    #[inline]
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool {
        key.as_str() == query
    }
}

#[derive(Debug)]
pub struct BkTree<KQ, M, A>
where
    KQ: KeyQuery,
    M: Metric<<KQ as KeyQuery>::Query>,
    A: NodeAllocator,
{
    root: Option<A::Node>,
    max_depth: usize,
    metric: M,
    node_allocator: A,
    kq: KQ,
}

pub type BkInRamTree<KQ, M> = BkTree<KQ, M, BkInRamAllocator<<KQ as KeyQuery>::Key>>;

impl<KQ: KeyQuery, M: Metric<KQ::Query>> BkInRamTree<KQ, M> {
    pub fn new(metric: M) -> Self {
        let alloc: BkInRamAllocator<KQ::Key> = Default::default();
        BkInRamTree::new_with_allocator(metric, alloc)
    }
}

impl<K: Clone, KQ, M, N, Alloc> BkTree<KQ, M, Alloc>
where
    KQ: KeyQuery<Key = K> + Default,
    M: Metric<<KQ as KeyQuery>::Query>,
    N: BkNode<Key = K>,
    Alloc: NodeAllocator<Node = N>,
{
    pub fn new_with_allocator(metric: M, alloc: Alloc) -> Self {
        BkTree {
            root: None,
            max_depth: 0,
            metric: metric,
            node_allocator: alloc,
            kq: Default::default(),
        }
    }

    /// Add keys to a tree.
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
                let child = self.node_allocator.new(self.kq.to_key(query));
                root = Some(child);
            }
            Some(ref mut root) => {
                let mut cur = root;
                let cur_key = cur.key();
                let mut dist = self.distance(cur_key, query);
                let mut d = 1;
                while cur.has_child_at(dist) && (dist == 0 || !self.kq.eq(cur.key(), query)) {
                    cur = cur.child_at_mut(dist).unwrap();
                    dist = self.distance(cur.key(), query);
                    d += 1;
                }
                assert!(!cur.has_child_at(dist) || self.kq.eq(cur.key(), query));
                if !self.kq.eq(cur.key(), query) {
                    let child = self.node_allocator.new(self.kq.to_key(query));
                    cur.set_child_node(dist, child);
                }
                if self.max_depth < d {
                    self.max_depth = d;
                }
            }
        }

        self.root = root;
    }

    pub fn find<'a>(
        &'a self,
        needle: &'a KQ::Query,
        tolerance: Dist,
    ) -> impl 'a + Iterator<Item=(Dist, K)> {
        BkFind::new(&self.kq, &self.metric, self.max_depth, self.root.as_ref(), tolerance, needle)
    }

    fn distance(&self, key: &KQ::Key, query: &KQ::Query) -> Dist {
        self.kq.distance(&self.metric, key, query)
    }
}

#[derive(Debug, Clone)]
struct BkFindEntry<'a, N: 'a + BkNode> {
    dist: Dist,
    node: &'a N,
}

struct BkFind<'a, KQ, N: 'a, M>
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
    fn new (kq: &'a KQ, metric: &'a M, max_depth: usize, root: Option<&'a N>, tolerance: Dist, needle: &'a KQ::Query) -> Self {
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


#[cfg(test)]
mod tests {
    use crate::bktree::BkInRamTree;
    use crate::bktree::BkTree;
    use crate::bktree::StringKey;
    use crate::bktree::U64Key;
    use crate::metric::hamming::HammingMetric;
    use crate::metric::strlen::StrLenMetric;

    fn hamming_tree() -> BkInRamTree<U64Key, HammingMetric<u64>> {
        BkTree::new(Default::default())
        //let metric: HammingMetric<u64> = Default::default();
        //let mut tree : BkInRamTree<u64, HammingMetric<u64>, u64> = BkTree::new(metric);
        //tree
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
        let tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        println!("Empty string tree: {:?}", tree);
    }

    #[test]
    fn can_add_empty_string() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("");
        println!("empty string tree: {:?}", tree);
    }

    #[test]
    fn can_add_string() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("foo");
        println!("foo string tree: {:?}", tree);
    }

    #[test]
    fn can_add_many_strings() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("foo");
        tree.add("bar");
        tree.add("baz");
        tree.add("left");
        tree.add("ship");
        println!("many string tree: {:?}", tree);
    }

    #[test]
    fn can_add_find_exact_match() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("foo");
        tree.add("bar");
        tree.add("baz");
        tree.add("left");
        tree.add("ship");
        let results = tree.find("foo", 0).map(|(_, s)| s).collect::<Vec<String>>();
        assert_eq!(vec!["foo", "bar", "baz"], results);
    }

    #[test]
    fn can_add_find_distant_match() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("quux");
        tree.add("foo");
        tree.add("bar");
        tree.add("baz");
        tree.add("left");
        tree.add("ship");
        let results = tree.find("foo", 1).map(|(_, s)| s).collect::<Vec<String>>();
        assert_eq!(vec!["quux", "left", "ship", "foo", "bar", "baz"], results);
    }
}
