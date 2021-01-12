#![allow(unused)]

pub mod binread_utils;
pub mod binwrite_utils;

// TODO: NullString
pub use binwrite_utils::{Pool, NullString};
use std::marker::PhantomData;
use std::convert::TryFrom;

use binread::{BinRead, BinReaderExt, FilePtr32, BinResult, ReadOptions};
use binread::io::{Read, Seek, SeekFrom};
use std::any::Any;
use std::ops::{Deref, DerefMut};

#[allow(non_camel_case_types)]
pub type r32<T> = binwrite_utils::RelPtr32<T>;

#[allow(non_camel_case_types)]
pub type a32<T> = binread_utils::AbsPtr32<T>;

#[allow(non_camel_case_types)]
pub type s32 = i32;

// TODO: implement
//pub struct VarLen(u128);

// TODO: pass args
#[derive(BinRead)]
pub struct BlockPtr<BR: BinRead<Args=()>> {
    #[br(restore_position)]
    offset: u32,
    // the idea is that this makes r32 act the way we want
    // TODO: may need to do this somewhere else
    #[br(deref_now, offset = offset as u64 + 8)]
    pub block: a32<BR>,
    pub len: u32
}

#[derive(BinRead, PartialEq, Debug)]
#[br(big)]
#[repr(u16)]
pub enum Endian {
    #[br(magic(0xFEFFu16))] Big = 0xFEFF,
    #[br(magic(0xFFFEu16))] Little = 0xFFFE,
}

#[derive(BinRead)]
pub struct FileHeader {
    pub magic: [u8; 4],
    pub endian: Endian, // 0xFEFF or 0xFFEE
    // TODO: does endianness apply?
    pub version: u16, // 0xAABB (AA = major, BB = minor)
    #[br(is_big = endian == Endian::Big)]
    pub file_size: u32,
    #[br(is_big = endian == Endian::Big)]
    pub header_size: u16,
    #[br(is_big = endian == Endian::Big)]
    pub block_count: u16,
}

// Will NOT downcast properly to files with a32 references
#[derive(BinRead)]
pub struct GenericFile {
    pub header: FileHeader,
    #[br(is_big = header.endian == Endian::Big, count = header.block_count, align_after = 0x20)]
    pub blocks: Vec<BlockPtr<GenericBlock>>,
}

#[derive(BinRead)]
pub struct BlockHeader {
    pub magic: [u8; 4],
    pub size: u32,
}

// Will NOT downcast properly to blocks with a32 references
#[derive(BinRead)]
pub struct GenericBlock {
    pub header: BlockHeader,
    #[br(count = header.size)]
    pub body: Vec<u8>
}

// TODO: make generic over count type
pub struct Table<T>(pub Vec<T>);

impl<Arg: Copy + 'static, BR: BinRead<Args=Arg>> BinRead for Table<BR> {
    type Args = Arg;

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        let mut temp_options = ro.clone();
        temp_options.count = Some(u32::read_options(reader, ro, ())? as usize);
        Ok(Table(Vec::read_options(reader, &temp_options, args)?))
    }

    fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
        let mut temp_options = ro.clone();
        temp_options.count = Some(u32::read_options(reader, ro, ())? as usize);
        self.0.after_parse(reader, &temp_options, args)
    }
}

// multiple Types are currently handled by optionally passing the type to the wrapped type
// TODO: outer struct w/ type, inner enum with reference
pub enum MultiReference<BR: BinRead> {
    Absolute(u8, a32<BR>),
    Relative(u8, r32<BR>)
}

impl<Arg: Any + Copy, BR: BinRead<Args=(u8, Arg)>> BinRead for MultiReference<BR> {
    type Args = Arg;

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        let layout: ReferenceLayout = ReferenceLayout::read_options(reader, ro, ())?;

        if layout.is_relative != 0 {
            Ok(MultiReference::Relative(layout.ty, r32::read_options(reader, ro, (layout.ty, args))?))
        } else {
            println!("Warning: absolute reference at pos: {:X}", reader.seek(SeekFrom::Current(0))? - 4);
            let abs: a32<BR> = a32::read_options(reader, ro, (layout.ty, args))?;
            // todo: move into AbsPtr?
            let mut error = Some(||{});
            error = None;
            binread::error::assert(reader, abs.ptr() != 0u32, "abs.ptr() != 0", error)?;
            Ok(MultiReference::Absolute(layout.ty, abs))
        }
    }

    fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
        match self {
            MultiReference::Relative(ty, rel) => rel.after_parse(reader, ro, (*ty, args)),
            MultiReference::Absolute(ty, abs) => abs.after_parse(reader, ro, (*ty, args))
        }
    }
}

impl<BR: BinRead> Deref for MultiReference<BR> {
    type Target = BR;

    fn deref(&self) -> &Self::Target {
        match self {
            MultiReference::Relative(_, rel) => rel.deref(),
            MultiReference::Absolute(_, abs) => abs.deref()
        }
    }
}

impl<BR: BinRead> DerefMut for MultiReference<BR> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MultiReference::Relative(_, rel) => rel.deref_mut(),
            MultiReference::Absolute(_, abs) => abs.deref_mut()
        }
    }
}

pub struct Single<BR: BinRead>(BR);

impl<BR: BinRead> BinRead for Single<BR> {
    type Args = (u8, BR::Args);

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        let (ty, args) = args;
        // let mut error = Some(||{});
        // error = None;
        //binread::error::assert(reader, ty == 0, "ty == 0", error)?;
        if ty != 0 {
            println!("Unknown type encountered! type(0x{:X} != 0 in single type reference! pos: 0x{:X}, offset: 0x{:X}", ty,  reader.seek(SeekFrom::Current(0))?, ro.offset)
        }
        let mut temp = BR::read_options(reader, ro, args)?;
        // TODO: still need to figure out when and where this should be called
        temp.after_parse(reader, ro, args)?;
        Ok(Single(temp))
    }

    // fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
    //     self.0.after_parse(reader, ro, args.1)
    // }
}

pub struct Reference<BR: BinRead>(MultiReference<Single<BR>>);

impl<BR: BinRead> BinRead for Reference<BR> {
    type Args = BR::Args;

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        Ok(Reference(MultiReference::read_options(reader, ro, args)?))
    }

    fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
        self.0.after_parse(reader, ro, args)
    }
}

impl<BR: BinRead> Deref for Reference<BR> {
    type Target = BR;

    fn deref(&self) -> &Self::Target {
        &(self.0).0
    }
}

impl<BR: BinRead> DerefMut for Reference<BR> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut (self.0).0
    }
}

#[derive(BinRead, Debug)]
pub struct ReferenceLayout {
    pub is_relative: u8,
    pub ty: u8,
    pub padding: u16
}

#[derive(BinRead)]
pub enum SoundEncoding {
    SPcm8 = 0,
    SPcm16 = 1,
    DspAdpcm = 2
}

#[derive(BinRead, Debug)]
pub struct TypedId {
    ty: u8,
    id: [u8; 3] // u24
}


pub struct DerefTest<BR: BinRead>(pub BR);

impl<BR: BinRead> BinRead for DerefTest<BR> {
    type Args = BR::Args;

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        let mut ret = BR::read_options(reader, ro, args)?;
        ret.after_parse(reader, ro, args)?;
        Ok(DerefTest(ret))
    }
}

impl<BR: BinRead> Deref for DerefTest<BR> {
    type Target = BR;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}