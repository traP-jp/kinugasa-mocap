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
    output: PathBuf,
    duration: Duration,
    profile: String,
}

fn run_receiver(config: &Config) -> Result<()> {
    gst::init().context("failed to initialize GStreamer")?;
    kinugasa_gst::rist::register(None).context("failed to register ristserversrc")?;

    let output = config
        .output
        .to_str()
        .context("--output must be valid UTF-8 for GStreamer filesink")?;

    let source = gst::ElementFactory::make("ristserversrc")
        .property("url", &config.url)
        .property("profile", &config.profile)
        .build()
        .context("failed to create ristserversrc")?;
    let sink = gst::ElementFactory::make("filesink")
        .property("location", output)
        .build()
        .context("failed to create filesink")?;

    let pipeline = gst::Pipeline::new();
    pipeline
        .add_many([&source, &sink])
        .context("failed to add elements to pipeline")?;
    gst::Element::link_many([&source, &sink]).context("failed to link RIST source to filesink")?;

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
    let mut output = None;
    let mut duration = Duration::from_secs(5);
    let mut profile = "simple".to_string();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--url" => url = Some(next_value(&mut args, "--url")?),
            "--output" => output = Some(PathBuf::from(next_value(&mut args, "--output")?)),
            "--duration" => {
                duration = Duration::from_secs_f64(
                    next_value(&mut args, "--duration")?
                        .parse()
                        .context("--duration must be seconds")?,
                );
            }
            "--profile" => {
                profile = next_value(&mut args, "--profile")?;
                ensure_profile(&profile)?;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }

    ensure_profile(&profile)?;

    Ok(Config {
        url: url.context("--url is required")?,
        output: output.context("--output is required")?,
        duration,
        profile,
    })
}

fn ensure_profile(profile: &str) -> Result<()> {
    match profile {
        "simple" | "main" | "advanced" => Ok(()),
        _ => bail!("unknown RIST profile: {profile}"),
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next()
        .with_context(|| format!("{flag} requires a value"))
}

fn print_help() {
    println!(
        "\
Receive RIST input with GStreamer and write the raw payload to a TS file.

Required:
  --url URL          RIST receiver URL, for example rist://@127.0.0.1:1234
  --output PATH      output TS file path

Optional:
  --duration SEC     receive duration. Default: 5
  --profile NAME     simple, main, or advanced. Default: simple
"
    );
}
