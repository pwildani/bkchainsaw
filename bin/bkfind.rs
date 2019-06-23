extern crate bkchainsaw;
extern crate chrono;

use std::boxed::Box;
use std::cell::RefCell;
use std::clone::Clone;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io::Cursor;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::rc::Rc;

use byteorder::{ByteOrder, LittleEndian};
//use typed_arena::Arena;

use bkchainsaw::array_storage::{FNode, FixedKeysConfig, InStorageNode};
use bkchainsaw::bkfile::open_mmap;
use bkchainsaw::bkfile::{FileDescrHeader, FileSection, Header};
use bkchainsaw::bknode::BkNode;
use bkchainsaw::bktree::BkTree;
use bkchainsaw::extensible_mmap::ExtensibleMmapMut;
use bkchainsaw::metric::Metric;
use bkchainsaw::Dist;
use bkchainsaw::HammingMetric;

use sha2::Digest;
use structopt::StructOpt;

#[derive(Debug, Default, StructOpt)]
#[structopt(name = "bkfile_from_ints", about = "Build a bkfile")]
struct CommandLineArgs {
    #[structopt(parse(from_os_str))]
    input_filename: PathBuf,

    #[structopt(parse(from_os_str))]
    output_filename: PathBuf,

    #[structopt(
        name = "preserve_intermediates",
        help = "Keep the intermediate files around for debugging"
    )]
    preserve_intermediates: bool,
}

// F64BNode8 uses 8 bytes per key
const KEY_SIZE: u64 = 8;

#[derive(Clone)]
struct FromFileAllocator<'f> {
    config: FixedKeysConfig,

    dist: Rc<RefCell<&'f [u8]>>,
    child_index: Rc<RefCell<&'f [u8]>>,
    num_children: Rc<RefCell<&'f [u8]>>,
    keys: Rc<RefCell<&'f [u8]>>,
    // phantom: PhantomData<K>,
}

trait BuildableFrom<T: ?Sized> {
    fn build(raw_data: &T) -> Self;
}

impl BuildableFrom<[u8]> for u64 {
    fn build(raw_data: &[u8]) -> Self {
        LittleEndian::read_u64(raw_data)
    }
}

impl<'f> FromFileAllocator<'f> {
    fn fnode(&self, index: u64) -> FNode<&'f [u8]> {
        FNode {
            config: self.config,
            index: index as usize,
            child_index: Rc::clone(&self.child_index),
            num_children: Rc::clone(&self.num_children),
            dist: Rc::clone(&self.dist),
            key: Rc::clone(&self.keys),
        }
    }
}

struct BkTreeFromFile<'f> {
    file_descr: FileDescrHeader,
    config: FixedKeysConfig,
    alloc: FromFileAllocator<'f>,
}

impl<'f> BkTreeFromFile<'f> {
    fn start_query_hamming_u64<'q>(&'q self) -> Queryable<'f, 'q, u64, HammingMetric<u64>> {
        let depth = self.file_descr.max_depth;
        let fnode: FNode<&'f [u8]> = self.alloc.fnode(0);
        let key = fnode.with_key_bytes(u64::build);
        let stash: NodeStash<'f, 'q, u64> = NodeStash {
            stash: Default::default(),
        };
        let mut query = Queryable {
            max_depth_hint: self.file_descr.max_depth as usize,
            config: self.config.clone(),
            alloc: self.alloc.clone(),
            stash: stash,
            //arena: Arena::with_capacity(depth as usize),
            root: None,
            phantom: Default::default(),
        };
        let root = match key {
            None => None,
            Some(k) => Some(QueryNode {
                key: k,
                stash: query.stash.clone(),
                alloc: query.alloc.clone(),
                fnode,
            }),
        };
        query.root = root;
        return query;
    }
}

struct QueryNode<'f, 'q:'f, K> {
    key: K,
    alloc: FromFileAllocator<'f>,
    stash: NodeStash<'f, 'q, K>,
    fnode: FNode<&'f [u8]>,
}

struct NodeStash<'f, 'q: 'f, K> {
    stash: Rc<RefCell<HashMap<usize, Box<QueryNode<'f, 'q, K>>>>>,
}

impl<'f, 'q: 'f, K> Clone for NodeStash<'f, 'q, K> {
    fn clone(&self) -> Self {
        NodeStash {
            stash: self.stash.clone(),
        }
    }
}

impl<'f, 'q: 'f, K:Clone> NodeStash<'f, 'q, K> {
    pub fn stash(
        &self,
        index: usize,
        key: &K,
        alloc: &FromFileAllocator<'f>,
        fnode: FNode<&'f [u8]>,
        ) -> &Box<QueryNode<'f, 'q, K>> {
        /*
        *self.stash.borrow_mut().entry(index).or_insert_with(|| Box::new(
                QueryNode{
                    key: key.clone(),
                    stash: self.clone(),
                    alloc: alloc.clone(),
                    fnode: fnode.clone(),
                }))
        */
        {
            let mut stash = self.stash.borrow_mut();
            if ! stash.contains_key(&index) {
                stash.insert(index, Box::new(
                        QueryNode{
                            key: key.clone(),
                            stash: self.clone(),
                            alloc: alloc.clone(),
                            fnode: fnode.clone(),
                        }));
            }
        }
        return self.stash.borrow().get(&index).unwrap();
    }
}

struct Queryable<'f, 'q, K, M: Metric> {
max_depth_hint: usize,
config: FixedKeysConfig,
alloc: FromFileAllocator<'f>,
//arena: Arena<QueryNode<'f, 'q, K>>,
stash: NodeStash<'f, 'q, K>,
    root: Option<QueryNode<'f, 'q, K>>,
    phantom: PhantomData<M>,
}

impl<'f, 'q: 'f, K: 'q, M: Metric> BkTree<'f, K> for Queryable<'f, 'q, K, M>
where
    K: Clone,
    K: BuildableFrom<[u8]>,
{
    type Metric = M;
    type Node = QueryNode<'f, 'q, K>;

    fn root(&self) -> &Option<Self::Node>
    where
        K: BuildableFrom<[u8]>,
    {
        &self.root
    }

    fn max_depth_hint(&self) -> usize {
        self.max_depth_hint
    }
}

impl<'f, 'q:'f, K: BuildableFrom<[u8]> + Clone> BkNode for QueryNode<'f, 'q, K> {
    type Key = K;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn has_child_at(&self, dist: Dist) -> bool  {
        if let Some(base) = self.fnode.children_offset() {
            for i in 0..self.fnode.child_count().unwrap_or(0) as u64 {
                let child = self.alloc.fnode(base as u64 + i);
                if let Some(d) = child.distance() {
                    if d == dist {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    fn child_at(&self, dist: Dist) -> Option<&Self> {
        let base = self.fnode.children_offset()?;
        for i in 0..self.fnode.child_count()? {
            let index = base+i;
            let child = self.alloc.fnode(index as u64);
            if let Some(child_dist) = child.distance() {
                if child_dist == dist {
                    if let Some(key) = child.with_key_bytes(Self::Key::build) {
                        return Some(&self.stash.stash(index, &key, &self.alloc, child));
                    }
                }
            }
        }
        None
    }

    fn children_vector(&self) -> Vec<(Dist, &Self)> {
        let mut out: Vec<(Dist, &Self)> = vec![];
        if let (Some(base), Some(child_count)) =
            (self.fnode.children_offset(), self.fnode.child_count())
        {
            for i in 0..child_count {
                let index = base + i;
                let child = self.alloc.fnode(index as u64);
                if let Some(dist) = child.distance() {
                    if let Some(key) = child.with_key_bytes(Self::Key::build) {
                        out.push((dist as Dist, &self.stash.stash(index, &key, &self.alloc, child)));
                    }
                }
            }
        }
        return out;
    }
}

fn main() -> Result<(), Box<dyn Error + 'static>> {
    let opts = CommandLineArgs::from_args();
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);

    let treefile = open_mmap(&opts.input_filename)?;
    let mut header_cursor = Cursor::new(&treefile);
    let header = Header::read(&mut header_cursor)?;

    // only handle u64 keys for now
    // TODO: 128bits and variable sized
    assert_eq!(
        header
            .descr
            .section
            .key
            .ok_or("missing key section")?
            .item_size,
        Some(8)
    );

    let child_index = header
        .descr
        .section
        .child_index
        .ok_or("missing index section")?;
    let num_children = header
        .descr
        .section
        .num_children
        .ok_or("missing child count section")?;
    let dist = header.descr.section.dist.ok_or("missing dist section")?;
    let keys = header.descr.section.key.ok_or("missing key section")?;

    let config: FixedKeysConfig = FixedKeysConfig {
        child_index: child_index.item_size.ok_or("undefined child size")? as usize,
        num_children: num_children.item_size.ok_or("undefined child count size")? as usize,
        dist: dist.item_size.ok_or("undefined dist size")? as usize,
        key: keys.item_size.ok_or("undefined key size")? as usize,
    };

    let file_chunk = |chunk: FileSection| -> Rc<RefCell<&[u8]>> {
        let start = header.end + chunk.offset;
        let end = start + chunk.bytes;
        Rc::new(RefCell::new(&treefile[start as usize..end as usize]))
    };

    let alloc: FromFileAllocator = FromFileAllocator {
        config,
        dist: file_chunk(dist),
        child_index: file_chunk(child_index),
        num_children: file_chunk(num_children),
        keys: file_chunk(keys),
    };

    let mut treestorage = BkTreeFromFile {
        file_descr: header.descr,
        config,
        alloc,
    };
    let mut tree = treestorage.start_query_hamming_u64();

    Ok(())
}
