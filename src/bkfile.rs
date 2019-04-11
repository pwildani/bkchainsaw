/**
 * File format: (Version \n\0\0\1)
 *   Magic number:
 *     "BKTree: " + "0000\n"
 *   Checksum: "SHA256: " + hex sha-256 of the remainder of the file following this newline + "\n---\n"
 *   CBOR encoded header as a map:
 *       "Created-On: " + ISO-8601 timestamp
 *       "Node-Format": "8 bits distance, 8 bits child\n"
 *       "Node-Bytes": integer, node storage size
 *       "Node-Offset": integer, byte offset after the end of the header where nodes start
 *           Should be "0\n"
 *       "Node-Count": optional, integer, number of nodes
 *       "Key-Format": "fixed 64 bits\n" (future work: "variable length\n")
 *       "Key-Offset": integer, byte offset after header where keys start
 *       "Key-Bytes": integer, key storage size
 *       "Padding:": optional if lucky, '.' repeated (0 to 63 times) until the byte after the end
 *           of header marker is 64-byte aligned from the start of the file.
 *
 *   Binary data: Offsets in the header start counting from here. The first byte of the node array.
 *   is at offset 0.
 *      * node array
 *      * 0 padding to next 64-byte-aligned position from the start of the file.
 *      * key array
 */

use memmap::MmapOptions;
use memmap::Mmap;
use std::io::Result as IOResult;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom};
use std::fs::File;
use std::error::Error;
use sha2::{Sha256, Digest};
use std::io;
use std::error;

fn open_mmap(filename: &str, offset: usize, length: usize) -> IOResult<Mmap> {
    let file = File::open(filename)?;
    // let mmap = unsafe { MmapOptions::new().map(&file)? };
    let mmap = unsafe { Mmap::map(&file)?  };
    Ok(mmap)
}

trait TrimStart {
    type Elt;
    fn trim_start_matches(&self, val: Self::Elt) -> Self;
}

impl TrimStart for String {
    type Elt = char;
    fn trim_start_matches(&self, val: char) -> Self {
        let mut chars = self.chars();
        while let Some(c) = chars.next() {
            if c != val {
                return chars.collect::<String>();
            }
        }
        return "".to_string();
    }
}

impl TrimStart for Vec<u8> {
    type Elt = u8;
    fn trim_start_matches(&self, val: u8) -> Self {
        let mut i = 0;
        while self[i] == val {
            i += 1;
        }
        return (self)[i..].to_vec();
    }
}


#[derive(Debug, Default, Deserialize, Serialize)]
struct FileDescrHeader {
    #[serde(rename="Created-On")]
    created_on: String,
    
    #[serde(rename="Node-Format")]
    node_format: String,
    #[serde(rename="Node-Bytes")]
    node_bytes: String,
    #[serde(rename="Node-Offset")]
    node_offset: u64,
    #[serde(rename="Node-Count")]
    node_count: u64,

    #[serde(rename="Key-Format")]
    key_format: String,
    #[serde(rename="Key-Offset")]
    key_offset: u64,
    #[serde(rename="Key-Bytes")]
    key_bytes: u64,

    #[serde(rename="Padding")]
    padding: String,
}


impl FileDescrHeader {
    fn encode(&mut self, offset: usize)  -> Vec<u8> {
        // Ensure 64 byte alignment
        const ALIGNMENT: usize = 64;
        self.padding = "".to_string();
        let mut buffer = serde_cbor::to_vec(&self).unwrap();
        let padding = ALIGNMENT - (offset + buffer.len()) % ALIGNMENT;
        self.padding = ".".repeat(padding);
        buffer = serde_cbor::to_vec(&self).unwrap();
        assert_eq!(0, (offset + buffer.len()) % ALIGNMENT);
        return buffer
    }
}

#[derive(Debug, Default)]
pub struct Header {
    version: Vec<u8>,
    checksum: Vec<u8>,
    descr: FileDescrHeader,
}


impl Header {
    pub fn read(file: &mut File, verify_checksum: bool) -> Result<Header, Box<dyn error::Error + 'static>> {
        let mut header: Header = Default::default();
        let mut reader = BufReader::new(file);

        // Check the magic number
        reader.read_until('\n' as u8, &mut header.version)?;
        if header.version != "BKTREE: 0000".as_bytes() {
            return Err("Unknown file format (expected \"BKTREE: 0000\")".into());
        }

        // Read the checksum
        let mut checksum_type: Vec<u8> = Vec::new();
        reader.read_until(':' as u8, &mut checksum_type)?;
        if checksum_type != "SHA256".as_bytes() {
            return Err("Unknown checksum format (expected \"SHA256\")".into());
        }
        let mut checksum : Vec<u8> = Vec::new();
        reader.read_until('\n' as u8, &mut checksum);
        header.checksum = checksum.trim_start_matches(' ' as u8);
        

        let descr_start = reader.seek(SeekFrom::Current(0))?;
        if verify_checksum {
            let mut hasher = Sha256::new();
            let n = io::copy(&mut reader, &mut hasher)?;
            let found = format!("{:x}", hasher.result());
            if found.as_bytes() != header.checksum.as_slice() {
                return Err(format!("Checksum failure. Found {:?}, expected {:?}", found, header.checksum).into());
            }
        }
        reader.seek(SeekFrom::Start(descr_start))?;

        return Ok(header);

    }
}


