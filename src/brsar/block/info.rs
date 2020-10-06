#![allow(unused)]

use crate::common::*;
use binread::{BinRead, FilePtr32};
use std::convert::TryFrom;

#[derive(BinRead)]
pub struct InfoBlock {
    pub header: BlockHeader,
    pub sound_table: Reference<Table<Reference<SoundInfo>>>,
    pub bank_table: Reference<Table<Reference<() /* BankInfo*/>>>,
    pub player_table: Reference<Table<Reference<() /*PlayerInfo*/>>>,
    pub file_table: Reference<Table<Reference<FileInfo>>>,
    pub group_table: Reference<Table<Reference<GroupInfo>>>,
    //#[br(align_after = 0x20)]
    pub sound_archive_info: Reference<SoundArchiveInfo>
}

// TODO: version differences
#[derive(BinRead)]
pub struct SoundInfo {
    pub string_id: u32, //TypedId,
    pub file_id: u32, // TypedId,
    pub player_id: TypedId,
    pub sound_info_3d: Reference<()>,
    pub volume: u8,
    pub player_priority: u8,
    pub sound_type: SoundType,
    pub remote_filter: u8,
    pub details: MultiReference<SoundDetails>,
    pub user: [u32; 2],
    pub pan_mode: PanMode,
    pub pan_curve: PanCurve,
    pub actor_player_id: u8,
    pub reserved: u8
}

#[derive(BinRead)]
#[repr(u8)]
pub enum SoundType {
    #[br(magic = 0u8)] Invalid = 0,
    #[br(magic = 1u8)] Sequence = 1,
    #[br(magic = 2u8)] Stream = 2,
    #[br(magic = 3u8)] Wave = 3
}

#[derive(BinRead)]
#[br(import(ty: u8, a: ()))]
pub enum SoundDetails {
    // TODO: confirm type IDs
    #[br(pre_assert(ty == 1))] Sequence(SeqDetails),
    #[br(pre_assert(ty == 2))] Stream(StreamDetails),
    #[br(pre_assert(ty == 3))] Wave(WaveDetails)
}

// from tockdom wiki
#[derive(BinRead)]
pub struct SeqDetails {
    seq_label_entry: u32,
    soundbank_index: u32,
    unknown: [u8; 3], // part of alloc_track?
    alloc_track: u8, // not u16?
    priority: u8,
    unknown2: [u8; 7] // unknown
}

#[derive(BinRead)]
pub struct StreamDetails {
    start_pos: u32,
    unknown: u8,
    channel_count: u8, // maybe in older versions this + unknown was bitmask, as mentioned by gota?
    alloc_track: u8, // *shrug*
    unknown2: [u8; 5]
}

#[derive(BinRead)]
pub struct WaveDetails {
    sound_data_node: u32,
    unknown: [u8; 3], // part of alloc_track?
    alloc_track: u8,
    priority: u8,
    unknown2: [u8; 7]
}

#[derive(BinRead)]
#[repr(u8)]
pub enum PanMode {
    #[br(magic = 0u8)] Dual = 0,
    #[br(magic = 1u8)] Balance = 1
}

#[derive(BinRead)]
#[repr(u8)]
pub enum PanCurve {
    #[br(magic = 0u8)] Sqrt = 0,
    #[br(magic = 1u8)] Sqrt0Db = 1,
    #[br(magic = 2u8)] Sqrt0DbClamp = 2,
    #[br(magic = 3u8)] SinCos = 3,
    #[br(magic = 4u8)] SinCos0Db = 4,
    #[br(magic = 5u8)] SinCos0DbClamp = 5,
    #[br(magic = 6u8)] Linear = 6,
    #[br(magic = 7u8)] Linear0Db = 7,
    #[br(magic = 8u8)] Linear0DbClamp = 8
}

#[derive(BinRead)]
pub struct BankInfo {
    string_id: TypedId,
    file_id: TypedId,
    reserved: u32
}

#[derive(BinRead)]
pub struct PlayerInfo {
    string_id: TypedId,
    max_sounds: u8, // maybe u32?
    padding: [u8; 3],
    heap_space: u32,
    reserved: u32
}

#[derive(BinRead)]
pub struct FileInfo {
    pub file_size: u32,
    pub archive_size: u32, // "length of audio data, null for external or rseq"
    pub file_id: s32, // RhythmRevolution not clear on what this is, maybe an entry number? always 0xFFFFFFFF according to tockdom
    //#[br(try)]
    pub external_file: u64, //Option<DerefTest<Reference<NullString>>>,
    // "offset to second subsection, or external file name" is this to GroupEntry or Table<GroupEntry>?
    pub file_positions: Reference<Table<Reference<FilePosition>>>,
    // tockdom places a single FilePosition here:
    // file_position: FilePosition
}

#[derive(BinRead)]
pub struct FilePosition {
    pub group_index: u32,
    pub item_index: u32
}

#[derive(BinRead)]
pub struct GroupInfo {
    string_id: TypedId, // file name index
    group_id: s32, // actually unknown, always 0xFFFFFFFF?
    external_file: u64 /*Reference<NullString>*/,
    pub file_base: u32,
    total_size: u32,
    archive_base: u32, // TODO: type?
    archive_size: u32, // TODO: total size?
    #[br(args(file_base as u64, archive_base as u64))]
    pub entries: Reference<Table<Reference<GroupEntry>>>,
    // the table itself usually follows immediately afterward
}

#[derive(BinRead)]
pub struct File(#[br(parse_with = binread::helpers::read_bytes)] pub Vec<u8>);

#[derive(BinRead)]
#[br(import(file_base: u64, archive_base: u64))]
pub struct GroupEntry {
    file_id: TypedId, // file_table index? sound index?
    // nintendo, why do you have to put size after the offsets :(
    // this is probably temporary until Vec<u8> gets replaced with a more appropriate type?
    // #[br(restore_position, map = |(_, size): (u32, u32)| size)]
    // pub file_size: u32,
    // #[br(offset = file_base, count = file_size)]
    pub file_offset: binread::PosValue<u32>,//FilePtr32<File>, // type? // TODO: this is inefficient, since multiple groups can refer to the same file.
    pub file_size: u32,
    // nintendo, why do you have to put size after the offsets :(
    // this is probably temporary until Vec<u8> gets replaced with a more appropriate type?
    // #[br(restore_position, map = |(_, size): (u32, u32)| size)]
    // archive_size: u32,
    #[br(offset = archive_base)]
    archive_offset: FilePtr32<()>, // type? file or subsection?
    archive_size: u32, // file or subsection?
    reserved: u32
}

#[derive(BinRead)]
pub struct SoundArchiveInfo {
    max_sequences: u16,
    max_seq_tracks: u16,
    max_streams: u16,
    max_stream_tracks: u16,
    max_stream_channels: u16,
    max_waves: u16,
    max_wave_tracks: u16,
    padding: u16,
    reserved: u32
}