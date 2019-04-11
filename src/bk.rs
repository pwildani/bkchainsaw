
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

