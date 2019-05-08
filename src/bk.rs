use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::option::Option;
use std::vec::Vec;

use crate::bknode::{BkNode, BkNodeMut};
use crate::bktree::{BkTree, BkTreeRootMut};
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
            key,
            children: Vec::with_capacity(0),
        }
    }

    pub fn children_iter(&self) -> impl Iterator<Item = (Dist, &Self)> {
        self.children
            .iter()
            // This implementation stores the distance to the child implicitly as the index into
            // the child vector.
            .enumerate()
            .filter(|(_, child)| child.is_some())
            .map(|(dist, child)| (dist, child.as_ref().unwrap()))
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
            Some(Some(ref mut child)) => Some(child),
        }
    }
    fn ref_child_at_mut(&mut self, dist: Dist) -> &mut Option<Self> {
        if self.children.len() <= dist {
            self.children.resize_with(dist + 1, || None);
        }
        &mut self.children[dist]
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

    fn new_child(&'a self, key: K) -> Result<Self::Node, Box<dyn Error>> {
        Ok(BkInRam::new(key))
    }
}

pub const U64_ALLOC: BkInRamAllocator<'static, u64> = BkInRamAllocator(PhantomData);
pub const STRING_ALLOC: BkInRamAllocator<'static, String> = BkInRamAllocator(PhantomData);

pub struct BkInRamTree<'nodes, M, A>
where
    M: Metric,
    A: 'nodes + NodeAllocator<'nodes>,
{
    pub root: Option<A::Node>,
    pub max_depth: usize,
    pub node_count: u64,
    node_allocator: &'nodes A,
    phantom: PhantomData<M>,
}

impl<'nodes, M, A, N> Debug for BkInRamTree<'nodes, M, A>
where
    M: Metric,
    N: Debug,
    A: 'nodes + NodeAllocator<'nodes, Node = N>,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("BkInRamTree")
            .field("node_count", &self.node_count)
            .field("max_depth", &self.max_depth)
            .field("root", &self.root)
            .finish()
    }
}

impl<'nodes, K, M, Alloc> BkInRamTree<'nodes, M, Alloc>
where
    K: Clone,
    M: Metric,
    Alloc: 'nodes + NodeAllocator<'nodes, Node = BkInRam<K>>,
{
    pub fn new(alloc: &'nodes Alloc) -> Self {
        BkInRamTree {
            root: None,
            max_depth: 0,
            node_count: 0,
            node_allocator: alloc,
            phantom: PhantomData {},
        }
    }
}

impl<'nodes, K, M, Alloc> BkTreeRootMut<'nodes, K> for BkInRamTree<'nodes, M, Alloc>
where
    K: 'nodes + Clone,
    M: Metric,
    Alloc: 'nodes + NodeAllocator<'nodes, Node = BkInRam<K>>,
{
    type Alloc = Alloc;

    fn node_allocator(&self) -> &'nodes Self::Alloc {
        &self.node_allocator
    }

    fn root_mut(&mut self) -> &mut Option<<Self as BkTree<'nodes, K>>::Node> {
        &mut self.root
    }

    fn max_depth_mut(&mut self) -> &mut usize {
        &mut self.max_depth
    }

    fn incr_node_count(&mut self) {
        self.node_count += 1;
    }
}

impl<'nodes, K, M, A> BkTree<'nodes, K> for BkInRamTree<'nodes, M, A>
where
    K: Clone + 'nodes,
    M: Metric,
    A: 'nodes + NodeAllocator<'nodes, Node = BkInRam<K>>,
{
    type Metric = M;
    type Node = <A as NodeAllocator<'nodes>>::Node;

    fn root(&self) -> &Option<Self::Node> {
        &self.root
    }

    fn max_depth_hint(&self) -> usize {
        self.max_depth
    }
}

#[derive(Debug, Clone)]
struct BkFindEntry<'n, N: 'n + BkNode> {
    dist: Dist,
    node: &'n N,
}
