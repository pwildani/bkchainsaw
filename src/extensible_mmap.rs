use std::cmp::max;
use std::fs::File;
use std::io::Result as IoResult;

use memmap::MmapMut;
use memmap::MmapOptions;

// TODO: let the caller parameterize our growth strategy.
const ONE_GIB: usize = 1 * 1024 * 1024 * 1024;

pub struct ExtensibleMmapMut {
    backing: File,
    options: MmapOptions,
    alloc: usize,
    ram: MmapMut,
}

impl ExtensibleMmapMut {
    pub fn on(backing: File) -> IoResult<Self> {
        let options = MmapOptions::new();
        let ram = unsafe { options.map_mut(&backing) }?;
        Ok(ExtensibleMmapMut {
            backing,
            options,
            ram,
            alloc: 0,
        })
    }

    pub fn next_offset(&self) -> usize {
        self.alloc
    }

    pub fn ram_mut(&mut self) -> &mut [u8] {
        &mut *self.ram
    }

    pub fn ensure_capacity(&mut self, len: usize) -> IoResult<()> {
        let cur_size = self.ram.len();
        if cur_size < len {
            // Double up to 1G, then increment by at least 1G
            let new_size = max(
                len,
                if cur_size > ONE_GIB {
                    cur_size + ONE_GIB
                } else {
                    cur_size * 2
                },
            );
            self.backing.set_len(new_size as u64)?;
            self.ram.flush_async()?;
            // TODO: figure out how to drop self::ram before allocating another giant chunk of address space.
            let mut new_ram = unsafe { MmapOptions::new().map_mut(&self.backing) }?;
            std::mem::swap(&mut self.ram, &mut new_ram);
            assert!(self.ram.len() >= len);
        }
        Ok(())
    }

    pub fn alloc_bytes(&mut self, additional: usize) -> IoResult<(usize, &mut [u8])> {
        let start = self.alloc;
        let end = self.alloc + additional;
        self.ensure_capacity(end)?;
        return Ok((start, &mut self.ram[start..end]));
    }
}
