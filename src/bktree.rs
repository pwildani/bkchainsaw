use std::borrow::Borrow;
/**
 * let metric = ...  // e.g. metric : HammingMetric<KeyType> = Default::default();
 * let tree = BkTree::new(metric);
 * tree.add(key1);
 */
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::option::Option;

use crate::bknode::BkNode;
use crate::metric::Metric;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dist(u32);

impl From<Dist> for usize {
    fn from(i: Dist) -> usize {
        i.0 as usize
    }
}

impl From<u32> for Dist {
    fn from(i: u32) -> Dist {
        Dist(i)
    }
}

impl From<u64> for Dist {
    fn from(i: u64) -> Dist {
        Dist(i as u32)
    }
}

pub trait NodeAllocator {
    type Node: BkNode;
    fn new(&mut self, key: <Self::Node as BkNode>::Key) -> Self::Node;
}

/// BK tree node optimised for small distances.
///  TODO: use feature(const_generics) to drop the vec overhead.
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
    type Dist = super::bktree::Dist;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn has_child_at(&self, dist: Dist) -> bool {
        let udist: usize = dist.into();
        let child: Option<&Option<Self>> = self.children.get(udist);
        match child {
            None | Some(None) => false,
            Some(_) => true,
        }
    }

    fn child_at_mut(&mut self, dist: Dist) -> Option<&mut Self> {
        let udist: usize = dist.into();
        match self.children.get_mut(udist) {
            None | Some(None) => None,
            Some(child @ Some(_)) => child.as_mut(),
        }
    }

    fn set_child_node(&mut self, dist: Dist, node: Self) {
        let udist: usize = dist.into();
        if self.children.len() <= udist {
            self.children.resize_with(udist + 1, || None);
        }
        assert!(!self.has_child_at(dist));
        self.children[udist] = Some(node);
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

    fn distance<D, M: Metric<D, Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> D;
    fn to_key(&self, query: &Self::Query) -> Self::Key;
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct U64Key;

impl KeyQuery for U64Key {
    type Key = u64;
    type Query = u64;

    #[inline]
    fn distance<D, M: Metric<D, Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> D {
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
    fn distance<D, M: Metric<D, Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> D {
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
    M: Metric<Dist, <KQ as KeyQuery>::Query>,
    A: NodeAllocator,
{
    root: Option<A::Node>,
    metric: M,
    node_allocator: A,
    kq: KQ,
}

pub type BkInRamTree<KQ, M> = BkTree<KQ, M, BkInRamAllocator<<KQ as KeyQuery>::Key>>;

impl<KQ: KeyQuery, M: Metric<Dist, KQ::Query>> BkInRamTree<KQ, M> {
    pub fn new(metric: M) -> Self {
        let alloc: BkInRamAllocator<KQ::Key> = Default::default();
        BkInRamTree::new_with_allocator(metric, alloc)
    }
}

impl<K: Clone, KQ, M, N, Alloc> BkTree<KQ, M, Alloc>
where
    KQ: KeyQuery<Key = K> + Default,
    M: Metric<Dist, <KQ as KeyQuery>::Query>,
    N: BkNode<Key = K, Dist = Dist>,
    Alloc: NodeAllocator<Node = N>,
{
    pub fn new_with_allocator(metric: M, alloc: Alloc) -> Self {
        BkTree {
            root: None,
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
        match self.root {
            None => {
                let child = self.node_allocator.new(self.kq.to_key(query));
                self.root = Some(child);
            }
            Some(ref mut root) => {
                let mut cur = root;
                let mut dist = self.kq.distance(&self.metric, cur.key(), query);
                while cur.has_child_at(dist) && (dist == Dist(0) || !self.kq.eq(cur.key(), query)) {
                    cur = cur.child_at_mut(dist).unwrap();
                    dist = self.kq.distance(&self.metric, cur.key(), query);
                }
                assert!(!cur.has_child_at(dist) || self.kq.eq(cur.key(), query));
                if !self.kq.eq(cur.key(), query) {
                    let child = self.node_allocator.new(self.kq.to_key(query));
                    cur.set_child_node(dist, child);
                }
            }
        }
    }
}
