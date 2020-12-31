mod common;

use egg_mode::media::{get_status, media_types, set_metadata, upload_media, ProgressInfo};
use egg_mode::tweet::DraftTweet;

use std::io::{stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

use structopt::StructOpt;
use tokio::time::sleep;

#[derive(StructOpt)]
/// A simple CLI for uploading a tweet, optionally with media attched
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
        println!("Media uploaded");
        // Wait 60 seconds for processing
        print!("Waiting for media to finish processing..");
        stdout().flush()?;
        for ct in 0..=60u32 {
            match get_status(handle.id.clone(), &config.token).await?.progress {
                None | Some(ProgressInfo::Success) => {
                    println!("\nMedia sucessfully processed");
                    break;
                }
                Some(ProgressInfo::Pending(_)) | Some(ProgressInfo::InProgress(_)) => {
                    print!(".");
                    stdout().flush()?;
                    sleep(Duration::from_secs(1)).await;
                }
                Some(ProgressInfo::Failed(err)) => Err(err)?,
            }
            if ct == 60 {
                Err("Error: timeout")?
            }
        }
    }

    tweet.send(&config.token).await?;
    println!("Sent tweet: '{}'", args.text);
    Ok(())
}
