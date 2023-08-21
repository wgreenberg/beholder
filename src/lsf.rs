use deku::prelude::*;

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(magic = b"LSOF")]
pub struct LsfHeader {
    pub version: u32,
}
