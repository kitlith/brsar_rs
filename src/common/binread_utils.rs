use binread::{BinRead, BinReaderExt, FilePtr8, BinResult, ReadOptions, FilePtr};

use binread::io::{Cursor, Read, Seek};
use std::io;
use binread::file_ptr::IntoSeekFrom;
use std::ops::{Deref, DerefMut};
use binwrite::{BinWrite, WriterOption};
use std::convert::TryInto;

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
/// NOTE: This integrates with BinWrite, but does not serialize the value on its own.
/// You need to add the value to a pool manually when needed.
///
/// TODO: example
///
/// See [`binread::FilePtr`](binread::FilePtr) for more information.
pub struct AbsPtr<Ptr: IntoSeekFrom, T: BinRead>(FilePtr<Ptr, Relative<T>>);

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
            AbsPtr::<Ptr, BR>(
                FilePtr::<Ptr, _>::parse(reader, &temp_options, (ro.offset, args))?
            ).into_inner()
        )
    }

    pub fn ptr(&self) -> Ptr {
        self.0.ptr
    }
    pub fn set_ptr(&mut self, new: Ptr) {
        self.0.ptr = new;
    }
}

use crate::common::binwrite_utils::pool::BinWriteLength;

impl<Ptr, BR, E> AbsPtr<Ptr, BR> where
    Ptr: IntoSeekFrom,
    BR: BinRead + BinWriteLength<dyn io::Write>,
    usize: TryInto<Ptr, Error = E>
{
    pub fn add_to_pool<'a>(&'a mut self, pool: &mut super::Pool<'a>) -> Result<(), E> {
        self.0.ptr = pool.push(&mut self.0.value.as_mut().unwrap().0).try_into()?;
        Ok(())
    }
}

impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> Deref for AbsPtr<Ptr, BR> {
    type Target = BR;

    fn deref(&self) -> &Self::Target {
        &self.0.deref().0
    }
}

impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> DerefMut for AbsPtr<Ptr, BR> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.deref_mut().0
    }
}

impl<Ptr: BinRead<Args = ()> + BinWrite + IntoSeekFrom, BR: BinRead> BinWrite for AbsPtr<Ptr, BR> {
    fn write_options<W: io::Write>(&self, writer: &mut W, options: &WriterOption) -> io::Result<()> {
        self.ptr().write_options(writer, options)

        // Values should be handled separately by Pools.
    }
}

pub struct CurPos(pub u64);

impl BinRead for CurPos {
    type Args = ();

    fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
        Ok(CurPos(reader.seek(io::SeekFrom::Current(0))?))
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

    #[derive(BinRead)]
    struct Wrapper<BR: BinRead<Args=()>>(BR);

    #[test]
    //#[should_panic(expected = "Deref'd FilePtr before reading (make sure to use FilePtr::after_parse first)")] // TODO
    fn file_ptr() {
        let test: FilePtr8<u8> = Cursor::new([0x01, 0xFF]).read_be().unwrap();

        // FAILS: after_parse was never called.
        assert_eq!(*test, 0xFF);
    }

    #[test]
    fn file_ptr_wrapper() {
        let test: Wrapper<FilePtr8<u8>> = Cursor::new([0x01, 0xFF]).read_be().unwrap();

        // PASSES: after_parse *was* called.
        assert_eq!(*test.0, 0xFF);
    }

    #[test]
    #[should_panic(expected = "Deref'd FilePtr before reading (make sure to use FilePtr::after_parse first)")] // TODO
    fn nested_file_ptr() {
        let test: Wrapper<FilePtr8<FilePtr8<u8>>> = Cursor::new([0x01, 0x02, 0xFF]).read_be().unwrap();

        // FAILS: after_parse was never called.
        assert_eq!(**test.0, 0xFF);
    }

    #[test]
    fn nested_file_ptr_wrapper() {
        let test: Wrapper<FilePtr8<Wrapper<FilePtr8<u8>>>> = Cursor::new([0x01, 0x02, 0xFF]).read_be().unwrap();

        // PASSES: after_parse *was* called.
        assert_eq!(*test.0.0, 0xFF);
    }

    #[derive(BinRead)]
    struct Try<BR: BinRead<Args=()>>(
        #[br(try)]
        Option<BR>
    );

    #[test]
    //#[should_panic(expected = "Deref'd FilePtr before reading (make sure to use FilePtr::after_parse first)")]
    fn try_file_ptr() {
        let test: Try<FilePtr8<u8>> = Cursor::new([0x01, 0xFF]).read_be().unwrap();

        // FAILS: after_parse was never called.
        assert_eq!(*test.0.unwrap(), 0xFF)
    }

    #[test]
    fn try_file_ptr_wrapper() {
        let test: Try<Wrapper<FilePtr8<u8>>> = Cursor::new([0x01, 0xFF]).read_be().unwrap();

        assert_eq!(*test.0.unwrap().0, 0xFF)
    }

    #[derive(BinRead)]
    struct CurPosTest {
        initial: CurPos,
        a: u32,
        mid: CurPos,
        b: u64,
        end: CurPos
    }
     #[test]
    fn test_curpos() {
         fn verify_curpos(test: CurPosTest, expected_initial: u64) {
             assert_eq!(test.initial.0, expected_initial);
             assert_eq!(test.mid.0, expected_initial + 4);
             assert_eq!(test.end.0, expected_initial + 4 + 8);
         }

         let test: CurPosTest = Cursor::new([0x00u8; 12]).read_be().unwrap();
         verify_curpos(test, 0);

         let test: (u32, CurPosTest) = Cursor::new([0x00u8; 16]).read_be().unwrap();
         verify_curpos(test.1, 4);
     }
}