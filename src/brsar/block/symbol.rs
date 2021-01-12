#![allow(unused)]

use crate::common::*;
use super::info::{SoundInfo, PlayerInfo, GroupInfo, BankInfo};

use std::convert::TryFrom;
use std::marker::PhantomData;
use binread::BinRead;
use binread::FilePtr32;

// struct SymbolBlock<'a> {
//     names: Vec<&'a CStr>,
//     sound_tree: PatriciaTree<()>,
//     player_tree: PatriciaTree<()>,
//     group_tree: PatriciaTree<()>,
//     bank_tree: PatriciaTree<()>
// }
//
// impl<'a> TryFrom<BlockHeader<'a>> for SymbolBlock<'a> {
//     type Error = ();
//
//     fn try_from(_value: BlockHeader) -> Result<Self, Self::Error> {
//         unimplemented!()
//     }
// }
//
// impl<'a> Block<'a> for SymbolBlock<'a> {
//     const MAGIC: [u8; 4] = *b"SYMB";
// }

type PatriciaTree/*<T>*/ = nintendo_patricia_tree::PatriciaTree<TreeData/*<T>*/>;

#[derive(BinRead)]
pub struct SymbolBlock {
    pub header: BlockHeader,
    pub string_table: r32<Table<r32<NullString>>>,
    pub sound_tree: r32<PatriciaTree/*<SoundInfo>*/>,
    pub player_tree: r32<PatriciaTree/*<PlayerInfo>*/>,
    pub group_tree: r32<PatriciaTree/*<GroupInfo>*/>,
    pub bank_tree: r32<PatriciaTree/*<BankInfo>*/>,
    //name_table: Table<r32<CString>>, location coincidence.
}

// TODO: rename to something to do with indices?
#[derive(BinRead)]
pub struct TreeData/*<T>*/ {
    pub string_index: u32,
    pub item_index: u32, // in info
    /*_phantom: PhantomData<T>*/
}