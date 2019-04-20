use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::option::Option;
use std::vec::Vec;

use crate::bknode::{BkNode, BkNodeMut};
use crate::bktree::{BkTree, BkTreeAdd, BkTreeRootMut};
use crate::keyquery::KeyQuery;
use crate::metric::Metric;

use crate::nodeallocator::NodeAllocator;
use crate::Dist;

/// BK tree node optimised for small distances.
/// TODO: consider feature(const_generics) to drop the vec overhead, once that's stable.
/// (https://github.com/rust-lang/rust/issues/44580)
pub struct BkInRam<K> {
    pub key: K,
    children: Vec<Option<Self>>,
}

impl<K> BkInRam<K> {
    pub fn new(key: K) -> BkInRam<K> {
        BkInRam {
            key: key,
            children: Vec::with_capacity(16),
        }
    }

    pub fn children_iter(&self) -> impl Iterator<Item = (Dist, &Self)> {
        self.children
            .iter()
            // This implementation stores the distance to the child implicitly as the index into
            // the child vector.
            .enumerate()
            .filter(|(_, child)| child.is_some())
            .map(|(dist, child)| (dist.into(), child.as_ref().unwrap()))
            .rev() // Find here looks at the last child first, and things play nicer if the closest is first.
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

    fn child_at(&self, dist: Dist) -> Option<&Self> {
        match self.children.get(dist) {
            None | Some(None) => None,
            Some(child @ Some(_)) => child.as_ref(),
        }
    }

    fn children_vector(&self) -> Vec<(Dist, &Self)> {
        self.children_iter().collect()
    }
}

impl<'a, K> BkNodeMut for BkInRam<K> {
    fn child_at_mut(&mut self, dist: Dist) -> Option<&mut Self> {
        match self.children.get_mut(dist) {
            None | Some(None) => None,
            Some(child @ Some(_)) => child.as_mut(),
        }
    }

    fn set_child_node(&mut self, dist: Dist, node: Self) {
        if self.children.len() <= dist {
            self.children.resize_with(dist + 1, || None);
        }
        assert!(!self.has_child_at(dist));
        self.children[dist] = Some(node);
    }
}

impl<K> Debug for BkInRam<K>
where
    K: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let children: Vec<_> = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, x)| x.is_some())
            .collect();
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

    // Can't error.
    //type AllocationError = Box<dyn Error>;

    fn new_root(&'a self, key: K) -> Result<Self::Node, Box<dyn Error>> {
        Ok(BkInRam::new(key))
    }

    fn new_child(&'a self, key: K) -> Result<Self::Node, Box<dyn Error>> {
        Ok(BkInRam::new(key))
    }
}

pub const U64_ALLOC: BkInRamAllocator<'static, u64> = BkInRamAllocator(PhantomData);
pub const STRING_ALLOC: BkInRamAllocator<'static, String> = BkInRamAllocator(PhantomData);

pub struct BkInRamTree<'nodes, KQ, M, A>
where
    KQ: KeyQuery,
    M: Metric<<KQ as KeyQuery>::Query>,
    A: 'nodes + NodeAllocator<'nodes, Node = BkInRam<<KQ as KeyQuery>::Key>>,
{
    pub root: Option<A::Node>,
    pub max_depth: usize,
    pub node_count: u64,
    metric: M,
    node_allocator: &'nodes A,
    kq: KQ,
}

impl<'nodes, K, KQ, M, A> Debug for BkInRamTree<'nodes, KQ, M, A>
where
    K: Debug + Clone,
    KQ: KeyQuery<Key = K>,
    M: Metric<<KQ as KeyQuery>::Query>,
    A: 'nodes + NodeAllocator<'nodes, Node = BkInRam<<KQ as KeyQuery>::Key>>,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("BkInRamTree")
            .field("node_count", &self.node_count)
            .field("max_depth", &self.max_depth)
            .field("root", &self.root)
            .finish()
    }
}

impl<'nodes, K, KQ, M, Alloc> BkInRamTree<'nodes, KQ, M, Alloc>
where
    K: Clone,
    KQ: KeyQuery<Key = K> + Default,
    M: Metric<<KQ as KeyQuery>::Query>,
    Alloc: 'nodes + NodeAllocator<'nodes, Node = BkInRam<K>>,
{
    pub fn new(metric: M, alloc: &'nodes Alloc) -> Self {
        BkInRamTree {
            root: None,
            max_depth: 0,
            node_count: 0,
            metric: metric,
            node_allocator: alloc,
            kq: Default::default(),
        }
    }
}

impl<'nodes, Q, K, KQ, M, Alloc> BkTreeRootMut<'nodes, K> for BkInRamTree<'nodes, KQ, M, Alloc>
where
    K: Clone,
    Q: Sized,
    KQ: KeyQuery<Key = K, Query = Q> + Default,
    M: Metric<<KQ as KeyQuery>::Query>,
    Alloc: 'nodes + NodeAllocator<'nodes, Node = BkInRam<K>>,
{
    type Alloc = Alloc;

    fn node_allocator(&mut self) -> &'nodes Self::Alloc {
        &self.node_allocator
    }

    fn root_mut(&mut self) -> &mut Option<<Self as BkTree<K>>::Node> {
        &mut self.root
    }

    fn max_depth_mut(&mut self) -> &mut usize {
        &mut self.max_depth
    }

    fn incr_node_count(&mut self) {
        self.node_count += 1;
    }
}

impl<'nodes, K, KQ, M, A, Q> BkTree<K> for BkInRamTree<'nodes, KQ, M, A>
where
    K: Clone,
    Q: Sized,
    KQ: KeyQuery<Key = K, Query = Q>,
    M: Metric<Q>,
    A: 'nodes + NodeAllocator<'nodes, Node = BkInRam<K>>,
{
    type KQ = KQ;
    type Metric = M;
    type Node = <A as NodeAllocator<'nodes>>::Node;

    fn find_each<'a, F>(
        &'a self,
        needle: &'a <Self::KQ as KeyQuery>::Query,
        tolerance: Dist,
        callback: F,
    ) where
        F: FnMut(Dist, &<Self::KQ as KeyQuery>::Key),
    {
        if let Some(ref root) = self.root {
            let finder = BkFind::new(self.max_depth, Some(root), tolerance, needle);
            finder.each::<KQ, M, F>(callback);
        }
    }
}

#[derive(Debug, Clone)]
struct BkFindEntry<'n, N: 'n + BkNode> {
    dist: Dist,
    node: &'n N,
}

pub struct BkFind<'q, 'n, Q: 'q, N: 'n>
where
    N: 'n + BkNode,
{
    tolerance: Dist,
    needle: &'q Q,
    root: Option<&'n N>,
    stack: Vec<BkFindEntry<'n, N>>,
}

impl<'q, 'n, Q: 'q, N: 'n> BkFind<'q, 'n, Q, N>
where
    N: 'n + BkNode,
{
    pub fn new(max_depth_hint: usize, root: Option<&'n N>, tolerance: Dist, needle: &'q Q) -> Self {
        let stack = Vec::with_capacity(max_depth_hint);
        BkFind {
            tolerance,
            needle,
            root,
            stack,
        }
    }
}

impl<'q, 'n, Q: 'q, N: 'n, K: 'n + Clone> BkFind<'q, 'n, Q, N>
where
    N: 'n + BkNode<Key = K>,
{
    pub fn each<KQ, M, F>(mut self, mut callback: F)
    where
        KQ: KeyQuery<Key = <N as BkNode>::Key, Query = Q>,
        M: Metric<Q>,
        F: FnMut(Dist, &'n <KQ as KeyQuery>::Key),
    {
        if let Some(root) = self.root.take() {
            let dist = M::distance_static(KQ::to_query_static(root.key()), self.needle);
            self.stack.push(BkFindEntry {
                dist: dist,
                node: root,
            })
        }

        while let Some(candidate) = self.stack.pop() {
            // Enqueue the children.
            let min: Dist = candidate.dist.saturating_sub(self.tolerance);
            let max: Dist = candidate.dist.saturating_add(self.tolerance);
            let children = candidate.node.children_vector();
            for (dist, child) in children.iter() {
                if min <= *dist && *dist <= max {
                    let child_dist =
                        M::distance_static(KQ::to_query_static(child.key()), self.needle);
                    self.stack.push(BkFindEntry {
                        dist: child_dist,
                        node: *child,
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
