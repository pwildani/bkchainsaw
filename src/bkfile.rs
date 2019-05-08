/**
 * File format: (Version \n\0\0\1)
 *   Magic number:
 *     "BKTree: " + "0000\n"
 *   Checksum: "SHA256: " + hex sha-256 of the remainder of the file following this newline + "\n---\n"
 *   CBOR encoded header as a map:
 *       "Created-On":  ISO-8601 timestamp
 *       "Node-Count": optional, integer, number of nodes
 *        "Sections": {
 *           "Key": Key data, Child: Child indicies, Num: child counts, Dist: distances between nodes
 *           "Key" | "Child" | "Num" | "Dist": {
 *              ItemSize: integer, byte size of a single value in this section. unset implies variable size
 *              Bytes: integer, total bytes
 *              Offset: integer, byte ofset after the end of the header where this section starts
 *           }
 *        }
 *
 *       "Padding:": optional if lucky, '.' repeated (0 to 63 times) until the byte after the end
 *           of header marker is 64-byte aligned from the start of the file.
 *
 *   Binary data: Offsets in the header start counting from here. The first byte of the node array.
 *   is at offset 0.
 *      * node array
 *      * 0 padding to next 64-byte-aligned position from the start of the file.
 *      * key array
 */
//use memmap::MmapOptions;
use memmap::Mmap;
use std::fs::File;
use std::io::Result as IOResult;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom};
//use std::error::Error;
use sha2::{Digest, Sha256};
use std::error;
use std::io;

fn open_mmap(filename: &str) -> IOResult<Mmap> {
    let file = File::open(filename)?;
    // let mmap = unsafe { MmapOptions::new().map(&file)? };
    let mmap = unsafe { Mmap::map(&file)? };
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
        "".to_string()
    }
}

impl TrimStart for Vec<u8> {
    type Elt = u8;
    fn trim_start_matches(&self, val: u8) -> Self {
        let mut i = 0;
        while self[i] == val {
            i += 1;
        }
        (self)[i..].to_vec()
    }
}

#[derive(Debug, Copy, Clone, Default, Deserialize, Serialize)]
pub struct FileSection {
    #[serde(rename = "ItemSize")]
    pub item_size: Option<u64>,
    #[serde(rename = "Bytes")]
    pub bytes: u64,
    #[serde(rename = "Offset")]
    pub offset: u64,
}

#[derive(Debug, Copy, Clone, Default, Deserialize, Serialize)]
pub struct FileSections {
    #[serde(rename = "Dist")]
    pub dist: Option<FileSection>,
    #[serde(rename = "Child")]
    pub child_index: Option<FileSection>,
    #[serde(rename = "Num")]
    pub num_children: Option<FileSection>,
    #[serde(rename = "Key")]
    pub key: Option<FileSection>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileDescrHeader {
    #[serde(rename = "Created-On")]
    pub created_on: String,

    #[serde(rename = "Node-Count")]
    pub node_count: u64,

    pub section: FileSections,

    #[serde(rename = "Padding", default)]
    padding: String,
}

impl FileDescrHeader {
    pub fn encode(&mut self, offset: usize) -> Vec<u8> {
        // Ensure 64 byte alignment for the byte following the header.
        const ALIGNMENT: usize = 64;
        self.padding = "".to_string();
        let mut buffer = serde_cbor::to_vec(&self).unwrap();
        let padding = ALIGNMENT - (offset + buffer.len() + 1) % ALIGNMENT;
        self.padding = ".".repeat(padding);
        buffer = serde_cbor::to_vec(&self).unwrap();
        assert_eq!(63, (offset + buffer.len()) % ALIGNMENT);
        buffer
    }
}

#[derive(Debug, Default)]
pub struct Header {
    version: Vec<u8>,
    checksum: Vec<u8>,
    descr: FileDescrHeader,
}

pub const MAGIC_VERSION: &str = "BKTREE: 0000";
pub const HASH_HEADER_NAME: &str = "SHA256";
pub const PREFIX_SIZE: usize = 86;

impl Header {
    pub fn read(
        file: &mut File,
        verify_checksum: bool,
    ) -> Result<Header, Box<dyn error::Error + 'static>> {
        let mut header: Header = Default::default();
        let mut reader = BufReader::new(file);

        // Check the magic number
        reader.read_until(b'\n' as u8, &mut header.version)?;
        if header.version != MAGIC_VERSION.as_bytes() {
            return Err("Unknown file format (expected \"BKTREE: 0000\")".into());
        }

        // Read the checksum
        let mut checksum_type: Vec<u8> = Vec::new();
        reader.read_until(b':' as u8, &mut checksum_type)?;
        if checksum_type != HASH_HEADER_NAME.as_bytes() {
            return Err("Unknown checksum format (expected \"SHA256\")".into());
        }
        let mut checksum: Vec<u8> = Vec::new();
        reader.read_until('\n' as u8, &mut checksum)?;
        header.checksum = checksum.trim_start_matches(b' ' as u8);

        let descr_start = reader.seek(SeekFrom::Current(0))?;
        if verify_checksum {
            let mut hasher = Sha256::new();
            io::copy(&mut reader, &mut hasher)?;
            let found = format!("{:x}", hasher.result());
            if found.as_bytes() != header.checksum.as_slice() {
                return Err(format!(
                    "Checksum failure. Found {:?}, expected {:?}",
                    found, header.checksum
                )
                .into());
            }
        }
        reader.seek(SeekFrom::Start(descr_start))?;

        Ok(header)
    }
}
