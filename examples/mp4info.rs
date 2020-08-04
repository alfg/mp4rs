use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::path::Path;

use mp4::{Mp4Track, Result, TrackType};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: mp4info <filename>");
        std::process::exit(1);
    }

    if let Err(err) = info(&args[1]) {
        let _ = writeln!(io::stderr(), "{}", err);
    }
}

fn info<P: AsRef<Path>>(filename: &P) -> Result<()> {
    let f = File::open(filename)?;
    let size = f.metadata()?.len();
    let reader = BufReader::new(f);

    let mp4 = mp4::Mp4Reader::read_header(reader, size)?;

    println!("Metadata:");
    println!("  size            : {}", mp4.size());
    println!("  major_brand     : {}", mp4.major_brand());
    let mut compatible_brands = String::new();
    for brand in mp4.compatible_brands().iter() {
        compatible_brands.push_str(&brand.to_string());
        compatible_brands.push_str(",");
    }
    println!("  compatible_brands: {}", compatible_brands);
    println!(
        "Duration: {}, timescale: {}",
        mp4.duration(),
        mp4.timescale()
    );

    for track in mp4.tracks().iter() {
        let media_info = match track.track_type()? {
            TrackType::Video => video_info(track),
            TrackType::Audio => audio_info(track),
        };
        println!(
            "  Track: #{}({}) {}: {} ({:?}), {}",
            track.track_id(),
            track.language(),
            track.track_type()?,
            track.media_type()?,
            track.box_type(),
            media_info
        );
    }

    Ok(())
}

fn video_info(track: &Mp4Track) -> String {
    format!(
        "{}x{}, {} kb/s, {:.2} fps",
        track.width(),
        track.height(),
        track.bitrate() / 1000,
        track.frame_rate_f64()
    )
}

fn audio_info(track: &Mp4Track) -> String {
    let ch = match track.channel_count() {
        1 => String::from("mono"),
        2 => String::from("stereo"),
        n => format!("{}-ch", n),
    };
    format!(
        "{} Hz, {}, {} kb/s",
        track.sample_rate(),
        ch,
        track.bitrate() / 1000
    )
}
