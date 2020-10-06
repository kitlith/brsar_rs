use brsar_rs::brsar::BRSAR;
use brsar_rs::brsar::block::info::{SoundInfo, FileInfo, SoundType};
use binread::{BinReaderExt, BinRead};

use std::path::PathBuf;
use structopt::StructOpt;
use std::fs::File;
use std::ops::Deref;
use std::error::Error;
use std::io::Write;
use std::os::unix::fs::FileExt;

#[derive(Debug, StructOpt)]
#[structopt(name = "extract_files")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    #[structopt(parse(from_os_str), default_value="output/", short="o", long="output")]
    output_folder: PathBuf
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let mut input_file = File::open(opt.input)?;

    let brsar: BRSAR = input_file.read_be()?;
    let symbol = brsar.symbol.block.deref();
    let info = brsar.info.block.deref();

    for (sound_idx, sound) in info.sound_table.deref().0.iter().enumerate() {
        //println!("{}", sound_idx);
        let output_ext = match sound.sound_type {
            SoundType::Sequence => "brseq",
            SoundType::Stream => "brstm",
            SoundType::Wave => "brwav",
            _ => "bin"
        };

        let filename = (symbol.string_table.0).0[sound.string_id as usize].to_string();
        let mut file_path = opt.output_folder.join(&filename);
        file_path.set_extension(output_ext);

        let file: &FileInfo = info.file_table.deref().0[sound.file_id as usize].deref();

        if let Some(pos) = file.file_positions.0.get(0) {
            let group = &info.group_table.0[pos.group_index as usize];

            let item = &(group.entries.0)[pos.item_index as usize];

            let mut bytes = vec![0; item.file_size as usize];
            let pos = group.file_base + item.file_offset.val;
            println!("{}: (base: 0x{:X}, offset: 0x{:X}) @ 0x{:X}", filename, group.file_base, item.file_offset.val, item.file_offset.pos);
            if let Ok(_) = input_file.read_exact_at(&mut bytes, pos as u64) {
                File::create(file_path).unwrap().write_all(&bytes).unwrap();
            } else {
                println!("Failed to read '{}' from pos: {:X}, size: {:X}", filename, pos, bytes.len());
            }
        }
    }

    Ok(())
}