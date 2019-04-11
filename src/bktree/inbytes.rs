use typed_arena::Arena;

use crate::array_storage::F64BNode8;
use crate::bknode::BkNode;
use crate::bktree::bktree::NodeAllocator;

use crate::Dist;


trait NodeStorage<NodeType> {
    fn node_buffer(&self) -> &[u8];
    fn key_buffer(&self) -> &[u8];
    fn render_node_at_offset(&self, offset: usize) -> &NodeType;
}

struct U64StorageFromBuffer<'a> {
    nodes: Vec<u8>,
    key: Vec<u8>,

    node_arena: Arena<F64BkNode<'a>>,
}

impl<'a> U64StorageFromBuffer<'a> {
    fn new(nodes: &'a [u8], keys: &'a [u8]) -> U64StorageFromBuffer<'a> {
        // TODO: figure out a typical query scope and pre-size the arena accordingly.
        U64StorageFromBuffer { nodes, keys, node_arena: Arena::new() };
    }
}

impl<'a> NodeStorage<F64BkNode<'a>> for U64StorageFromBuffer<'a> {
    fn node_buffer(&self) -> &[u8] { self.nodes }
    fn key_buffer(&self) -> &[u8] { self.keys }
    fn render_node_at_offset(&self, offset: usize) -> &F64BkNode<'a> {
        self.node_arena.alloc(F64BkNode::new(self, offset))
    }
}



pub struct F64BkNode<'a> {

    store: &'a U64StorageFromBuffer<'a>,
    offset: usize,

    #[cfg(target_endian = "big")]
    key_value: u64,
}

impl<'a> F64BkNode<'a> {
    fn wrapper<'b> (store: &'b U64StorageFromBuffer, offset: usize) -> F64BNode8<'b> {
        F64BNode8 {
            offset: offset,
            node_buffer: store.node_buffer(),
            key_buffer: store.key_buffer(),
        }
    }

    fn view(&self) -> F64BNode8 {
        Self::wrapper(&self.store, self.offset)
    }

    fn new(store: &'a U64StorageFromBuffer, offset: usize) -> F64BkNode<'a> {
        #[cfg(target_endian = "little")]
        return F64BkNode{ store, offset };

        #[cfg(target_endian = "big")]
        return F64BkNode{
            store,
            offset,
            key: LittleEndian::read_u64(Self::wrapper(store, offset).key_bytes().unwrap())
        };
    }

    fn node_at_offset(&self, offset: usize) -> F64BkNode {
        self.storage.render_node_at_offset(offset)
    }
}

impl<'a> BkNode for F64BkNode<'a> {
    type Key = u64;

    #[cfg(target_endian = "little")]
    fn key(&self) -> &Self::Key {
        let bytes = self.view().key_bytes().unwrap();
        let mut key: &u64 = unsafe { &*bytes.as_ptr() };
        return key;
    }

    #[cfg(target_endian = "big")]
    fn key(&self) -> &Self::Key {
        &self.key_value
    }

    fn has_child_at(&self, dist: Dist) -> bool {
        let view = self.view();
        let mut opt_child = view.first_child();
        let mut remaining = view.child_count().unwrap_or(0);
        while remaining > 0 && opt_child.is_some() {
            let child = opt_child.unwrap();
            child = child.next_node();
            if child.dist().unwrap_or(Dist::MAX) == dist {
                return true;
            }
            remaining -=1;
        }
        return false
    }

    fn child_at_mut(&mut self, dist: Dist) -> Option<&mut Self> {
        unimplemented!("No mutation yet!");
    }

    fn child_at(&self, dist: Dist) -> Option<&Self> {
        let view = self.view();
        let mut opt_child = view.first_child();
        let mut remaining = view.child_count().unwrap_or(0);
        while remaining > 0 && opt_child.is_some() {
            let child = opt_child.unwrap();
            child = child.next_node();
            if child.dist().unwrap_or(Dist::MAX) == dist {
                return self.storage.render_node_at_offset(child.offset);
            }
            remaining -=1;
        }
        return None
    }

    fn set_child_node(&mut self, dist: Dist, node: Self) {
        unimplemented!("No mutation yet!");
    }

    fn children_vector(&self) -> Vec<(Dist, &Self)> {
        let view = self.view();
        let mut child_view = view.first_child();
        let mut remaining = view.child_count().unwrap_or(0);
        let children = Vec::with_capacity(remaining);
        while remaining > 0 && child_view.is_some() {
            let child = child_view.unwrap();
            children.push((view, self.node_at_offset(child.offset)));
            child_view = child.next_node();
            remaining -= 1;
        }
        return view;
    }
}

