use std::path::PathBuf;

use argh::FromArgs;
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::pod::Pod;
use symphonia::core::{
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

pub const DEFAULT_RATE: u32 = 44100;
pub const DEFAULT_CHANNELS: u32 = 2;
pub const DEFAULT_VOLUME: f64 = 0.7;
pub const PI_2: f64 = std::f64::consts::PI + std::f64::consts::PI;
pub const CHAN_SIZE: usize = std::mem::size_of::<i16>();

#[derive(FromArgs)]
/// pwsb - pipewire soundboard
struct Args {
    /// path to file
    #[argh(option, short = 'f')]
    file: PathBuf,

    /// target node name to connect to
    #[argh(option, short = 't')]
    target: Option<String>,
}

pub fn main() -> Result<(), pw::Error> {
    let args: Args = argh::from_env();
    decode_file(args.file);
    return Ok(());

    pw::init();
    let mainloop = pw::main_loop::MainLoop::new(None)?;
    let context = pw::context::Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let data: f64 = 0.0;

    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::MEDIA_CATEGORY => "Capture",
    };
    if let Some(target) = args.target {
        props.insert(*pw::keys::TARGET_OBJECT, target);
    }
    let stream = pw::stream::Stream::new(&core, "audio-src", props)?;

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .process(|stream, acc| match stream.dequeue_buffer() {
            None => println!("No buffer received"),
            Some(buffer) => {
                noise(buffer, acc);
            }
        })
        .register()?;

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::S16LE);
    audio_info.set_rate(DEFAULT_RATE);
    audio_info.set_channels(DEFAULT_CHANNELS);

    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(pw::spa::pod::Object {
            type_: pw::spa::sys::SPA_TYPE_OBJECT_Format,
            id: pw::spa::sys::SPA_PARAM_EnumFormat,
            properties: audio_info.into(),
        }),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    stream.connect(
        spa::utils::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    mainloop.run();

    Ok(())
}

fn noise(mut buffer: pw::buffer::Buffer<'_>, acc: &mut f64) {
    let datas = buffer.datas_mut();
    let stride = CHAN_SIZE * DEFAULT_CHANNELS as usize;
    let data = &mut datas[0];
    let n_frames = if let Some(slice) = data.data() {
        let n_frames = slice.len() / stride;
        for i in 0..n_frames {
            *acc += PI_2 * 440.0 / DEFAULT_RATE as f64;
            if *acc >= PI_2 {
                *acc -= PI_2
            }
            let val = (f64::sin(*acc) * DEFAULT_VOLUME * 16767.0) as i16;
            for c in 0..DEFAULT_CHANNELS {
                let start = i * stride + (c as usize * CHAN_SIZE);
                let end = start + CHAN_SIZE;
                let chan = &mut slice[start..end];
                chan.copy_from_slice(&i16::to_le_bytes(val));
            }
        }
        n_frames
    } else {
        0
    };
    let chunk = data.chunk_mut();
    *chunk.offset_mut() = 0;
    *chunk.stride_mut() = stride as _;
    *chunk.size_mut() = (stride * n_frames) as _;
}

fn decode_file(path: PathBuf) {
    // Create a probe hint using the file's extension. [Optional]
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        match std::str::from_utf8(ext.as_encoded_bytes()) {
            Ok(ext) => {
                hint.with_extension(ext);
            }
            Err(err) => eprintln!("could not decode file extension: {}", err),
        };
    }

    // Open the media source.
    let src = std::fs::File::open(path).expect("failed to open media");

    // Create the media source stream.
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source.
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .expect("unsupported format");

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known (decodeable) codec.
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");

    // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("unsupported codec");

    // Store the track identifier, it will be used to filter packets.
    let track_id = track.id;

    // The decode loop.
    loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(err)) => {
                eprintln!("io error: {}", err);
                return;
            }
            Err(err) => {
                // A unrecoverable error occurred, halt decoding.
                panic!("{}", err);
            }
        };

        // Consume any new metadata that has been read since the last packet.
        while !format.metadata().is_latest() {
            // Pop the old head of the metadata queue.
            format.metadata().pop();

            // Consume the new metadata at the head of the metadata queue.
        }

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(_decoded) => {
                println!("decoded");
            }
            Err(symphonia::core::errors::Error::IoError(_)) => {
                // The packet failed to decode due to an IO error, skip the packet.
                continue;
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                continue;
            }
            Err(err) => {
                // An unrecoverable error occurred, halt decoding.
                panic!("{}", err);
            }
        }
    }
}
