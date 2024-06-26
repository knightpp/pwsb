use anyhow::{Context, Result};
use std::path::PathBuf;

use symphonia::core::{
    audio::{RawSampleBuffer, SignalSpec},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

pub fn decode_file(path: PathBuf, spec: SignalSpec) -> Result<Vec<Vec<u8>>> {
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
    let src = std::fs::File::open(path).context("failed to open media")?;

    // Create the media source stream.
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source.
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .context("unsupported format")?;

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known (decodeable) codec.
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("no supported audio tracks")?;

    // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .context("unsupported codec")?;

    // Store the track identifier, it will be used to filter packets.
    let track_id = track.id;

    let mut result = Vec::new();
    // The decode loop.
    loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(_)) => return Ok(result),
            Err(err) => return Err(err.into()),
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
            Ok(decoded) => {
                // Create a raw sample buffer that matches the parameters of the decoded audio buffer.
                let mut sample_buf = RawSampleBuffer::<f32>::new(decoded.capacity() as u64, spec);

                // Copy the contents of the decoded audio buffer into the sample buffer whilst performing
                // any required conversions.
                sample_buf.copy_interleaved_ref(decoded);

                result.push(sample_buf.as_bytes().to_owned());
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
                return Err(err.into());
            }
        }
    }
}
