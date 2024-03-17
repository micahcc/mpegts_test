use clap::Parser;
use log::error;
use log::info;
use std::fs::File;
use std::io::prelude::*;

mod image;
use image::{generate_image, PixFmt};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Where to send, can be file:///path udp://ip:port or - for stdout
    #[arg(short, long)]
    target: String,

    /// Number of frames to produce.
    #[arg(short, long)]
    num_frames: u32,

    /// Image size width
    #[arg(short, long)]
    xsize: u32,

    /// Image size height
    #[arg(short, long)]
    ysize: u32,
}

pub fn send_to_file(
    num_frames: u32,
    width: u32,
    height: u32,
    filename: &str,
) -> anyhow::Result<()> {
    let mut fd = File::create(filename)?;
    let mut my_h264_writer = less_avc::H264Writer::new(fd).unwrap();
    for i in 0..num_frames {
        let input_yuv = generate_image(i as u32, &PixFmt::Rgb8, width, height).unwrap();
        let frame_view = input_yuv.view();
        my_h264_writer.write(&frame_view).unwrap();
    }

    return Ok(());
}

pub fn send_to_udp(num_frames: u32, ip: &str, port: i32) -> anyhow::Result<()> {
    return Ok(());
}

pub fn send_to_stdout(num_frames: u32) -> anyhow::Result<()> {
    return Ok(());
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    match args.target {
        t if t.starts_with("file://") => {
            send_to_file(args.num_frames, args.xsize, args.ysize, &t[7..])?;
        }
        t if t.starts_with("udp://") => {
            let t = &t[6..];
            let spl: Vec<&str> = t.split(':').collect();
            if spl.len() == 2 {
                send_to_udp(args.num_frames, spl[0], spl[1].parse()?)?;
            } else {
                error!("UDP must have exactly one :");
                return Err(anyhow::anyhow!("UDP must have exactly one :"));
            }
        }
        t if t == "-" => {
            send_to_stdout(args.num_frames)?;
        }
        t => {
            send_to_file(args.num_frames, args.xsize, args.ysize, &t)?;
        }
    }

    return Ok(());
}
