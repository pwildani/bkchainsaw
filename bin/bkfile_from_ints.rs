extern crate bkchainsaw;

use std::boxed::Box;
use std::cell::RefCell;
use std::cmp::max;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Result as IoResult;
use std::io::Write as IOWrite;
use std::io::{BufRead, BufReader, BufWriter};
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::rc::Rc;

use bkchainsaw::array_storage::F64BNode8;
use bkchainsaw::array_storage::{InStorageNode, InStorageNodeMut};
use bkchainsaw::bk;
use bkchainsaw::bkfile;
use bkchainsaw::bknode::BkNode;
use bkchainsaw::bktree;
use bkchainsaw::bktree::BkTreeAdd;
use bkchainsaw::bktreemut;
use bkchainsaw::keys;
use bkchainsaw::HammingMetric;

use bkchainsaw::extensible_mmap::ExtensibleMmapMut;

#[macro_use]
extern crate structopt;

use structopt::StructOpt;
use tempfile;

use memmap::MmapMut;
use memmap::MmapOptions;

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

// TODO: handle more file types than fixed u64 keys with <256 distances and children
// F64BNode8 uses 8 bytes per node
const NODE_SIZE: u64 = 8;

// F64BNode8 uses 8 bytes per key
const KEY_SIZE: u64 = 8;

struct InFileAllocator {
    nodes: ExtensibleMmapMut,
    keys: ExtensibleMmapMut,
}

fn walk(
    alloc: &mut InFileAllocator,
    offset: usize,
    dist: usize,
    node: &bk::BkInRam<u64>,
) -> Result<(), Box<dyn Error>> {
    let children = node.children_vector();
    // A BkFile is a pre-order representation. We have to allocate space for all of this
    // node's children contiguously and earlier in the file than any of the grandchildren.
    let (child_offset, _) = alloc
        .nodes
        .alloc_bytes(NODE_SIZE as usize * children.len())?;
    // F64Node8 can compute where to put its key.
    // Future work: for variable sized keys, the key offset calculated here needs to be
    // passed forward.
    let (_, _) = alloc.keys.alloc_bytes(KEY_SIZE as usize)?;
    // this block should be a  fn render_at ... , but that requires building the children vector twice.
    {
        // This should be safe because the space for this node was allocated in the previous
        // call.
        let mut mirror = F64BNode8 {
            offset,
            key_buffer: RefCell::new(alloc.keys.ram_mut()),
            node_buffer: RefCell::new(alloc.nodes.ram_mut()),
        };
        // println!("{} == {}", offset, node.key);
        mirror.set_key(node.key)?;
        mirror.set_dist(dist)?;
        mirror.set_num_children(children.len())?;
        mirror.set_child_offset(child_offset)?;
    }

    for (i, (dist, child)) in children.iter().enumerate() {
        let offset = child_offset + NODE_SIZE as usize * i;
        walk(alloc, offset, *dist, child)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error + 'static>> {
    let opts = CommandLineArgs::from_args();
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);

    // Step 1: build the tree in RAM
    let mut tree: bk::BkInRamTree<
        '_,
        keys::U64Key,
        HammingMetric<u64>,
        bk::BkInRamAllocator<'_, u64>,
    > = bk::BkInRamTree::new(HammingMetric::default(), &bk::U64_ALLOC);
    let numbers = BufReader::new(File::open(args[1].clone())?).lines();
    for numstr in numbers {
        let num: u64 = numstr?.parse()?;
        tree.add(num)?;
    }

    // Step 2: Render the ndoes into bytes.
    // TODO preserve temps for debugging;
    let nodestemp = tempfile::tempfile()?;
    let keystemp = tempfile::tempfile()?;
    nodestemp.set_len(tree.node_count * NODE_SIZE)?;
    keystemp.set_len(tree.node_count * KEY_SIZE)?;

    let mut alloc = InFileAllocator {
        nodes: ExtensibleMmapMut::on(nodestemp)?,
        keys: ExtensibleMmapMut::on(keystemp)?,
    };
    if let Some(ref node) = tree.root {
        alloc.nodes.alloc_bytes(NODE_SIZE as usize)?;
        alloc.keys.alloc_bytes(KEY_SIZE as usize)?;
        walk(&mut alloc, 0, 0, &node)?;
    }

    println!("nodes bytes: {} / {}", alloc.nodes.len(), alloc.nodes.capacity());
    println!("keys bytes: {} / {}", alloc.keys.len(), alloc.keys.capacity());

    // Step 3: build the header (key offset = nodes.lengths
    // Step 4: Checksum: header + nodes + bytes
    // Step 5: write it out

    Ok(())
}
