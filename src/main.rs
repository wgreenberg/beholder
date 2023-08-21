use beholder::*;
use std::fs::File;
use std::io::{Read, SeekFrom, Seek};
use deku::prelude::*;
use lz4_flex::block::decompress;

fn main() {
    let mut file = File::open("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Baldurs Gate 3\\Data\\Gustav.pak").unwrap();
    let mut buf = vec![0; 1000];
    file.read_exact(&mut buf).unwrap();
    let (_, header) = pak::PakHeader::from_bytes((&buf, 0)).unwrap();
    dbg!(&header);
    assert_eq!(header.version, 18);

    file.seek(SeekFrom::Start(header.file_list_offset)).unwrap();
    buf.clear();
    buf.resize(header.file_list_size as usize, 0);
    file.read_exact(&mut buf).unwrap();
    let ((rest, _), file_list_hdr) = pak::PakFileListHeader::from_bytes((&buf, 0)).unwrap();
    let lsf_entries: Vec<pak::PakFileEntry> = file_list_hdr.decompress_iter(&rest).unwrap()
        .map(|r| r.unwrap())
        .filter(|f| f.get_name().ends_with(".lsf"))
        .collect();
    use std::collections::HashMap;
    let mut lsf_versions: HashMap<u32, u32> = HashMap::new();
    for entry in lsf_entries {
        file.seek(SeekFrom::Start(entry.get_offset_in_file() as u64)).unwrap();
        let mut compressed = vec![0; entry.size_on_disk as usize];
        file.read_exact(&mut compressed).unwrap();
        let decompressed = decompress(&compressed, entry.uncompressed_size as usize).unwrap();
        let (_, lsf_header) = lsf::LsfHeader::from_bytes((&decompressed, 0)).unwrap();
        lsf_versions.entry(lsf_header.version).and_modify(|v| *v += 1).or_insert(1);
    }
    dbg!(lsf_versions);
}
