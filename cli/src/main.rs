#[macro_use]
extern crate log;

use std::path::PathBuf;

use clap::Parser;
use emojify_tg_sticker::EMOJI_SIZE;
use image::ImageReader;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    image_path: PathBuf,
    output_dir: Option<PathBuf>,
}

fn main() -> color_eyre::eyre::Result<()> {
    let _ = color_eyre::install();
    pretty_env_logger::try_init()?;

    let args = Args::parse();
    let input_image = ImageReader::open(&args.image_path)?.decode()?;

    let result = emojify_tg_sticker::transform(&input_image)?;
    if result.cols * EMOJI_SIZE < input_image.width() {
        warn!(
            "input image width was truncated to {}px",
            result.cols * EMOJI_SIZE
        );
    }
    if result.rows() * EMOJI_SIZE < input_image.height() {
        warn!(
            "input image height was truncated to {}px",
            result.rows() * EMOJI_SIZE
        );
    }

    let outdir = args.output_dir.unwrap_or_else(|| {
        warn!("output directory was not specified; defaulting to input filename sans extension");
        args.image_path.with_extension("")
    });
    info!("outputting images to {:?}", outdir);
    let _ = std::fs::create_dir_all(&outdir);

    for (idx, emoji) in result.emojis.iter().enumerate() {
        let outpath = outdir.join(format!("{:0>2}.png", idx + 1));
        emoji.save(outpath)?;
    }

    Ok(())
}
