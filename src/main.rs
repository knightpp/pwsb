use std::{error::Error, path::PathBuf};

use argh::FromArgs;
use pipewire as pw;

mod player;
use player::*;

mod decoder;
use decoder::*;
use symphonia::core::audio::{Layout, SignalSpec};

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

pub fn main() -> Result<(), Box<dyn Error>> {
    pw::init();

    let args: Args = argh::from_env();
    let audio = decode_file(
        args.file,
        SignalSpec::new_with_layout(DEFAULT_RATE, Layout::Stereo),
    )
    .expect("decode file");

    pipewire_play(args.target, audio)?;

    Ok(())
}
