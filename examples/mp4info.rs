use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::path::Path;

use mp4::{Result, Mp4Reader, TrackType};

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

    let mut mp4 = Mp4Reader::new(reader);
    mp4.read(size)?;

    println!("File:");
    println!("  size:  {}", mp4.size());
    println!("  brands:     {:?} {:?}\n",
             mp4.ftyp.major_brand, mp4.ftyp.compatible_brands);

    if let Some(ref moov) = mp4.moov {
        println!("Movie:");
        println!("  version:       {:?}", moov.mvhd.version);
        println!("  creation time: {}",
                 creation_time(moov.mvhd.creation_time));
        println!("  duration:      {:?}", moov.mvhd.duration);
        println!("  timescale:     {:?}\n", moov.mvhd.timescale);

        println!("Found {} Tracks", moov.traks.len());
        for trak in moov.traks.iter() {
            let tkhd = trak.tkhd.as_ref().unwrap();
            println!("Track: {:?}", tkhd.track_id);
            println!("  flags:    {:?}", tkhd.flags);
            println!("  id:       {:?}", tkhd.track_id);
            println!("  duration: {:?}", tkhd.duration);
            if tkhd.width != 0 && tkhd.height != 0 {
                println!("    width:    {:?}", tkhd.width);
                println!("    height:   {:?}", tkhd.height);
            }
            if let Some(ref mdia) = trak.mdia {
                let hdlr = mdia.hdlr.as_ref().unwrap();
                let mdhd = mdia.mdhd.as_ref().unwrap();
                let stts = mdia
                    .minf
                    .as_ref()
                    .map(|m| m.stbl.as_ref().map(|s| s.stts.as_ref()).flatten())
                    .flatten();

                println!("  type:     {:?}",
                         get_handler_type(hdlr.handler_type.value.as_ref()));
                println!("  language: {:?}", mdhd.language);

                println!("  media:");
                if let Some(ref s) = stts {
                    println!("    sample count: {:?}", s.entries[0].sample_count);
                }
                println!("    timescale:    {:?}", mdhd.timescale);
                println!("    duration:     {:?} (media timescale units)",
                         mdhd.duration);
                println!("    duration:     {:?} (ms)",
                         get_duration_ms(mdhd.duration, mdhd.timescale));
                if get_handler_type(hdlr.handler_type.value.as_ref()) == TrackType::Video {
                    if let Some(ref s) = stts {
                        println!("    frame rate: (computed): {:?}",
                                 get_framerate(s.entries[0].sample_count,
                                               mdhd.duration, mdhd.timescale));
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_handler_type(handler: &str) -> TrackType {
    let mut typ: TrackType = TrackType::Unknown;
    match handler {
        "vide" => typ = TrackType::Video,
        "soun" => typ = TrackType::Audio,
        "meta" => typ = TrackType::Unknown,
        _ => (),
    }
    return typ;
}

fn get_duration_ms(duration: u64, timescale: u32) -> String {
    let ms = (duration as f64 / timescale as f64) * 1000.0;
    return format!("{:.2}", ms.floor());
}

fn get_framerate(sample_count: u32, duration: u64, timescale: u32) -> String {
    let sc = (sample_count as f64) * 1000.0;
    let ms = (duration as f64 / timescale as f64) * 1000.0;
    return format!("{:.2}", sc / ms.floor());
}

fn creation_time(creation_time: u64) -> u64 {
    // convert from MP4 epoch (1904-01-01) to Unix epoch (1970-01-01)
    if creation_time >= 2082844800 {
        creation_time - 2082844800
    } else {
        creation_time
    }
}
