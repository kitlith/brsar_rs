use binwrite::{BinWrite, WriterOption};
use std::io::{Result, Write};

// NOTE: I could create a trait akin to DekuUpdate, but I think i'm just gonna ad-hoc it for now.

// TODO: how to account for alignment?
pub trait BinLength {
    //const ALIGN: usize = 0;
    fn serialized_length(&self) -> usize;
}

pub trait ConstBinLength {
    //const ALIGN: usize = 0;
    const LENGTH: usize;
}

// region Standard ConstBinLength Implementations

impl ConstBinLength for () {
    const LENGTH: usize = 0;
}

impl ConstBinLength for u8 {
    const LENGTH: usize = 1;
}

impl ConstBinLength for i8 {
    const LENGTH: usize = 1;
}

impl ConstBinLength for u16 {
    const LENGTH: usize = 2;
}

impl ConstBinLength for i16 {
    const LENGTH: usize = 2;
}

impl ConstBinLength for u32 {
    const LENGTH: usize = 4;
}

impl ConstBinLength for i32 {
    const LENGTH: usize = 4;
}

impl ConstBinLength for f32 {
    const LENGTH: usize = 4;
}

impl ConstBinLength for u64 {
    const LENGTH: usize = 8;
}

impl ConstBinLength for i64 {
    const LENGTH: usize = 8;
}

impl ConstBinLength for f64 {
    const LENGTH: usize = 8;
}

impl ConstBinLength for u128 {
    const LENGTH: usize = 16;
}

// endregion

// region Standard BinLength Implementations
// TODO: char

impl<T: ConstBinLength> BinLength for T {
    //const ALIGN: usize = T::ALIGN;
    fn serialized_length(&self) -> usize {
        Self::LENGTH
    }
}

// TODO: specialization
// impl<T: ConstBinLength> BinLength for &[T] {
//     fn serialized_length(&self) -> usize {
//         T::length * self.len()
//     }
// }

impl<T: BinLength> BinLength for &[T] {
    //const ALIGN: usize = T::ALIGN;
    fn serialized_length(&self) -> usize {
        self.iter().map(BinLength::serialized_length).sum()
    }
}

// TODO: fixed size arrays, tuples

// TODO: specialization
// impl<T: ConstBinLength> BinLength for Vec<T> {
//     fn serialized_length(&self) -> usize {
//         T::length * self.len()
//     }
// }

impl<T: BinLength> BinLength for Vec<T> {
    //const ALIGN: usize = T::ALIGN;
    fn serialized_length(&self) -> usize {
        self.iter().map(BinLength::serialized_length).sum()
    }
}

// endregion

pub(crate) mod pool {
    use super::BinLength;
    use binwrite::{BinWrite, WriterOption};
    use std::io::{Result, Write};

    pub trait BinWriteLength<W: ?Sized>: BinLength {
        fn write_options(&self, writer: &mut W, options: &WriterOption) -> Result<()>;
    }

    impl<T, W: Write> BinWriteLength<W> for T where T: BinWrite + BinLength {
        fn write_options(&self, writer: &mut W, options: &WriterOption) -> Result<()> {
            self.write_options(writer, options)
        }
    }

    pub type PoolEntry<'a> = &'a dyn BinWriteLength<dyn Write>;

    impl<'a> BinWrite for PoolEntry<'a> {
        fn write_options<W: Write>(&self, writer: &mut W, options: &WriterOption) -> Result<()> {
            let writer = writer.into();
            self.write_options(writer, options)
        }
    }

    // TODO: Is there a better way to handle this?
    #[derive(BinWrite)]
    pub struct Pool<'a> {
        contents: Vec<PoolEntry<'a>>,
        #[binwrite(ignore)]
        cur_len: usize
    }

    impl<'a> Pool<'a> {
        pub fn new() -> Pool<'a> {
            Pool {
                contents: Vec::new(),
                cur_len: 0
            }
        }

        /// Clears pool
        pub fn clear(&mut self) {
            self.cur_len = 0;
            self.contents.clear();
        }

        /// Returns offset from start of pool
        // TODO: start alignment?
        pub fn push(&mut self, item: PoolEntry<'a>) -> usize {
            let tmp = self.cur_len;
            self.cur_len += item.serialized_length();
            self.contents.push(item);
            tmp
        }
    }

    impl<'a> BinLength for Pool<'a> {
        fn serialized_length(&self) -> usize {
            self.cur_len
        }
    }

    // #[test]
    // fn pool_test() {
    //     #[derive(BinRead)]
    //     struct Test<'a> {
    //         dataptr: super::RelPtr8<u8>,
    //         data: Pool<'a>
    //     };
    //
    //
    // }
}

pub use pool::Pool;

mod null_string {
    use binread::{BinRead, NullString};
    use binwrite::BinWrite;
    /// Wrapper around binread::NullString to add BinWrite support
    #[derive(BinRead, BinWrite, Clone, PartialEq, Default)]
    pub struct WriteNullString {
        #[binwrite(cstr, preprocessor(NullString::to_string))]
        inner: NullString
    }

    impl super::BinLength for WriteNullString {
        fn serialized_length(&self) -> usize {
            self.inner.0.len() + 1 // add one for null byte
        }
    }

    impl ToString for WriteNullString {
        fn to_string(&self) -> String {
            self.inner.to_string()
        }
    }

    impl std::ops::Deref for WriteNullString {
        type Target = Vec<u8>;

        fn deref(&self) -> &Self::Target {
            &self.inner.0
        }
    }

    impl std::fmt::Debug for WriteNullString {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "NullString({:?})", self.inner.to_string())
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use crate::common::binwrite_utils::BinLength;

        #[test]
        fn serialize_test() {
            let test = WriteNullString { inner: NullString(Vec::from(&b"Test!"[..])) };

            let expected = Vec::from(&b"Test!\0"[..]);

            let mut writer = std::io::Cursor::new(Vec::new());
            test.write(&mut writer).unwrap();

            let res = writer.into_inner();
            assert_eq!(res, expected);
            assert_eq!(res.len(), test.serialized_length());
        }

        #[derive(BinWrite)]
        struct Bad(u32);

        #[derive(BinWrite)]
        struct Good {
            inner: u32
        }

        #[test]
        fn test_bad() {
            let mut writer = std::io::Cursor::new(Vec::new());
            //Bad(0x01020304u32).write(&mut writer);
            Good {inner: 0x01020304}.write(&mut writer);
            assert_eq!(writer.into_inner(), vec![4, 3, 2, 1]);
        }
    }
}

pub use null_string::WriteNullString as NullString;
mod file_ptr {
    use binread::{BinRead, FilePtr, ReadOptions, BinResult};
    use binread::file_ptr::IntoSeekFrom;
    use binread::io::{Read, Seek};

    use binwrite::{BinWrite, WriterOption};
    use super::Pool;
    use std::io;

    use std::ops::{Deref, DerefMut};
    use crate::common::binwrite_utils::pool::BinWriteLength;
    use std::convert::{TryFrom, TryInto};

    /// A wrapper type for representing a layer of indirection from the start of a file.
    ///
    /// This wrapper created for the purpose of enabling binwrite serialization.
    /// NOTE: This does not serialize the value on its own.
    /// You need to add the value to a pool manually when needed.
    ///
    /// TODO: example
    ///
    /// See [`binread::FilePtr`](binread::FilePtr) for more information.
    pub struct RelPtr<Ptr: IntoSeekFrom, T: BinRead>(FilePtr<Ptr, T>);

    /// Type alias for 8-bit absolute pointers
    pub type RelPtr8<T> = RelPtr<u8, T>;
    /// Type alias for 16-bit absolute pointers
    pub type RelPtr16<T> = RelPtr<u16, T>;
    /// Type alias for 32-bit absolute pointers
    pub type RelPtr32<T> = RelPtr<u32, T>;
    /// Type alias for 64-bit absolute pointers
    pub type RelPtr64<T> = RelPtr<u64, T>;
    /// Type alias for 128-bit absolute pointers
    pub type RelPtr128<T> = RelPtr<u128, T>;

    impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> BinRead for RelPtr<Ptr, BR> {
        type Args = BR::Args;

        fn read_options<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<Self> {
            Ok(RelPtr(FilePtr::read_options(reader, ro, args)?))
        }

        fn after_parse<R: Read + Seek>(&mut self, reader: &mut R, ro: &ReadOptions, args: Self::Args) -> BinResult<()> {
            self.0.after_parse(reader, ro, args)?;
            // TODO: remove when binread bug is fixed
            self.0.value.as_mut().unwrap().after_parse(reader, ro, args)
        }
    }

    impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> RelPtr<Ptr, BR> {
        /// Consume the pointer and return the inner type
        ///
        /// # Panics
        ///
        /// Will panic if the file pointer hasn't been properly postprocessed
        pub fn into_inner(self) -> BR {
            self.0.into_inner()
        }

        /// Custom parser designed for use with the `parse_with` attribute ([example](binread::attribute#custom-parsers))
        /// that reads a [`FilePtr`](FilePtr) then immediately dereferences it into an owned value
        pub fn parse<R: Read + Seek>(
            reader: &mut R,
            ro: &ReadOptions,
            args: BR::Args
        ) -> BinResult<BR> {
            Ok(
                RelPtr::<Ptr, BR>(
                    FilePtr::<Ptr, _>::parse(reader, ro, args)?
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

    impl<Ptr, BR, E> RelPtr<Ptr, BR> where
        Ptr: IntoSeekFrom,
        BR: BinRead + BinWriteLength<dyn io::Write>,
        usize: TryInto<Ptr, Error = E>
    {
        pub fn add_to_pool<'a>(&'a mut self, pool: &mut Pool<'a>) -> Result<(), E> {
            self.0.ptr = pool.push(self.0.value.as_mut().unwrap()).try_into()?;
            Ok(())
        }
    }

    impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> Deref for RelPtr<Ptr, BR> {
        type Target = BR;

        fn deref(&self) -> &Self::Target {
            self.0.deref()
        }
    }

    impl<Ptr: BinRead<Args = ()> + IntoSeekFrom, BR: BinRead> DerefMut for RelPtr<Ptr, BR> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.0.deref_mut()
        }
    }

    impl<Ptr: BinRead<Args = ()> + BinWrite + IntoSeekFrom, BR: BinRead> BinWrite for RelPtr<Ptr, BR> {
        fn write_options<W: io::Write>(&self, writer: &mut W, options: &WriterOption) -> io::Result<()> {
            self.ptr().write_options(writer, options)

            // Values should be handled separately by Pools.
        }
    }
}

pub use file_ptr::*;