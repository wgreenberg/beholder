use crate::common::StaticByteSize;
use crate::error::UnpackResult;

use deku::{prelude::*, error::DekuError};
use lz4_flex::block::decompress;
use flate2::{Decompress, FlushDecompress};

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", magic = b"LSPK")]
pub struct PakHeader {
    pub version: u32,
    pub file_list_offset: u64,
    pub file_list_size: u32,
    pub flags: u8,
    pub priority: u8,
    pub md5: [u8; 16],
    pub num_parts: u16,
}

impl StaticByteSize for PakHeader {
    const SIZE: usize = 4 + 8 + 4 + 1 + 1 + 16 + 2;
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8", bits = 4, endian = "little")]
pub enum PakFileCompressionMethod {
    #[deku(id = "0")]
    None,
    #[deku(id = "1")]
    Zlib,
    #[deku(id = "2")]
    LZ4,
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
pub struct PakFileEntry {
    pub name: [u8; 256],
    pub offset_in_file1: u32,
    pub offset_in_file2: u16,
    pub archive_part: u8,
    #[deku(pad_bits_before = "4")]
    pub compression_method: PakFileCompressionMethod,
    pub size_on_disk: u32,
    pub uncompressed_size: u32,
}

impl StaticByteSize for PakFileEntry {
    const SIZE: usize = 256 + 4 + 2 + 1 + 1 + 4 + 4;
}

impl PakFileEntry {
    pub fn get_name(&self) -> String {
        self.name.iter()
            .take_while(|c| **c != 0)
            .map(|c| *c as char)
            .collect()
    }

    pub fn get_offset_in_file(&self) -> usize {
        (self.offset_in_file1 as usize) | ((self.offset_in_file2 as usize) << 32)
    }

    pub fn decompress(&self, data: &[u8]) -> UnpackResult<Vec<u8>> {
        let mut decompressed = Vec::with_capacity(self.uncompressed_size as usize);
        match self.compression_method {
            PakFileCompressionMethod::None => {
                decompressed.extend_from_slice(&data[..self.uncompressed_size as usize]);
            },
            PakFileCompressionMethod::Zlib => {
                let mut decoder = Decompress::new(false);
                decoder.decompress_vec(&data, &mut decompressed, FlushDecompress::None)?;
            },
            PakFileCompressionMethod::LZ4 => {
                decompressed.extend(decompress(&data, self.uncompressed_size as usize)?);
            },
        }
        Ok(decompressed)
    }
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct PakFileListHeader {
    pub num_files: u32,
    pub compressed_size: u32,
}

impl PakFileListHeader {
    pub fn decompress(&self, compressed_file_list: &[u8]) -> UnpackResult<Vec<PakFileEntry>> {
        let buf = decompress(compressed_file_list, PakFileEntry::SIZE * self.num_files as usize)?;
        let mut files = Vec::new();
        let mut bookmark = &buf[..];
        for _ in 0..self.num_files {
            let ((rest, leftover), file) = PakFileEntry::from_bytes((&bookmark, 0))?;
            assert_eq!(leftover, 0);
            bookmark = rest;
            files.push(file);
        }
        Ok(files)
    }

    pub fn decompress_iter(&self, compressed_file_list: &[u8]) -> UnpackResult<PakFileEntryIter> {
        let buf = decompress(compressed_file_list, PakFileEntry::SIZE * self.num_files as usize)?;
        Ok(PakFileEntryIter {
            decompressed_file_list: buf,
            num_files: self.num_files as usize,
            current_file: 0,
        })
    }
}

pub struct PakFileEntryIter {
    decompressed_file_list: Vec<u8>,
    num_files: usize,
    current_file: usize,
}

impl Iterator for PakFileEntryIter {
    type Item = UnpackResult<PakFileEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_file >= self.num_files {
            return None;
        }
        let buf = &self.decompressed_file_list[self.current_file * PakFileEntry::SIZE..];
        match PakFileEntry::from_bytes((&buf, 0)) {
            Ok((_, file)) => {
                self.current_file += 1;
                Some(Ok(file))
            },
            Err(e) => Some(Err(e.into())),
        }
    }
}
