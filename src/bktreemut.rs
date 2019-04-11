
impl<'a, K: 'a + Clone, KQ, M, N, Alloc, Tree> BkTreeMut<KQ, M>
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
