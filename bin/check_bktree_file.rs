extern crate bkchainsaw;

use std::env;
use std::error::Error;
use std::fs::File;

use bkchainsaw::bkfile;

fn main() -> Result<(), Box<dyn Error + 'static>> {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);
    let mut treefile = File::open(args[1].clone())?;
    bkfile::Header::read(&mut treefile, true)?;

    Ok(())
}
