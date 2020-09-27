#![allow(unused)]

use crate::common::*;
use std::convert::TryFrom;
use binread::BinRead;

// TODO: work from FileInfo from Info Block?
#[derive(BinRead)]
pub struct FileBlock {
    header: BlockHeader,
    #[br(count = header.size as u64)]
    body: Vec<u8>
}