extern crate bkchainsaw;


use serde_json;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write as IOWrite;
use std::io::{BufReader, BufWriter};
use std::io::{Seek, SeekFrom};

use bkchainsaw::bkfile;

#[macro_use]
extern crate structopt;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "bktree_from_json", about = "Build a bktree file from parts")]
struct CommandLineArgs {
    /// Activate debug mode
    #[structopt(short = "d", long = "debug")]
    debug: bool,

    #[structopt(parse(from_os_str))]
    header_filename: PathBuf,

    #[structopt(parse(from_os_str))]
    nodes_filename: PathBuf,

    #[structopt(parse(from_os_str))]
    keys_filename: PathBuf,

    /// Output file, stdout if not present
    #[structopt(parse(from_os_str))]
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn Error + 'static>> {
    let opts = CommandLineArgs::from_args();
	let args: Vec<String> = env::args().collect();
    // 1: input descr json file
    // 2: checksum value
    // 3: output header + cbor descr btree file, no nodes or keys
	println!("args: {:?}", args);
	let descrfile = File::open(args[1].clone())?;
    let mut descr : bkfile::FileDescrHeader = serde_json::from_reader(BufReader::new(descrfile))?;

	let outfile = File::create(args[3].clone())?;
    let mut out = BufWriter::new(outfile);
    writeln!(out, "{}", bkfile::MAGIC_VERSION)?;
    writeln!(out, "{}: {}", bkfile::HASH_HEADER_NAME, args[2])?;

    let pos = out.seek(SeekFrom::Current(0))?;
    let descr_bytes = descr.encode(pos as usize);
    out.write_all(&descr_bytes[..])?;
    out.flush()?;

    Ok(())
}


