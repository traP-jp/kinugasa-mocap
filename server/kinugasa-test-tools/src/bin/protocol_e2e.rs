use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use kinugasa_test_tools::e2e::{CompareMode, ProtocolE2eConfig, run_protocol_e2e};

fn main() -> Result<()> {
    let config = parse_args(std::env::args().skip(1))?;
    let report = run_protocol_e2e(config)?;

    println!("protocol E2E passed");
    println!("  sender url: {}", report.sender_url);
    println!("  receiver url: {}", report.receiver_url);
    println!("  reference ts: {}", report.reference_ts.display());
    println!("  output ts: {}", report.output_ts.display());
    println!("  work dir: {}", report.work_dir.display());
    if let Some(receiver_done) = report.receiver_done {
        println!("  receiver done: {}", receiver_done.display());
    }
    println!(
        "  receiver terminated by harness: {}",
        report.receiver_was_terminated
    );
    if let Some(fingerprint) = report.video_fingerprint {
        println!(
            "  video: {} bytes, hash {:016x}",
            fingerprint.bytes, fingerprint.hash
        );
    }
    if let Some(fingerprint) = report.audio_fingerprint {
        println!(
            "  audio: {} bytes, hash {:016x}",
            fingerprint.bytes, fingerprint.hash
        );
    }

    Ok(())
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<ProtocolE2eConfig> {
    let mut config = ProtocolE2eConfig::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sender-url" => config.sender_url = next_value(&mut args, "--sender-url")?,
            "--receiver" => config.receiver = next_value(&mut args, "--receiver")?,
            "--receiver-url" => {
                config.receiver_url = Some(next_value(&mut args, "--receiver-url")?)
            }
            "--receiver-done" => {
                config.receiver_done =
                    Some(PathBuf::from(next_value(&mut args, "--receiver-done")?))
            }
            "--output" => config.output = Some(PathBuf::from(next_value(&mut args, "--output")?)),
            "--work-dir" => {
                config.work_dir = Some(PathBuf::from(next_value(&mut args, "--work-dir")?))
            }
            "--host" => config.host = next_value(&mut args, "--host")?,
            "--port" => {
                config.port = Some(
                    next_value(&mut args, "--port")?
                        .parse()
                        .context("--port must be a u16")?,
                );
            }
            "--duration" => {
                config.duration = Duration::from_secs_f64(
                    next_value(&mut args, "--duration")?
                        .parse()
                        .context("--duration must be seconds")?,
                );
            }
            "--startup-delay" => {
                config.startup_delay = Duration::from_secs_f64(
                    next_value(&mut args, "--startup-delay")?
                        .parse()
                        .context("--startup-delay must be seconds")?,
                );
            }
            "--receiver-timeout" => {
                config.receiver_timeout = Duration::from_secs_f64(
                    next_value(&mut args, "--receiver-timeout")?
                        .parse()
                        .context("--receiver-timeout must be seconds")?,
                );
            }
            "--compare" => {
                config.compare = CompareMode::parse(&next_value(&mut args, "--compare")?)?;
            }
            "--skip-send" => config.skip_send = true,
            "--ignore-sender-exit-status" => config.ignore_sender_exit_status = true,
            "--keep-work-dir" => config.keep_work_dir = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }

    Ok(config)
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next()
        .with_context(|| format!("{flag} requires a value"))
}

fn print_help() {
    println!(
        "\
kinugasa-test-tools protocol E2E harness

Required:
  --sender-url URL       ffmpeg output URL. Supports {{host}} and {{port}}.
                        Not required with --skip-send.
  --receiver COMMAND     shell command that receives the stream and writes {{output}}.
                        Or use --receiver-done for an external receiver.

Optional:
  --receiver-url URL     receiver-side URL placeholder value. Defaults to --sender-url.
  --receiver-done PATH   marker file written by an external receiver container.
  --output PATH          received TS output path. Defaults inside the work dir.
  --work-dir PATH        parent directory for temporary files.
  --host HOST            host placeholder and auto-port bind host. Default: 127.0.0.1
  --port PORT            port placeholder. Default: auto-reserved UDP port.
  --duration SECONDS     generated test stream duration. Default: 5
  --startup-delay SEC    delay before ffmpeg sender starts. Default: 0.8
  --receiver-timeout SEC time to wait for receiver exit after sender ends. Default: 10
  --compare MODE         both, video, audio, or none. Default: both
  --skip-send            do not send the generated reference TS.
  --ignore-sender-exit-status
                       continue to TS comparison even if ffmpeg sender exits non-zero
  --keep-work-dir        keep temporary files after success.

Placeholders:
  {{host}} {{port}} {{duration}} {{reference}} {{output}} {{sender_url}} {{receiver_url}}

Example:
  cargo run -p kinugasa-test-tools --bin protocol_e2e -- \\
    --sender-url 'udp://{{host}}:{{port}}?pkt_size=1316' \\
    --receiver 'ffmpeg -hide_banner -loglevel error -y -i udp://{{host}}:{{port}} -t {{duration}} -c copy {{output}}'
"
    );
}
