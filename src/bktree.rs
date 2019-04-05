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
impl Into<u32> for Dist {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

pub trait NodeAllocator {
    type Key;
    type Node: BkNode;
    fn new(&mut self, key: Self::Key) -> Self::Node;
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

#[derive(Default, Derivative)]
#[derivative(Debug)]
pub struct BkInRamAllocator<K>(#[derivative(Debug = "ignore")] PhantomData<K>);

impl<K> NodeAllocator for BkInRamAllocator<K> {
    type Key = K;
    type Node = BkInRam<Self::Key>;

    fn new(&mut self, key: Self::Key) -> Self::Node {
        BkInRam::new(key)
    }
}

#[derive(Debug)]
pub struct BkTree<N: BkNode, M: Metric<Dist, N::Key>, Alloc: NodeAllocator> {
    root: Option<N>,
    metric: M,
    node_allocator: Alloc,
}

pub type BkInRamTree<K, M> = BkTree<BkInRam<K>, M, BkInRamAllocator<K>>;

impl<K: PartialEq, M: Metric<Dist, K>> BkInRamTree<K, M> {
    pub fn new(metric: M) -> Self {
        BkInRamTree::new_with_allocator(metric, BkInRamAllocator(PhantomData))
    }
}

impl<
        K: PartialEq,
        N: BkNode<Key = K, Dist = Dist>,
        M: Metric<Dist, K>,
        Alloc: NodeAllocator<Key = K, Node = N>,
    > BkTree<N, M, Alloc>
{
    pub fn new_with_allocator(metric: M, alloc: Alloc) -> Self {
        BkTree {
            root: None,
            metric: metric,
            node_allocator: alloc,
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
    pub fn add(&mut self, key: K) {
        match self.root {
            None => {
                let child = self.node_allocator.new(key);
                self.root = Some(child);
            }
            Some(ref mut root) => {
                let mut cur = root;
                let mut dist = self.metric.distance(cur.key(), &key);
                while cur.has_child_at(dist) && (dist == Dist(0) || cur.key() != &key) {
                    cur = cur.child_at_mut(dist).unwrap();
                    dist = self.metric.distance(cur.key(), &key);
                }
                assert!(!cur.has_child_at(dist) || cur.key() == &key);
                if cur.key() != &key {
                    let child = self.node_allocator.new(key);
                    cur.set_child_node(dist, child);
                }
            }
        }
    }
}
