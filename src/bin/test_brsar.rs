use brsar_rs::brsar::BRSAR;
use std::path::PathBuf;
use structopt::StructOpt;
use std::error::Error;
use std::fs::File;
use binread::{BinReaderExt, BinRead};
use std::ops::Deref;

#[derive(Debug, StructOpt)]
#[structopt(name = "test_brsar")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let mut file = File::open(opt.input)?;

    let brsar: BRSAR = file.read_be()?;

    // for (idx, name) in (brsar.symbol.block.string_table.0).0.iter().enumerate() {
    //     println!("{}: {}:", idx, name.to_string());
    //     let trees = [
    //         ("sound", &brsar.symbol.block.sound_tree),
    //         ("player", &brsar.symbol.block.player_tree),
    //         ("group", &brsar.symbol.block.group_tree),
    //         ("bank", &brsar.symbol.block.bank_tree)
    //     ];
    //     for (tree_name, tree) in trees.into_iter() {
    //         if let Some(data) = tree.search(name) {
    //             if data.string_index as usize == idx {
    //                 println!("    {}: (string: {}, item: {})", tree_name, data.string_index, data.item_index);
    //             }
    //         }
    //     }
    // }

    for (idx, file) in brsar.info.block.file_table.deref().0.iter().enumerate() {
        let external = file.external_file.as_ref().map(|f| f.0.to_string());
        println!("{}: (external: {:?})", idx, external);
        for pos in file.file_positions.deref().0.iter() {
            let group_name = brsar.symbol.block.group_tree
                .get(pos.group_index as usize)
                .and_then(|group| (brsar.symbol.block.string_table.0).0.get(group.string_index as usize))
                .map(|name| name.to_string())
                .unwrap_or_else(|| pos.group_index.to_string());
            println!("    {}: {}", group_name, pos.item_index);
        }
    }

    Ok(())
}