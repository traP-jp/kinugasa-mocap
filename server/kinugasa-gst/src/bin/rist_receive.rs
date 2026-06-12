use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use kinugasa_gst::rist::{FileReceiver, Profile};

fn main() -> Result<()> {
    let config = parse_args(std::env::args().skip(1))?;
    FileReceiver::bind_to_file(&config.url, &config.output, config.profile)
        .context("failed to configure RIST file receiver")?
        .run_for(config.duration)
        .context("failed while running RIST file receiver")?;
    Ok(())
}

struct Config {
    url: String,
    output: PathBuf,
    duration: Duration,
    profile: Profile,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Config> {
    let mut url = None;
    let mut output = None;
    let mut duration = Duration::from_secs(5);
    let mut profile = Profile::Simple;
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
                let value = next_value(&mut args, "--profile")?;
                profile = Profile::parse(&value)
                    .with_context(|| format!("unknown RIST profile: {value}"))?;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }

    Ok(Config {
        url: url.context("--url is required")?,
        output: output.context("--output is required")?,
        duration,
        profile,
    })
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next()
        .with_context(|| format!("{flag} requires a value"))
}

fn print_help() {
    println!(
        "\
Receive RIST input and write the raw payload to a TS file.

Required:
  --url URL          RIST receiver URL, for example rist://@127.0.0.1:1234
  --output PATH      output TS file path

Optional:
  --duration SEC     receive duration. Default: 5
  --profile NAME     simple, main, or advanced. Default: simple
"
    );
}
