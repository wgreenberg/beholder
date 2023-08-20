use std::io::Read;

use deku::prelude::*;
use lzzzz::lz4;
use flate2::{Decompress, FlushDecompress};

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", magic = b"LSPK")]
struct PakHeader {
    pub version: u32,
    pub file_list_offset: u64,
    pub file_list_size: u32,
    pub flags: u8,
    pub priority: u8,
    pub md5: [u8; 16],
    pub num_parts: u16,
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8", bits = 4, endian = "little")]
enum PakFileCompressionMethod {
    #[deku(id = "0")]
    None,
    #[deku(id = "1")]
    Zlib,
    #[deku(id = "2")]
    LZ4,
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
struct PakFileEntry {
    pub name: [u8; 256],
    pub offset_in_file1: u32,
    pub offset_in_file2: u16,
    pub archive_part: u8,
    #[deku(pad_bits_before = "4")]
    pub compression_method: PakFileCompressionMethod,
    pub size_on_disk: u32,
    pub uncompressed_size: u32,
}

impl PakFileEntry {
    // surely there's a better way...
    const SIZE: usize = 256 + 4 + 2 + 1 + 1 + 4 + 4;

    pub fn get_name(&self) -> String {
        self.name.iter()
            .take_while(|c| **c != 0)
            .map(|c| *c as char)
            .collect()
    }

    pub fn get_offset_in_file(&self) -> usize {
        (self.offset_in_file1 as usize) | ((self.offset_in_file2 as usize) << 32)
    }

    pub fn decompress(&self, data: &[u8]) -> Vec<u8> {
        let mut decompressed = Vec::with_capacity(self.uncompressed_size as usize);
        match self.compression_method {
            PakFileCompressionMethod::None => {
                decompressed.extend_from_slice(&data[..self.uncompressed_size as usize]);
            },
            PakFileCompressionMethod::Zlib => {
                let mut decoder = Decompress::new(false);
                decoder.decompress_vec(&data, &mut decompressed, FlushDecompress::None).unwrap();
            },
            PakFileCompressionMethod::LZ4 => {
                let mut decompressor = lz4::Decompressor::new().unwrap();
                decompressed.extend(decompressor.next(&data, self.uncompressed_size as usize).unwrap());
            },
        }
        decompressed
    }
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct PakCompressedFileList {
    pub num_files: u32,
    pub compressed_size: u32,
}

impl PakCompressedFileList {
    pub fn decompress(&self, compressed_file_list: &[u8]) -> Vec<PakFileEntry> {
        let mut decompressor = lz4::Decompressor::new().unwrap();
        let decompressed = decompressor.next(compressed_file_list, self.num_files as usize * PakFileEntry::SIZE).unwrap();
        let mut files = Vec::new();
        let mut bookmark = &decompressed[..];
        for _ in 0..self.num_files {
            let ((rest, _), file) = PakFileEntry::from_bytes((bookmark, 0)).unwrap();
            bookmark = rest;
            files.push(file);
        }
        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_file_bytes(filename: &str) -> Vec<u8> {
        use std::fs::File;

        let mut file = File::open(filename).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        data
    }

    #[test]
    fn test_header() {
        let bytes = get_file_bytes("test/data/ImprovedUI.pak");
        let (_, header) = PakHeader::from_bytes((bytes.as_ref(), 0)).unwrap();
        assert_eq!(header.version, 18);

        let ((rest, _), file_list_c) = PakCompressedFileList::from_bytes((&bytes, header.file_list_offset as usize * 8)).unwrap();
        let file_list = file_list_c.decompress(&rest);
        for f in file_list {
            let compressed_file = &bytes[f.get_offset_in_file()..f.get_offset_in_file() + f.size_on_disk as usize];
            let _contents = f.decompress(&compressed_file);
        }
    }
}
