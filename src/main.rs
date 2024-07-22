use cc_streaming::{cli::ARGS, decoder::Decoder, palette::Palette};
use ffmpeg_next::format::input;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    ffmpeg_next::init().unwrap();

    let mut ictx = input(&ARGS.input)?;
    let vid_stream = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .unwrap();
    let aud_stream = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Audio)
        .unwrap();

    let mut decoder = Decoder::new(vid_stream, aud_stream)?;

    let (mut imgs, samples) = decoder.decode_all(&mut ictx)?;
    drop(ictx);

    if let Ok(dir) = std::fs::read_dir("frames") {
        let mut is_correct_dir = true;
        for entry in dir {
            if let Some(ext) = entry?.path().extension() {
                if ext != "png" {
                    is_correct_dir = false;
                    break;
                }
            }
        }
        if is_correct_dir {
            std::fs::remove_dir_all("frames")?;
        } else {
            panic!("erm!")
        }
    }
    std::fs::create_dir("frames")?;

    for (i, val) in imgs.iter_mut().enumerate() {
        val.save(format!("frames/frame_{:04}_orig.png", i)).unwrap();
        let palette = Palette::new(16, val);

        palette.apply(val);

        let mut palette_img = image::RgbImage::new(4, 4);
        for (i, pix) in palette_img.pixels_mut().enumerate() {
            *pix = palette.palette()[i]
        }
        palette_img
            .save(format!("frames/frame_{:04}_palette.png", i))
            .unwrap();
        val.save(format!("frames/frame_{:04}.png", i)).unwrap();
    }

    println!("frames: {}", imgs.len());
    println!("samples: {}", samples.len());

    Ok(())
}
