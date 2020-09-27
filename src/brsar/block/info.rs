#![allow(unused)]

use crate::common::*;
use binread::{BinRead, FilePtr32};
use std::convert::TryFrom;

#[derive(BinRead)]
pub struct InfoBlock {
    header: BlockHeader,
    sound_table: Reference<Table<Reference<SoundInfo>>>,
    bank_table: Reference<Table<Reference<BankInfo>>>,
    player_table: Reference<Table<Reference<PlayerInfo>>>,
    file_table: Reference<Table<Reference<FileInfo>>>,
    group_table: Reference<Table<Reference<GroupInfo>>>,
    #[br(align_after = 0x20)]
    sound_archive_info: Reference<Table<Reference<SoundArchiveInfo>>>
}

// TODO: version differences
#[derive(BinRead)]
pub struct SoundInfo {
    string_id: TypedId,
    file_id: TypedId,
    player_id: TypedId,
    sound_info_3d: Reference<()>,
    volume: u8,
    player_priority: u8,
    sound_type: SoundType,
    remote_filter: u8,
    details: MultiReference<SoundDetails>,
    user: [u32; 2],
    pan_mode: PanMode,
    pan_curve: PanCurve,
    actor_player_id: u8,
    reserved: u8
}

#[derive(BinRead)]
#[repr(u8)]
pub enum SoundType {
    Invalid = 0,
    Sequence = 1,
    Stream = 2,
    Wave = 3
}

#[derive(BinRead)]
#[br(import(ty: u8, a: ()))]
pub enum SoundDetails {
    // TODO: check type IDs
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
    Dual = 0,
    Balance = 1
}

#[derive(BinRead)]
#[repr(u8)]
pub enum PanCurve {
    Sqrt = 0,
    Sqrt0Db = 1,
    Sqrt0DbClamp = 2,
    SinCos = 3,
    SinCos0Db = 4,
    SinCos0DbClamp = 5,
    Linear = 6,
    Linear0Db = 7,
    Linear0DbClamp = 8
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
    file_size: u32,
    archive_size: u32, // "length of audio data, null for external or rseq"
    file_id: s32, // RhythmRevolution not clear on what this is, maybe an entry number? always 0xFFFFFFFF according to tockdom
    external_file: Reference<NullString>,
    // "offset to second subsection, or external file name" is this to GroupEntry or Table<GroupEntry>?
    file_positions: Reference<Table<Reference<FilePosition>>>,
    // tockdom places a single FilePosition here:
    // file_position: FilePosition
}

#[derive(BinRead)]
pub struct FilePosition {
    group_id: TypedId, // group index?
    index: u32
}

#[derive(BinRead)]
pub struct GroupInfo {
    string_id: TypedId, // file name index
    group_id: s32, // actually unknown, always 0xFFFFFFFF?
    external_file: Reference<NullString>,
    file_base: u32,
    total_size: u32,
    archive_base: u32, // TODO: type?
    archive_size: u32, // TODO: total size?
    #[br(args(file_base as u64, archive_base as u64))]
    entries: Reference<Table<GroupEntry>>,
    // the table itself usually follows immediately afterward
}

#[derive(BinRead)]
#[br(import(file_base: u64, archive_base: u64))]
pub struct GroupEntry {
    file_id: TypedId, // group index?
    // nintendo, why do you have to put size after the offsets :(
    // this is probably temporary until Vec<u8> gets replaced with a more appropriate type?
    #[br(restore_position, map = |(_, size): (u32, u32)| size)]
    file_size: u32,
    #[br(offset = file_base, count = file_size)]
    file_offset: FilePtr32<Vec<u8>>, // type?
    //file_size: u32,
    // nintendo, why do you have to put size after the offsets :(
    // this is probably temporary until Vec<u8> gets replaced with a more appropriate type?
    #[br(restore_position, map = |(_, size): (u32, u32)| size)]
    archive_size: u32,
    #[br(offset = archive_base, count = archive_size)]
    archive_offset: FilePtr32<Vec<u8>>, // type? file or subsection?
    //archive_size: u32, // file or subsection?
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