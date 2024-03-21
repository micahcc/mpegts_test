use clap::Parser;
use log::error;
use log::info;
use mpeg2ts::es::StreamId;
use mpeg2ts::pes::PesHeader;
use mpeg2ts::time::Timestamp;
use mpeg2ts::ts::{
    payload, AdaptationExtensionField, AdaptationField, ContinuityCounter, Pid, ReadTsPacket,
    TransportScramblingControl, TsHeader, TsPacket, TsPacketReader, TsPacketWriter, TsPayload,
    WriteTsPacket,
};
use std::fs::File;
use std::io::prelude::*;

mod image;
use image::{generate_image, PixFmt};

struct TsStreamer<W> {
    writer: TsPacketWriter<W>,
    counter: ContinuityCounter,

    input_frame_number: u64,
    input_frame_unix_micros: u64,
    frame_rate: f64,
    stream_id: StreamId,

    output_frame_number: u64,
}

impl<W: std::io::Write> TsStreamer<W> {
    pub fn set_next_frame_meta(&mut self, frame_number: u64, unix_timestamp_micros: u64) {
        self.input_frame_number = frame_number;
        self.input_frame_unix_micros = unix_timestamp_micros;
    }
}

impl<W: std::io::Write> TsStreamer<W> {
    pub fn new_stream(stream: W) -> anyhow::Result<Self> {
        let mut writer = TsPacketWriter::<W>::new(stream);
        let stream_id = StreamId::new_video(0)?;
        Ok(Self {
            writer,
            counter: ContinuityCounter::new(),
            input_frame_number: 0,
            input_frame_unix_micros: 0,
            output_frame_number: 0,
            stream_id,
            frame_rate: 30.0,
        })
    }
}

impl<W: std::io::Write> std::io::Write for TsStreamer<W> {
    fn write(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        // write a PES packet

        // make a packet
        // copy data into packet
        info!(
            "Input count: {}, Output Frame Count: {}",
            self.input_frame_number, self.output_frame_number
        );

        let pts = 0; // todo
        let data = [0; 5];
        let packet = TsPacket {
            header: TsHeader {
                transport_error_indicator: false,
                transport_priority: false,
                pid: Pid::new(32).expect("allocate pid"),
                continuity_counter: ContinuityCounter::from_u8(self.counter.as_u8())
                    .expect("counter"),
                transport_scrambling_control: TransportScramblingControl::NotScrambled,
            },
            adaptation_field: Some(AdaptationField {
                discontinuity_indicator: false,
                random_access_indicator: true,
                es_priority_indicator: false,
                pcr: None,
                opcr: None,
                splice_countdown: None,
                transport_private_data: vec![],
                extension: None,
            }),
            payload: Some(TsPayload::Pes(payload::Pes {
                header: PesHeader {
                    copyright: false,
                    data_alignment_indicator: true,
                    escr: None,
                    original_or_copy: true,
                    stream_id: self.stream_id,
                    priority: false,
                    dts: None,
                    pts: Some(Timestamp::new(pts).expect("make timestamp")),
                },
                pes_packet_len: 0,
                data: payload::Bytes::new(&data).expect("make bytes"),
            })),
        };

        self.writer.write_ts_packet(&packet).expect("write packet");
        self.counter.increment();
        return Ok(0);
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        // not clear if we need to pass the flush all the way down
        // that would require adding a flush method or mut_stream
        // to TsPacketWriter
        return Ok(());
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Where to send, can be file:///path udp://ip:port or - for stdout
    #[arg(short, long)]
    target: String,

    /// Send a mpeg-ts stream instead of a raw x264 stream.
    #[arg(long)]
    mpegts: bool,

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

pub fn send_mpegts_to_file(
    num_frames: u32,
    width: u32,
    height: u32,
    filename: &str,
) -> anyhow::Result<()> {
    let mut fd = File::create(filename)?;
    let mut ts_streamer = TsStreamer::new_stream(fd)?;
    let mut my_h264_writer = less_avc::H264Writer::new(ts_streamer).unwrap();

    for i in 0..num_frames {
        let input_yuv = generate_image(i as u32, &PixFmt::Rgb8, width, height).unwrap();
        let frame_view = input_yuv.view();
        my_h264_writer.write(&frame_view).unwrap();
    }

    return Ok(());
}

pub fn send_x264_to_file(
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
            if args.mpegts {
                send_mpegts_to_file(args.num_frames, args.xsize, args.ysize, &t[7..])?;
            } else {
                send_x264_to_file(args.num_frames, args.xsize, args.ysize, &t[7..])?;
            }
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
            if args.mpegts {
                send_mpegts_to_file(args.num_frames, args.xsize, args.ysize, &t)?;
            } else {
                send_x264_to_file(args.num_frames, args.xsize, args.ysize, &t)?;
            }
        }
    }

    return Ok(());
}
