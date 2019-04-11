extern crate bkchainsaw;

use memmap::MmapOptions;
use memmap::Mmap;
use std::io::Result as IOResult;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom};
use std::fs::File;
use std::error::Error;
use sha2::{Sha256, Digest};
use std::io;
use std::env;

use bkchainsaw::bkfile;


fn main() -> Result<(), Box<dyn Error + 'static>> {
	let args: Vec<String> = env::args().collect();
	println!("args: {:?}", args);
	let mut treefile = File::open(args[1].clone())?;
	bkfile::Header::read(&mut treefile, true)?;

    Ok(())
}
