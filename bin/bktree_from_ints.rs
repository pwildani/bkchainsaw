extern crate bkchainsaw;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write as IOWrite;
use std::io::{BufRead, BufReader, BufWriter};
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use structopt::StructOpt;

use bkchainsaw::bk;
use bkchainsaw::bkfile;
use bkchainsaw::bktree;
use bkchainsaw::bktree::BkTreeAdd;
use bkchainsaw::bktreemut;
use bkchainsaw::keys;
use bkchainsaw::HammingMetric;

#[macro_use]
extern crate structopt;

#[derive(Debug, StructOpt)]
#[structopt(name = "bktree_from_ints", about = "Build an in-ram bktree")]
struct CommandLineArgs {
    #[structopt(parse(from_os_str))]
    input_filename: PathBuf,
}

fn main() -> Result<(), Box<dyn Error + 'static>> {
    let opts = CommandLineArgs::from_args();
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);
    // 1: input numbers
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
    println!("{:?}", tree);

    Ok(())
}
