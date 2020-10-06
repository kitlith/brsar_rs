#![allow(unused)]

use crate::common::*;
use std::convert::TryFrom;
use binread::BinRead;

// TODO: work from FileInfo from Info Block?
#[derive(BinRead)]
pub struct FileBlock {
    header: BlockHeader,
    // probably only really relevant during writing, as the info section
    // contains all the fileptrs into this section
    // #[br(align_before = 0x20, count = header.size as u64 - 0x20)]
    // body: Vec<u8>
}