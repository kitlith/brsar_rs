pub mod block;

use crate::common::*;
use block::{SymbolBlock, InfoBlock, FileBlock};
use binread::BinRead;

#[derive(BinRead)]
pub struct BRSAR {
    #[br(assert(header.block_count == 3), assert(header.version == 0x0104), assert(&header.magic == b"RSAR"))]
    pub header: FileHeader,
    #[br(deref_now, is_big = header.endian == Endian::Big)]
    pub symbol: BlockPtr<SymbolBlock>,
    #[br(is_big = header.endian == Endian::Big)]
    pub info: BlockPtr<InfoBlock>,
    #[br(is_big = header.endian == Endian::Big, align_after = 0x20)]
    pub file: BlockPtr<FileBlock>
}
