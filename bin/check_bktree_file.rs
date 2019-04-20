extern crate bkchainsaw;

use memmap::Mmap;
use memmap::MmapOptions;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Result as IOResult;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom};

use bkchainsaw::bkfile;

fn main() -> Result<(), Box<dyn Error + 'static>> {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);
    let mut treefile = File::open(args[1].clone())?;
    bkfile::Header::read(&mut treefile, true)?;

    Ok(())
}
