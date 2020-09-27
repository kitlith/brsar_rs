use brsar_rs::common::*;

use binread::BinReaderExt;
use structopt::StructOpt;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;

#[derive(Debug, StructOpt)]
#[structopt(name = "test_headers")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let mut file = File::open(opt.input)?;

    let file: GenericFile = file.read_be()?;

    println!("magic: {}", String::from_utf8(file.header.magic.to_vec())?);
    println!("endian: {:?}", file.header.endian);
    println!("version: {:x}", file.header.version);
    println!("file_size: {:x}, header_size: {:x}, block_count: {:x}", file.header.file_size, file.header.header_size, file.header.block_count);

    for block_ptr in file.blocks {
        let block = block_ptr.block.into_inner();
        println!("{}(size: {:x})", String::from_utf8(block.header.magic.to_vec())?, block.header.size);
        if block.header.size != block_ptr.len {
            println!("header size ({:x}) differs from block header size ({:x}", block_ptr.len, block.header.size);
        }
    }

    Ok(())
}