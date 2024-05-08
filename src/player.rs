use anyhow::{Context, Result};
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::pod::Pod;

pub const DEFAULT_RATE: u32 = 44100;
pub const DEFAULT_CHANNELS: u32 = 2;

pub struct Terminate;

pub fn pipewire_play(target: Option<String>, samples: Vec<Vec<u8>>) -> Result<()> {
    let (pw_sender, pw_receiver) = pipewire::channel::channel::<Terminate>();

    let mainloop = pw::main_loop::MainLoop::new(None)?;
    let context = pw::context::Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let _receiver = pw_receiver.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |_| mainloop.quit()
    });

    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_ROLE => "Communication",
        // *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::AUDIO_CHANNELS => "2",
        *pw::keys::MEDIA_CATEGORY => "Capture",
    };
    if let Some(target) = target {
        props.insert(*pw::keys::TARGET_OBJECT, target);
    }
    let stream = pw::stream::Stream::new(&core, "audio-src", props)?;

    let mut sample_n = 0;
    let _listener = stream
        .add_local_listener::<()>()
        .process(move |stream, _| match stream.dequeue_buffer() {
            None => println!("No buffer received"),
            Some(mut buffer) => {
                let data = &mut buffer.datas_mut()[0];
                let slice = match data.data() {
                    Some(slice) => slice,
                    None => return,
                };

                if sample_n >= samples.len() {
                    pw_sender.send(Terminate).ok();
                    return;
                }

                for (dst, src) in slice.iter_mut().zip(samples[sample_n].iter()) {
                    *dst = *src;
                }

                let chunk = data.chunk_mut();
                *chunk.offset_mut() = 0;
                *chunk.stride_mut() = (std::mem::size_of::<f32>() * DEFAULT_CHANNELS as usize) as _;
                *chunk.size_mut() = samples[sample_n].len() as _;

                sample_n += 1;
            }
        })
        .register()?;

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
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
    .context("serialize pod info")?
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).context("parse pod info")?];

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
