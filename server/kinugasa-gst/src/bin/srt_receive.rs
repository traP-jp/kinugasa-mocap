use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use gst::prelude::*;

fn main() -> Result<()> {
    let config = parse_args(std::env::args().skip(1))?;
    run_receiver(&config)?;
    Ok(())
}

struct Config {
    url: String,
    stream_id: String,
    output: PathBuf,
    duration: Duration,
    passphrase: String,
    pbkeylen: u32,
}

fn run_receiver(config: &Config) -> Result<()> {
    gst::init().context("failed to initialize GStreamer")?;
    kinugasa_gst::srt::register(None).context("failed to register kinugasasrtserversrc")?;

    let output = config
        .output
        .to_str()
        .context("--output must be valid UTF-8 for GStreamer filesink")?;

    let source = gst::ElementFactory::make("kinugasasrtserversrc")
        .property("uri", &config.url)
        .property("stream-ids", &config.stream_id)
        .property("passphrase", &config.passphrase)
        .property("pbkeylen", config.pbkeylen)
        .build()
        .context("failed to create kinugasasrtserversrc")?;
    let sink = gst::ElementFactory::make("filesink")
        .property("location", output)
        .build()
        .context("failed to create filesink")?;

    let pipeline = gst::Pipeline::new();
    pipeline
        .add_many([&source, &sink])
        .context("failed to add elements to pipeline")?;
    gst::Element::link_many([&source, &sink]).context("failed to link SRT source to filesink")?;

    pipeline
        .set_state(gst::State::Playing)
        .map_err(|_| anyhow::anyhow!("failed to start GStreamer pipeline"))?;

    let result = wait_for_completion(&pipeline, config.duration);

    let stop_result = pipeline
        .set_state(gst::State::Null)
        .map(|_| ())
        .map_err(|_| anyhow::anyhow!("failed to stop GStreamer pipeline"));

    result.and(stop_result)
}

fn wait_for_completion(pipeline: &gst::Pipeline, duration: Duration) -> Result<()> {
    let bus = pipeline.bus().context("pipeline has no bus")?;
    let timeout = gst::ClockTime::try_from(duration).context("--duration is too large")?;
    let message = bus.timed_pop_filtered(
        Some(timeout),
        &[gst::MessageType::Error, gst::MessageType::Eos],
    );

    let Some(message) = message else {
        return Ok(());
    };

    match message.view() {
        gst::MessageView::Error(error) => {
            let source = error
                .src()
                .map(|source| source.path_string().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            bail!(
                "GStreamer error from {source}: {} ({:?})",
                error.error(),
                error.debug()
            );
        }
        gst::MessageView::Eos(_) => Ok(()),
        _ => Ok(()),
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Config> {
    let mut url = None;
    let mut stream_id = None;
    let mut output = None;
    let mut duration = Duration::from_secs(5);
    let mut passphrase = String::new();
    let mut pbkeylen = 32;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--url" => url = Some(next_value(&mut args, "--url")?),
            "--stream-id" => stream_id = Some(next_value(&mut args, "--stream-id")?),
            "--output" => output = Some(PathBuf::from(next_value(&mut args, "--output")?)),
            "--passphrase" => passphrase = next_value(&mut args, "--passphrase")?,
            "--pbkeylen" => {
                pbkeylen = next_value(&mut args, "--pbkeylen")?
                    .parse()
                    .context("--pbkeylen must be 0, 16, 24, or 32")?;
                ensure_pbkeylen(pbkeylen)?;
            }
            "--duration" => {
                duration = Duration::from_secs_f64(
                    next_value(&mut args, "--duration")?
                        .parse()
                        .context("--duration must be seconds")?,
                );
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }

    ensure_pbkeylen(pbkeylen)?;

    Ok(Config {
        url: url.context("--url is required")?,
        stream_id: stream_id.context("--stream-id is required")?,
        output: output.context("--output is required")?,
        duration,
        passphrase,
        pbkeylen,
    })
}

fn ensure_pbkeylen(pbkeylen: u32) -> Result<()> {
    match pbkeylen {
        0 | 16 | 24 | 32 => Ok(()),
        _ => bail!("--pbkeylen must be 0, 16, 24, or 32"),
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next()
        .with_context(|| format!("{flag} requires a value"))
}

fn print_help() {
    println!(
        "\
Receive SRT input with kinugasa_gst::srt and write the raw payload to a TS file.

Required:
  --url URL          SRT receiver URL, for example srt://0.0.0.0:1234?mode=listener
  --stream-id ID     accepted SRT stream id
  --output PATH      output TS file path

Optional:
  --duration SEC     receive duration. Default: 5
  --passphrase TEXT  SRT encryption passphrase. Empty by default
  --pbkeylen BYTES   0, 16, 24, or 32. Default: 32
"
    );
}
