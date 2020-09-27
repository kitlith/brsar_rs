#![allow(unused)]

// TODO: NullString
pub use binread::NullString;
use std::marker::PhantomData;
use std::convert::TryFrom;

use binread::{BinRead, BinReaderExt, FilePtr32, BinResult, ReadOptions};
use binread::io::{Read, Seek};
use std::any::Any;

pub mod binread_utils {
    use binread::{BinRead, BinReaderExt, FilePtr8, BinResult, ReadOptions, FilePtr};

    use binread::io::{Cursor, Read, Seek};
    use binread::file_ptr::IntoSeekFrom;

    struct Relative<T: BinRead>(T);

    impl<BR: BinRead> BinRead for Relative<BR> {
        type Args = (u64, BR::Args);

        fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
            let (offset, args) = args;
            let mut temp_options = ro.clone();
            temp_options.offset = offset;

            Ok(Relative(BR::read_options(reader, &temp_options, args)?))
        }

        fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
            let (offset, args) = args;
            let mut temp_options = ro.clone();
            temp_options.offset = offset;

            self.0.after_parse(reader, &temp_options, args)
        }
    }

    /// A wrapper type for representing a layer of indirection from the start of a file.
    ///
    /// This wrapper will always read from an offset from 0, but will pass on any existing offset to
    /// the type it is wrapping.
    ///
    /// TODO: example
    ///
    /// See [`binread::FilePtr`](binread::FilePtr) for more information.
    pub struct AbsPtr<Ptr: BinRead<Args = ()> + IntoSeekFrom, T: BinRead>(FilePtr<Ptr, Relative<T>>);

    /// Type alias for 8-bit absolute pointers
    pub type AbsPtr8<T> = AbsPtr<u8, T>;
    /// Type alias for 16-bit absolute pointers
    pub type AbsPtr16<T> = AbsPtr<u16, T>;
    /// Type alias for 32-bit absolute pointers
    pub type AbsPtr32<T> = AbsPtr<u32, T>;
    /// Type alias for 64-bit absolute pointers
    pub type AbsPtr64<T> = AbsPtr<u64, T>;
    /// Type alias for 128-bit absolute pointers
    pub type AbsPtr128<T> = AbsPtr<u128, T>;

    impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> BinRead for AbsPtr<Ptr, BR> {
        type Args = BR::Args;

        fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
            let mut temp_options = ro.clone();
            temp_options.offset = 0;

            Ok(AbsPtr(FilePtr::read_options(reader, &temp_options, (ro.offset, args))?))
        }

        fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
            let mut temp_options = ro.clone();
            temp_options.offset = 0;

            self.0.after_parse(reader, &temp_options, (ro.offset, args))
        }
    }

    impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> AbsPtr<Ptr, BR> {
        /// Consume the pointer and return the inner type
	    ///
	    /// # Panics
	    ///
	    /// Will panic if the file pointer hasn't been properly postprocessed
        pub fn into_inner(self) -> BR {
            self.0.into_inner().0
        }

        /// Custom parser designed for use with the `parse_with` attribute ([example](binread::attribute#custom-parsers))
	    /// that reads a [`FilePtr`](FilePtr) then immediately dereferences it into an owned value
        pub fn parse<R: Read + Seek>(
            reader: &mut R,
            ro: &ReadOptions,
            args: BR::Args
        ) -> BinResult<BR> {
            let mut temp_options = ro.clone();
            temp_options.offset = 0;

            Ok(
                AbsPtr::<Ptr, _>(
                    FilePtr::<Ptr, _>::parse(reader, &temp_options, (ro.offset, args))?
                ).into_inner()
            )
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[derive(BinRead)]
        struct Inner(#[br(deref_now)] FilePtr8<u8>);

        #[derive(BinRead)]
        struct Outer(#[br(offset = 4, deref_now)] AbsPtr8<Inner>);

        #[test]
        fn binread_absptr_offset_passthrough() {
            let outer: Outer = Cursor::new([0x01u8, 0x02, 0xFE, 0x21, 0x22, 0x23, 0xFF]).read_be().unwrap();

            // this would be 0xFE if the offset wasn't passed through
            assert_eq!(outer.0.into_inner().0.into_inner(), 0xFF);
        }

        #[derive(BinRead)]
        struct Test {
            #[br(offset = 4)]
            a: FilePtr8<u8>,
            b: FilePtr8<u8>
        }

        #[test]
        fn does_binread_set_offset_only_for_current_item() {
            let test: Test = Cursor::new([0x00, 0x04, 0x20, 0xFE, 0xFF]).read_be().unwrap();

            assert_eq!(test.a.into_inner(), 0xFF);
            assert_eq!(test.b.into_inner(), 0xFF);
        }
    }
}

#[allow(non_camel_case_types)]
pub type r32<T> = FilePtr32<T>;

#[allow(non_camel_case_types)]
pub type a32<T> = binread_utils::AbsPtr32<T>;

#[allow(non_camel_case_types)]
pub type s32 = i32;

// TODO: parsing from bytes
pub struct VarLen(u128);

// TODO: pass args
#[derive(BinRead)]
pub struct BlockPtr<BR: BinRead<Args=()>> {
    #[br(restore_position)]
    offset: u32,
    // the idea is that this makes r32 act the way we want
    // TODO: may need to do this somewhere else
    #[br(offset = offset as u64 + 8)]
    pub block: a32<BR>,
    pub len: u32
}

#[derive(BinRead, PartialEq, Debug)]
#[br(big)]
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
pub struct Table<T>(Vec<T>);

impl<Arg: Copy + 'static, BR: BinRead<Args=Arg>> BinRead for Table<BR> {
    type Args = Arg;

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        let mut temp_options = ro.clone();
        temp_options.count = Some(u32::read_options(reader, ro, ())? as usize);
        Ok(Table(Vec::read_options(reader, &temp_options, args)?))
    }

    fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
        self.0.after_parse(reader, ro, args)
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
            Ok(MultiReference::Absolute(layout.ty, a32::read_options(reader, ro, (layout.ty, args))?))
        }
    }

    fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
        match self {
            MultiReference::Relative(ty, rel) => rel.after_parse(reader, ro, (*ty, args)),
            MultiReference::Absolute(ty, abs) => abs.after_parse(reader, ro, (*ty, args))
        }
    }
}

pub struct Single<BR: BinRead>(BR);

impl<BR: BinRead> BinRead for Single<BR> {
    type Args = (u8, BR::Args);

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        let (ty, args) = args;
        let mut error = Some(||{});
        error = None;
        binread::error::assert(reader, ty == 1, "ty == 1", error)?;
        Ok(Single(BR::read_options(reader, ro, args)?))
    }

    fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
        self.0.after_parse(reader, ro, args.1)
    }
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

#[derive(BinRead)]
pub struct ReferenceLayout {
    is_relative: u8,
    ty: u8,
    padding: u16
}

#[derive(BinRead)]
pub enum SoundEncoding {
    SPcm8 = 0,
    SPcm16 = 1,
    DspAdpcm = 2
}

#[derive(BinRead)]
pub struct TypedId {
    ty: u8,
    id: [u8; 3] // u24
}