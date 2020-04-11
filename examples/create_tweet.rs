mod common;

use egg_mode::media::{media_types, upload_media, set_metadata};
use egg_mode::tweet::DraftTweet;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    /// Text of the tweet
    text: String,
    /// Optionally attach media to tweet
    #[structopt(long, parse(from_os_str))]
    media: Option<PathBuf>,
    /// Optionally set alt-text for media
    #[structopt(long)]
    alt_text: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::from_args();
    let config = common::Config::load().await;

    let mut tweet = DraftTweet::new(args.text.clone());

    if let Some(path) = args.media {
        println!("Uploading media from '{}'", path.display());
        let typ = match path.extension().and_then(|os| os.to_str()).unwrap_or("") {
            "jpg" | "jpeg" => media_types::image_jpg(),
            "gif" => media_types::image_gif(),
            "png" => media_types::image_png(),
            "webp" => media_types::image_webp(),
            "mp4" => media_types::video_mp4(),
            _ => {
                eprintln!("Format not recognized, must be one of [jpg, jpeg, gif, png, webp, mp4]");
                std::process::exit(1);
            }
        };
        let bytes = std::fs::read(path)?;
        let handle = upload_media(&bytes, &typ, &config.token).await?;
        tweet.add_media(handle.id.clone());
        if let Some(alt) = &args.alt_text {
            set_metadata(&handle.id, alt, &config.token).await?;
        }
    }

    tweet.send(&config.token).await?;

    println!("Sent tweet: '{}'", args.text);

    Ok(())
}
