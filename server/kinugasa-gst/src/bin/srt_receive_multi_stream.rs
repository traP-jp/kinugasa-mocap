use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;
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
    stream_ids: Vec<String>,
    output: Option<PathBuf>,
    stream_outputs: Vec<StreamOutput>,
    duration: Duration,
    passphrase: String,
    pbkeylen: u32,
    actions: Vec<TimedAction>,
}

#[derive(Debug, Clone)]
struct StreamOutput {
    stream_id: String,
    output: PathBuf,
}

#[derive(Debug, Clone)]
struct TimedAction {
    after: Duration,
    kind: ActionKind,
    stream_id: String,
}

#[derive(Debug, Clone, Copy)]
enum ActionKind {
    Add,
    Remove,
}

fn run_receiver(config: &Config) -> Result<()> {
    gst::init().context("failed to initialize GStreamer")?;
    kinugasa_gst::srt::register(None).context("failed to register kinugasasrtserversrc")?;

    let output_routes = output_routes(config)?;

    let source = gst::ElementFactory::make("kinugasasrtserversrc")
        .property("uri", &config.url)
        .property("stream-ids", config.stream_ids.join(","))
        .property("passphrase", &config.passphrase)
        .property("pbkeylen", config.pbkeylen)
        .build()
        .context("failed to create kinugasasrtserversrc")?;
    let srt_source = source
        .clone()
        .downcast::<kinugasa_gst::srt::SrtServerSrc>()
        .expect("kinugasasrtserversrc factory must create SrtServerSrc");

    let pipeline = gst::Pipeline::new();
    pipeline
        .add(&source)
        .context("failed to add source to pipeline")?;

    let mut output_sinks = HashMap::new();
    for route in &output_routes {
        let output = route.output.to_str().with_context(|| {
            format!(
                "output for stream ID {} must be valid UTF-8",
                route.stream_id
            )
        })?;
        let sink = gst::ElementFactory::make("filesink")
            .property("location", output)
            .build()
            .with_context(|| {
                format!(
                    "failed to create filesink for stream ID {}",
                    route.stream_id
                )
            })?;
        pipeline
            .add(&sink)
            .with_context(|| format!("failed to add filesink for stream ID {}", route.stream_id))?;
        if output_sinks.insert(route.stream_id.clone(), sink).is_some() {
            bail!("duplicate output route for stream ID {}", route.stream_id);
        }
    }

    for stream_id in &config.stream_ids {
        let sink = output_sinks.get(stream_id).with_context(|| {
            format!("missing --stream-output for initial stream ID {stream_id}")
        })?;
        link_stream_output(&srt_source, sink, stream_id)?;
    }

    pipeline
        .set_state(gst::State::Playing)
        .map_err(|_| anyhow::anyhow!("failed to start GStreamer pipeline"))?;

    let action_handles = config
        .actions
        .iter()
        .cloned()
        .map(|action| {
            let srt_source = srt_source.clone();
            let output_sinks = output_sinks.clone();
            thread::spawn(move || {
                thread::sleep(action.after);
                match action.kind {
                    ActionKind::Add => {
                        srt_source.add_stream_id(&action.stream_id);
                        if let Some(sink) = output_sinks.get(&action.stream_id) {
                            link_stream_output(&srt_source, sink, &action.stream_id)?;
                        }
                        Ok::<(), anyhow::Error>(())
                    }
                    ActionKind::Remove => {
                        let _ = srt_source.remove_stream_id(&action.stream_id);
                        Ok::<(), anyhow::Error>(())
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    let result = wait_for_completion(&pipeline, config.duration);

    for handle in action_handles {
        handle
            .join()
            .map_err(|_| anyhow::anyhow!("dynamic stream-id action thread panicked"))??;
    }

    let stop_result = pipeline
        .set_state(gst::State::Null)
        .map(|_| ())
        .map_err(|_| anyhow::anyhow!("failed to stop GStreamer pipeline"));

    result.and(stop_result)
}

fn output_routes(config: &Config) -> Result<Vec<StreamOutput>> {
    if !config.stream_outputs.is_empty() {
        if config.output.is_some() {
            bail!("use either --output or --stream-output, not both");
        }
        return Ok(config.stream_outputs.clone());
    }

    let output = config.output.clone().context(
        "--output is required for one stream, or use repeatable --stream-output STREAM_ID=PATH",
    )?;
    let stream_id = match config.stream_ids.as_slice() {
        [stream_id] => stream_id.clone(),
        [] => bail!("--output requires exactly one initial --stream-id"),
        _ => bail!("--output can only be used with one stream ID; use --stream-output instead"),
    };
    Ok(vec![StreamOutput { stream_id, output }])
}

fn link_stream_output(
    source: &kinugasa_gst::srt::SrtServerSrc,
    sink: &gst::Element,
    stream_id: &str,
) -> Result<()> {
    let source_pad = source
        .pad_for_stream_id(stream_id)
        .with_context(|| format!("source pad for stream ID {stream_id} does not exist"))?;
    if source_pad.is_linked() {
        return Ok(());
    }
    let sink_pad = sink
        .static_pad("sink")
        .with_context(|| format!("filesink for stream ID {stream_id} has no sink pad"))?;
    source_pad
        .link(&sink_pad)
        .with_context(|| format!("failed to link stream ID {stream_id} to filesink"))?;
    Ok(())
}

fn wait_for_completion(pipeline: &gst::Pipeline, duration: Duration) -> Result<()> {
    let bus = pipeline.bus().context("pipeline has no bus")?;
    let timeout = gst::ClockTime::try_from(duration).context("--duration is too large")?;
    let message = bus.timed_pop_filtered(Some(timeout), &[gst::MessageType::Error]);

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
        _ => Ok(()),
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Config> {
    let mut url = None;
    let mut stream_ids = Vec::new();
    let mut output = None;
    let mut stream_outputs = Vec::new();
    let mut duration = Duration::from_secs(5);
    let mut passphrase = String::new();
    let mut pbkeylen = 32;
    let mut actions = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--url" => url = Some(next_value(&mut args, "--url")?),
            "--stream-id" => stream_ids.push(next_value(&mut args, "--stream-id")?),
            "--stream-ids" => {
                stream_ids.extend(parse_stream_ids(&next_value(&mut args, "--stream-ids")?));
            }
            "--output" => output = Some(PathBuf::from(next_value(&mut args, "--output")?)),
            "--stream-output" => {
                stream_outputs.push(parse_stream_output(&next_value(
                    &mut args,
                    "--stream-output",
                )?)?);
            }
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
            "--add-stream-id-after" => {
                actions.push(parse_timed_action(
                    ActionKind::Add,
                    &next_value(&mut args, "--add-stream-id-after")?,
                )?);
            }
            "--remove-stream-id-after" => {
                actions.push(parse_timed_action(
                    ActionKind::Remove,
                    &next_value(&mut args, "--remove-stream-id-after")?,
                )?);
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
        stream_ids: dedupe_stream_ids(stream_ids),
        output,
        stream_outputs,
        duration,
        passphrase,
        pbkeylen,
        actions,
    })
}

fn parse_timed_action(kind: ActionKind, value: &str) -> Result<TimedAction> {
    let (stream_id, seconds) = value
        .split_once(':')
        .context("timed action must be STREAM_ID:SECONDS")?;
    let stream_id = stream_id.trim();
    if stream_id.is_empty() {
        bail!("timed action stream ID must not be empty");
    }
    let after = Duration::from_secs_f64(seconds.parse().context("SECONDS must be a number")?);
    Ok(TimedAction {
        after,
        kind,
        stream_id: stream_id.to_string(),
    })
}

fn parse_stream_output(value: &str) -> Result<StreamOutput> {
    let (stream_id, path) = value
        .split_once('=')
        .context("--stream-output must be STREAM_ID=PATH")?;
    let stream_id = stream_id.trim();
    if stream_id.is_empty() {
        bail!("--stream-output stream ID must not be empty");
    }
    if path.is_empty() {
        bail!("--stream-output path must not be empty");
    }
    Ok(StreamOutput {
        stream_id: stream_id.to_string(),
        output: PathBuf::from(path),
    })
}

fn parse_stream_ids(value: &str) -> impl Iterator<Item = String> + '_ {
    value
        .split(',')
        .map(str::trim)
        .filter(|stream_id| !stream_id.is_empty())
        .map(ToOwned::to_owned)
}

fn dedupe_stream_ids(stream_ids: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for stream_id in stream_ids {
        if !stream_id.is_empty() && !deduped.contains(&stream_id) {
            deduped.push(stream_id);
        }
    }
    deduped
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
Receive multiple SRT stream IDs through kinugasa_gst::srt and write raw payloads to TS files.

Required:
  --url URL             SRT listener URL, for example srt://0.0.0.0:1234?mode=listener
  --stream-id ID        accepted SRT stream ID. Repeatable
  --stream-ids IDS      comma-separated accepted SRT stream IDs
  --output PATH         output TS file path for a single initial stream
  --stream-output ID=PATH
                        output TS file path for a stream ID. Repeatable

Optional:
  --duration SEC        receive duration. Default: 5
  --passphrase TEXT     SRT encryption passphrase. Empty by default
  --pbkeylen BYTES      0, 16, 24, or 32. Default: 32
  --add-stream-id-after STREAM_ID:SEC
  --remove-stream-id-after STREAM_ID:SEC
"
    );
}
