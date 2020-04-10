mod common;

use egg_mode::media::{media_types, upload_media, MediaCategory};
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::from_args();
    let config = common::Config::load().await;

    let mut tweet = DraftTweet::new(args.text.clone());

    if let Some(path) = args.media {
        println!("Uploading media from '{}'", path.display());
        let bytes = std::fs::read(path)?;
        let typ = media_types::image_jpg();
        let cat = MediaCategory::Image;
        let handle = upload_media(&bytes, &typ, &cat, &config.token).await?;
        tweet.add_media(handle.id);
    }

    tweet.send(&config.token).await?;

    println!("Sent tweet: '{}'", args.text);

    Ok(())
}
