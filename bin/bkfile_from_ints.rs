extern crate bkchainsaw;
extern crate chrono;

use std::boxed::Box;
use std::cell::RefCell;
use std::cmp::max;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Result as IoResult;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::rc::Rc;

use bkchainsaw::array_storage::F64BNode8;
use bkchainsaw::array_storage::{FNode, FixedKeysConfig};
use bkchainsaw::array_storage::{InStorageNode, InStorageNodeMut};
use bkchainsaw::bk;
use bkchainsaw::bkfile;
use bkchainsaw::bkfile::{FileSection, FileSections};
use bkchainsaw::bknode::BkNode;
use bkchainsaw::bktree;
use bkchainsaw::bktree::BkTreeAdd;
use bkchainsaw::bktreemut;
use bkchainsaw::keys;
use bkchainsaw::HammingMetric;
use bkchainsaw::Dist;

use bkchainsaw::extensible_mmap::ExtensibleMmapMut;

#[macro_use]
extern crate structopt;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
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
    config: FixedKeysConfig,
    index: u64,

    dist: Rc<RefCell<ExtensibleMmapMut>>,
    child_index: Rc<RefCell<ExtensibleMmapMut>>,
    num_children: Rc<RefCell<ExtensibleMmapMut>>,
    keys: Rc<RefCell<ExtensibleMmapMut>>,
}

impl InFileAllocator {
    fn alloc(&mut self, n: u64) -> Result<u64, Box<dyn Error>> {
        self.child_index.borrow_mut().alloc_bytes(self.config.child_index * n as usize)?;
        self.dist.borrow_mut().alloc_bytes(self.config.dist * n as usize)?;
        self.num_children.borrow_mut().alloc_bytes(self.config.num_children * n as usize)?;
        self.keys.borrow_mut().alloc_bytes(self.config.key * n as usize)?;
        let start = self.index;
        self.index += n;
        return Ok(start);
    }

    fn fnode<'s>(&'s self, index: u64) -> FNode<'s> {
        FNode {
            config: &self.config,
            index: index as usize,
            child_index: Rc::clone(&self.child_index),
            num_children: Rc::clone(&self.num_children),
            dist: Rc::clone(&self.dist),
            key: Rc::clone(&self.keys),
        }
    }
}

fn walk<'a, 'c, 'n>(
    config: &'c FixedKeysConfig,
    alloc: &'a mut InFileAllocator,
    index: u64,
    dist: Dist,
    node: &'n bk::BkInRam<u64>,
) -> Result<(), Box<dyn Error>> {
    let children = node.children_vector();
    // A BkFile is a pre-order representation. We have to allocate space for all of this
    // node's children contiguously and earlier in the file than any of the grandchildren.
    // F64Node8 can compute where to put its key.
    // Future work: for variable sized keys, the key offset calculated here needs to be
    // passed forward.
    
    let child_start_index = if children.len() == 0 { 0 } else { alloc.alloc(children.len() as u64)?};
    {
        // This should be safe because the space for this node was allocated in the previous
        // call.
        let mut mirror = alloc.fnode(index);
        mirror.set_key(node.key)?;
        mirror.set_dist(dist)?;
        mirror.set_num_children(children.len())?;
        mirror.set_child_index(child_start_index as usize)?;
    }

    for (i, (dist, child)) in children.iter().rev().enumerate() {
        let index = child_start_index + i as u64;
        walk(config, alloc, index, *dist, child)?;
    }
    Ok(())
}


fn byte_size_for_max_val(i: u64) -> usize {
    if i < 2<<8 {
        return 1;
    } else if i < 2<<16 {
        return 2;
    } else if i < 2<<24 {
        return 3;
    } else if i < 2<<32 {
        return 4;
    } else if i < 2<<40 {
        return 5;
    } else if i < 2<<48 {
        return 6;
    } else if i < 2<<56 {
        return 7;
    } else {
        return 8;
    }
}

fn align(alignment: u64, value: u64) -> u64 {
    return value + alignment - value % alignment;
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
    let keystemp = tempfile::tempfile()?;
    keystemp.set_len(tree.node_count * KEY_SIZE)?;
    let disttemp = tempfile::tempfile()?;
    disttemp.set_len(tree.node_count * 1)?;
    let child_index_temp = tempfile::tempfile()?;
    child_index_temp.set_len(tree.node_count * byte_size_for_max_val(tree.node_count) as u64)?;
    let num_children_temp = tempfile::tempfile()?;
    num_children_temp.set_len(tree.node_count * byte_size_for_max_val(tree.node_count) as u64)?;

    let child_index_size = byte_size_for_max_val(tree.node_count);
    // TODO: measure max node width ~= max dist
    let num_children_size = 1;
    // TODO: measure max distance
    let dist_size = byte_size_for_max_val(64);
    // TODO: handle variable key size
    let key_size = 8;
    let config = FixedKeysConfig {
        child_index: child_index_size,
        num_children: num_children_size,
        dist: dist_size,
        key: 8,
    };
    fn rref<T>(val: T) -> Rc<RefCell<T>> {
        Rc::new(RefCell::new(val))
    }

    let mut alloc = InFileAllocator {
        config,
        dist: rref(ExtensibleMmapMut::on(disttemp)?),
        child_index: rref(ExtensibleMmapMut::on(child_index_temp)?),
        num_children: rref(ExtensibleMmapMut::on(num_children_temp)?),
        keys: rref(ExtensibleMmapMut::on(keystemp)?),
        index: 0,
    };
    if let Some(ref node) = tree.root {
        alloc.alloc(1)?;
        walk(&config, &mut alloc, 0, 0, &node)?;
    }

    println!(
        "keys bytes: {} / {}",
        alloc.keys.borrow().len(),
        alloc.keys.borrow().capacity()
    );

    let mut sections :Vec<(u64, Rc<RefCell<ExtensibleMmapMut>>)> = Vec::new();

    // Step 3: build the header (key offset = nodes.lengths
    let mut descr: bkfile::FileDescrHeader = Default::default();
    descr.created_on = Utc::now().to_rfc3339();
    descr.node_count = tree.node_count;

    let mut offset = 0;
    let mut add_section = |dest: &mut Option<FileSection>, item_size: usize, bytes: Rc<RefCell<ExtensibleMmapMut>>| {
        let len = bytes.borrow().len() as u64;
        dest.replace(FileSection {
            offset,
            bytes: len,
            item_size: if item_size > 0 { Some(item_size as u64) } else { None },
        });
        sections.push((offset, bytes));
        offset = align(64, offset + len);
        assert_eq!(offset % 64, 0);
    };

    add_section(&mut descr.section.dist, config.dist, alloc.dist);
    add_section(&mut descr.section.child_index, config.child_index, alloc.child_index);
    add_section(&mut descr.section.num_children, config.num_children, alloc.num_children);
    add_section(&mut descr.section.key, config.key, alloc.keys);

    let header = descr.encode(bkfile::PREFIX_SIZE);
    println!("{:#?}", descr);

    // Step 4: Checksum: header + sections
    let mut hasher = Sha256::new();
    hasher.write(&header)?;
    let mut pos = 0;
    for (offset, ram) in sections.iter() {
        for _ in pos .. *offset {
            hasher.write(&[0u8])?;
            pos += 1;
        }
        assert_eq!(pos, *offset);
        hasher.write(ram.borrow_mut().ram())?;
        pos += ram.borrow().len() as u64;
    }

    // Step 5: write it out
    let mut out = BufWriter::new(File::create(opts.output_filename)?);
    write!(&mut out, "{}\n", bkfile::MAGIC_VERSION)?;
    write!(
        &mut out,
        "{}: {:064x}\n",
        bkfile::HASH_HEADER_NAME,
        hasher.result()
    )?;
    assert_eq!(
        bkfile::PREFIX_SIZE,
        out.seek(SeekFrom::Current(0))? as usize
    );
    io::copy(&mut header.as_slice(), &mut out)?;
    let mut pos = 0;
    for (offset, ref mut ram) in sections {
        for _ in pos .. offset {
            out.write(&[0u8])?;
            pos += 1;
        }
        assert_eq!(pos, offset);
        io::copy(&mut ram.borrow_mut().ram_mut().as_ref(), &mut out)?;
        pos += ram.borrow().len() as u64;
    }
    out.flush()?;

    Ok(())
}
