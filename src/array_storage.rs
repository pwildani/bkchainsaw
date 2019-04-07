
/*
   All multi byte entities are stored little endian.
*/


use byteorder::{LittleEndian, ByteOrder};

use crate::Dist;


trait InStorageNode<'a> {
    fn encoding_size(&self) -> usize;
    fn dist(&self) -> Option<Dist>;
    fn child_count(&self) -> Option<usize>;
    fn key_offset(&self) -> Option<usize>;
    fn key_length(&self) -> Option<usize>;
    fn key_bytes(&self) -> Option<&'a [u8]>;
}

/**
 * Max total key size is 4GiB. Max nodes total byte size is usize::MAX.

 * VBNode16 node array, 0 <= dist and children < 2**16,
 * {dist from parent, num children, key byte offset,}
 *   * dist from parent: 2 bytes
 *   * num children: 2 bytes
 *   * padding: 1 byte == 0
 *   * key byte offset: 4 bytes
 * == 8 bytes per entry

 * VBNode16 Key array: adjacent keys all smooshed together. These MUST be stored
 * in the same order as VBNode16 instances.
 */
#[derive(Clone)]
struct VBNode16<'a> {
    node_buffer: &'a [u8],
    key_buffer: &'a [u8],
    offset: usize,
}

impl<'a> VBNode16<'a> {
    fn get(&self, offset: usize, len: usize) -> Option<&[u8]> {
        get_slice(self.node_buffer, self.offset, offset, len)
    }

    fn key_end(&self) -> Option<usize> {
        // Read the key offset of the next VBNode16 to figure out where our key ends.
        // This is the V part of the name.
        let next = VBNode16{offset: self.offset + self.encoding_size(), ..*self};
        return next.key_offset();
    }
}

impl<'a> InStorageNode<'a> for VBNode16<'a> {

    fn encoding_size(&self) -> usize { 8 }

    fn dist(&self) -> Option<Dist> { 
        Some(LittleEndian::read_u16(self.get(0, 2)?) as Dist)
    }
    fn child_count(&self) -> Option<usize> { 
        Some(LittleEndian::read_u16(self.get(2, 2)?) as Dist)
    }
    fn key_offset(&self) -> Option<usize> {
        Some(LittleEndian::read_u16(self.get(4, 4)?) as Dist)
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

/**
 * Max total key size is usize::MAX bytes
 *
 * F64BNode8 node array, 0 <= dist and children < 2**8, keys are fixed size 8 byte values.
 *   * dist from parent: 1 byte
 *   * num children: 1 bytes
 *
 * F64BNode8 key array: adjacent keys at fixed offsets.
*/
#[derive(Clone)]
struct F64BNode8<'a> {
    node_buffer: &'a [u8],
    key_buffer: &'a [u8],
    offset: usize,
}

impl<'a> F64BNode8<'a> {
    fn get(&self, offset: usize, len: usize) -> Option<&[u8]> {
        get_slice(self.node_buffer, self.offset, offset, len)
    }

    fn key_end(&self) -> Option<usize> {
        let end = self.key_offset()? + self.key_length()?;
        if end <= self.key_buffer.len() {
            return Some(self.key_offset()? + 8);
        }
        return None;
    }
}

impl<'a> InStorageNode<'a> for F64BNode8<'a> {

    fn encoding_size(&self) -> usize { 2 }

    fn dist(&self) -> Option<Dist> { 
        //Some(LittleEndian::read_u8(self.get(0, 1)?) as Dist)
        Some(self.get(0, 1)?[0] as Dist)
    }
    fn child_count(&self) -> Option<usize> { 
        //Some(LittleEndian::read_u8(self.get(1, 1)?) as Dist)
        Some(self.get(1, 1)?[0] as Dist)
    }
    fn key_offset(&self) -> Option<usize> {
        let entry_index = self.offset / self.encoding_size();
        Some(entry_index * self.key_length()?)
    }
    fn key_length(&self) -> Option<usize> { Some(8) }

    fn key_bytes(&self) -> Option<&'a [u8]> {
        let start = self.key_offset()?;
        match self.key_end() {
            Some(end) => Some(&self.key_buffer[start..end]),
            // Last node in the file. Shouldn't happen for this type, unless the key buffer was truncated.
            None => panic!("Key buffer appears to have been truncated!"),
        }
    }
}


fn get_slice(buf: &[u8], offset1: usize, offset2: usize, len: usize) -> Option<&[u8]>{
    let start = offset1 + offset2;
    let end = start + len;
    if buf.len() >= end { 
        return Some(&buf[start..end]);
    }
    return None
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn single_vbnode16() {
        let nodes = &[8,0, 5,0, 1,0,0,0];
        let keys = &[0, 1,2,3,4,5,6,7];
        let node = VBNode16{offset: 0, node_buffer: nodes, key_buffer: keys};
        assert_eq!(Some(8), node.dist());
        assert_eq!(Some(5), node.child_count());
        assert_eq!(Some(1), node.key_offset());
        assert_eq!(Some(&keys[1..]), node.key_bytes());
    }

    #[test]
    fn two_vbnode16() {
        let nodes = &[8,0, 5,0, 1,0,0,0,  0,0, 0,0, 4,0,0,0];
        let keys = &[0,1,2,3,4,5,6,7];
        {
            let node = VBNode16{offset: 0, node_buffer: nodes, key_buffer: keys};
            assert_eq!(Some(&keys[1..4]), node.key_bytes());
        }
        {
            let node = VBNode16{offset: 8, node_buffer: nodes, key_buffer: keys};
            assert_eq!(Some(&keys[4..8]), node.key_bytes());
        }
    }

    #[test]
    fn single_f64bnode8() {
        let nodes = &[8,5];
        let keys = &[0, 1,2,3,4,5,6,7];
        let node = F64BNode8{offset: 0, node_buffer: nodes, key_buffer: keys};
        assert_eq!(Some(8), node.dist());
        assert_eq!(Some(5), node.child_count());
        assert_eq!(Some(0), node.key_offset());
        assert_eq!(Some(&keys[0..8]), node.key_bytes());
    }

    #[test]
    fn two_f64bnode8() {
        let nodes = &[8,5,4,3];
        let keys = &[0,1,2,3,4,5,6,7, 8,9,10,11,12,13,14,15];
        {
            let node = F64BNode8{offset: 0, node_buffer: nodes, key_buffer: keys};
            assert_eq!(Some(8), node.dist());
            assert_eq!(Some(5), node.child_count());
            assert_eq!(Some(0), node.key_offset());
            assert_eq!(Some(&keys[0..8]), node.key_bytes());
        }
        {
            let node = F64BNode8{offset: 2, node_buffer: nodes, key_buffer: keys};
            assert_eq!(Some(4), node.dist());
            assert_eq!(Some(3), node.child_count());
            assert_eq!(Some(8), node.key_offset());
            assert_eq!(Some(&keys[8..16]), node.key_bytes());
        }
    }
}