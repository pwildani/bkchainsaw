/*
 * This is one layer above a file format. It describes how to interpret two chunks of bytes as
 * node in a BK tree structure and the raw key bytes.
 *
 * All multi byte entities are stored little endian.
*/
use std::cell::RefCell;
use std::error::Error;

use byteorder::{ByteOrder, LittleEndian};

use crate::Dist;

pub trait InStorageNode {
    fn encoding_size(&self) -> usize;
    fn dist(&self) -> Option<Dist>;
    fn child_count(&self) -> Option<usize>;
    fn children_offset(&self) -> Option<usize>;
    fn key_offset(&self) -> Option<usize>;
    fn key_length(&self) -> Option<usize>;
    // fn key_bytes(&self) -> Option<&'a [u8]>;
}

type NodeMutationResult = Result<(), Box<dyn Error>>;

pub trait InStorageNodeMut: InStorageNode {
    type Key;
    fn set_key(&mut self, key: Self::Key) -> NodeMutationResult;
    fn set_dist(&mut self, dist: Dist) -> NodeMutationResult;
    fn set_num_children(&mut self, n: usize) -> NodeMutationResult;
    fn set_child_offset(&mut self, child_offset: usize) -> NodeMutationResult;
}

/**
 * Variable Key Bytes, 16 bit child counters and distances.
 *
 * Max total key size is 4GiB.
 * Max total node size is 4GiB

 * VBNode16 node array, 0 <= dist and children < 2**16,
 * {dist from parent, num children, key byte offset,}
 *   * dist from parent: 2 bytes
 *   * num children: 2 bytes
 *   * key byte offset: 4 bytes
 *   * children start offset: 4 bytes
 * == 12 bytes per entry

 * VBNode16 Key array: adjacent keys all smooshed together. These MUST be stored
 * in the same order as VBNode16 instances.
*/
#[derive(Clone)]
struct VBNode16<'a> {
    pub node_buffer: &'a [u8],
    pub key_buffer: &'a [u8],
    pub offset: usize,
}

/*
impl<'a> VBNode16<'a> {
    fn get(&self, offset: usize, len: usize) -> Option<&[u8]> {
        get_slice(self.node_buffer, self.offset, offset, len)
    }

    fn key_end(&self) -> Option<usize> {
        // Read the key offset of the next VBNode16 to figure out where our key ends.
        // This is the V part of the name.
        let next = VBNode16 {
            offset: self.offset + self.encoding_size(),
            ..*self
        };
        return next.key_offset();
    }
}
*/

/*
impl<'a> InStorageNode<'a> for VBNode16<'a> {
    fn encoding_size(&self) -> usize {
        12
    }

    fn dist(&self) -> Option<Dist> {
        Some(LittleEndian::read_u16(self.get(0, 2)?) as Dist)
    }
    fn child_count(&self) -> Option<usize> {
        Some(LittleEndian::read_u16(self.get(2, 2)?) as Dist)
    }
    fn key_offset(&self) -> Option<usize> {
        Some(LittleEndian::read_u16(self.get(4, 4)?) as Dist)
    }
    fn children_offset(&self) -> Option<usize> {
        Some(LittleEndian::read_u16(self.get(8, 4)?) as Dist)
    }
    fn key_length(&self) -> Option<usize> {
        Some(self.key_end()? - self.key_offset()?)
    }
    fn key_bytes(&self) -> Option<&'a [u8]> {
        let start = self.key_offset()?;
        Some(match self.key_end() {
            Some(end) => &self.key_buffer[start..end],
            // Last node in the file.
            None => &self.key_buffer[start..],
        })
    }
}
*/

/**
 * 64 bit keys, 8 bit child counters and distances.
 *
 * Max total key size is usize::MAX bytes
 * Max total node size is 4GiB
 *
 * F64BNode8 node array, 0 <= dist and children < 2**8, keys are fixed size 8 byte values.
 *   * dist from parent: 1 byte
 *   * num children: 1 bytes
 *   * padding: 2 bytes (must be 0)
 *   * children offset: 4 bytes
 *  Total: 8 bytes
 *
 * F64BNode8 key array: adjacent keys at fixed offsets.
*/

pub struct F64BNode8<'a> {
    pub node_buffer: RefCell<&'a mut [u8]>,
    pub key_buffer: RefCell<&'a mut [u8]>,
    pub offset: usize,
}

impl<'a> F64BNode8<'a> {
    fn key_end(&self) -> Option<usize> {
        let end = self.key_offset()? + self.key_length()?;
        if end <= self.key_buffer.borrow().len() {
            return Some(self.key_offset()? + 8);
        }
        return None;
    }
    fn key(&self) -> Option<u64> {
        let start = self.key_offset()?;
        match self.key_end() {
            Some(end) => Some(LittleEndian::read_u64(
                &self.key_buffer.borrow()[start..end],
            )),
            // Last node in the file. Shouldn't happen for this type, unless the key buffer was truncated.
            None => panic!("Key buffer appears to have been truncated!"),
        }
    }

    fn next_node(self) -> F64BNode8<'a> {
        F64BNode8 {
            offset: self.offset + self.encoding_size(),
            ..self
        }
    }

    fn prev_node(self) -> F64BNode8<'a> {
        F64BNode8 {
            offset: self.offset - self.encoding_size(),
            ..self
        }
    }

    fn first_child(self) -> Option<F64BNode8<'a>> {
        Some(F64BNode8 {
            offset: self.children_offset()?,
            ..self
        })
    }
}

impl<'a> InStorageNode for F64BNode8<'a> {
    fn encoding_size(&self) -> usize {
        8
    }

    fn dist(&self) -> Option<Dist> {
        //Some(LittleEndian::read_u8(self.get(0, 1)?) as Dist)
        Some(get_slice(&self.node_buffer.borrow(), self.offset, 0, 1)?[0] as Dist)
    }
    fn child_count(&self) -> Option<usize> {
        //Some(LittleEndian::read_u8(self.get(1, 1)?) as Dist)
        Some(get_slice(&self.node_buffer.borrow(), self.offset, 1, 1)?[0] as Dist)
    }
    fn children_offset(&self) -> Option<usize> {
        let offset =
            LittleEndian::read_u16(get_slice(&self.node_buffer.borrow(), self.offset, 4, 4)?)
                as Dist;
        if offset > 0 {
            Some(offset)
        } else {
            None
        }
    }
    fn key_offset(&self) -> Option<usize> {
        let entry_index = self.offset / self.encoding_size();
        Some(entry_index * self.key_length()?)
    }
    fn key_length(&self) -> Option<usize> {
        Some(8)
    }
}

impl<'a> InStorageNodeMut for F64BNode8<'a> {
    type Key = u64;

    fn set_key(&mut self, key: u64) -> NodeMutationResult {
        LittleEndian::write_u64(
            get_slice_mut(
                &mut self.key_buffer.borrow_mut(),
                self.key_offset().ok_or("no key offset")?,
                0,
                8,
            )
            .ok_or("out of space for key")?,
            key,
        );
        Ok(())
    }
    fn set_dist(&mut self, dist: Dist) -> NodeMutationResult {
        get_slice_mut(&mut self.node_buffer.borrow_mut(), self.offset, 0, 1)
            .ok_or("out of space for dist")?[0] = dist as u8;
        Ok(())
    }
    fn set_num_children(&mut self, n: usize) -> NodeMutationResult {
        get_slice_mut(&mut self.node_buffer.borrow_mut(), self.offset, 1, 1)
            .ok_or("out of space for child count")?[0] = n as u8;
        Ok(())
    }
    fn set_child_offset(&mut self, offset: usize) -> NodeMutationResult {
        LittleEndian::write_u16(
            get_slice_mut(&mut self.node_buffer.borrow_mut(), self.offset, 4, 4)
                .ok_or("out of space for child offset")?,
            offset as u16,
        );
        Ok(())
    }
}

fn get_slice<'a: 'b, 'b>(
    buf: &'a [u8],
    offset1: usize,
    offset2: usize,
    len: usize,
) -> Option<&'b [u8]> {
    let start = offset1 + offset2;
    let end = start + len;
    if buf.len() >= end {
        return Some(&buf[start..end]);
    }
    return None;
}

fn get_slice_mut<'a: 'b, 'b>(
    buf: &'a mut [u8],
    offset1: usize,
    offset2: usize,
    len: usize,
) -> Option<&'b mut [u8]> {
    let start = offset1 + offset2;
    let end = start + len;
    if buf.len() >= end {
        return Some(&mut buf[start..end]);
    }
    return None;
}

#[cfg(test)]
mod test {
    use super::*;

    /*
    #[test]
    fn single_vbnode16() {
        let nodes = &[8, 0, 5, 0, 1, 0, 0, 0, 7, 0, 0, 0];
        let keys = &[0, 1, 2, 3, 4, 5, 6, 7];
        let node = VBNode16 {
            offset: 0,
            node_buffer: nodes,
            key_buffer: keys,
        };
        assert_eq!(Some(8), node.dist());
        assert_eq!(Some(5), node.child_count());
        assert_eq!(Some(1), node.key_offset());
        assert_eq!(Some(7), node.children_offset());
        assert_eq!(Some(&keys[1..]), node.key_bytes());
    }

    #[test]
    fn two_vbnode16() {
        let nodes = &[
            8, 0, 5, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
        ];
        let keys = &[0, 1, 2, 3, 4, 5, 6, 7];
        {
            let node = VBNode16 {
                offset: 0,
                node_buffer: nodes,
                key_buffer: keys,
            };
            assert_eq!(Some(&keys[1..4]), node.key_bytes());
        }
        {
            let node = VBNode16 {
                offset: 12,
                node_buffer: nodes,
                key_buffer: keys,
            };
            assert_eq!(Some(4), node.key_offset());
            assert_eq!(Some(&keys[4..8]), node.key_bytes());
        }
    }
    */

    #[test]
    fn single_f64bnode8() {
        let nodes = &[8, 5, 0, 0, 1, 0, 0, 0];
        let keys = &[0, 1, 2, 3, 4, 5, 6, 7];
        let node = F64BNode8 {
            offset: 0,
            node_buffer: RefCell::new(nodes),
            key_buffer: RefCell::new(keys),
        };
        assert_eq!(Some(8), node.dist());
        assert_eq!(Some(5), node.child_count());
        assert_eq!(Some(0), node.key_offset());
        assert_eq!(Some(1), node.children_offset());
        assert_eq!(Some(&keys[0..8]), node.key_bytes());
    }

    #[test]
    fn two_f64bnode8() {
        let nodes = &[8, 5, 1, 0, 1, 0, 0, 0, 4, 3, 0, 0, 0, 0, 0, 0];
        let keys = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        {
            let node = F64BNode8 {
                offset: 0,
                node_buffer: RefCell::new(nodes),
                key_buffer: RefCell::new(keys),
            };
            assert_eq!(Some(8), node.dist());
            assert_eq!(Some(5), node.child_count());
            assert_eq!(Some(0), node.key_offset());
            assert_eq!(Some(1), node.children_offset());
            assert_eq!(Some(&keys[0..8]), node.key_bytes());
        }
        {
            let node = F64BNode8 {
                offset: 8,
                node_buffer: RefCell::new(nodes),
                key_buffer: RefCell::new(keys),
            };
            assert_eq!(Some(4), node.dist());
            assert_eq!(Some(3), node.child_count());
            assert_eq!(Some(8), node.key_offset());
            assert_eq!(None, node.children_offset());
            assert_eq!(Some(&keys[8..16]), node.key_bytes());
        }
    }
}
