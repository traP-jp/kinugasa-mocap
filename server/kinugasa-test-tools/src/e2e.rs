use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareMode {
    Both,
    Video,
    Audio,
    None,
}

impl CompareMode {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "both" => Ok(Self::Both),
            "video" => Ok(Self::Video),
            "audio" => Ok(Self::Audio),
            "none" => Ok(Self::None),
            _ => bail!("unknown compare mode: {value}"),
        }
    }

    fn includes_video(&self) -> bool {
        matches!(self, Self::Both | Self::Video)
    }

    fn includes_audio(&self) -> bool {
        matches!(self, Self::Both | Self::Audio)
    }
}

#[derive(Debug, Clone)]
pub struct ProtocolE2eConfig {
    pub sender_url: String,
    pub receiver: String,
    pub receiver_url: Option<String>,
    pub receiver_done: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub work_dir: Option<PathBuf>,
    pub host: String,
    pub port: Option<u16>,
    pub duration: Duration,
    pub startup_delay: Duration,
    pub receiver_timeout: Duration,
    pub compare: CompareMode,
    pub skip_send: bool,
    pub ignore_sender_exit_status: bool,
    pub keep_work_dir: bool,
}

impl Default for ProtocolE2eConfig {
    fn default() -> Self {
        Self {
            sender_url: String::new(),
            receiver: String::new(),
            receiver_url: None,
            receiver_done: None,
            output: None,
            work_dir: None,
            host: "127.0.0.1".to_owned(),
            port: None,
            duration: Duration::from_secs(5),
            startup_delay: Duration::from_millis(800),
            receiver_timeout: Duration::from_secs(10),
            compare: CompareMode::Both,
            skip_send: false,
            ignore_sender_exit_status: false,
            keep_work_dir: false,
        }
    }
}

#[derive(Debug)]
pub struct ProtocolE2eReport {
    pub work_dir: PathBuf,
    pub reference_ts: PathBuf,
    pub output_ts: PathBuf,
    pub sender_url: String,
    pub receiver_url: String,
    pub receiver_done: Option<PathBuf>,
    pub receiver_was_terminated: bool,
    pub video_fingerprint: Option<StreamFingerprint>,
    pub audio_fingerprint: Option<StreamFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamFingerprint {
    pub bytes: usize,
    pub hash: u64,
}

pub fn run_protocol_e2e(mut config: ProtocolE2eConfig) -> Result<ProtocolE2eReport> {
    validate_config(&config)?;

    let port = match config.port {
        Some(port) => port,
        None => reserve_udp_port(&config.host)?,
    };
    config.port = Some(port);

    let work_dir = create_work_dir(config.work_dir.as_deref())?;
    let reference_ts = work_dir.path().join("reference.ts");
    let output_ts = config
        .output
        .clone()
        .unwrap_or_else(|| work_dir.path().join("received.ts"));

    let mut placeholders = base_placeholders(&config, &reference_ts, &output_ts);
    let sender_url = expand_template(&config.sender_url, &placeholders);
    let receiver_url = expand_template(
        config
            .receiver_url
            .as_deref()
            .unwrap_or(config.sender_url.as_str()),
        &placeholders,
    );
    placeholders.insert("sender_url", shell_quote(&sender_url));
    placeholders.insert("receiver_url", shell_quote(&receiver_url));
    let receiver_command = expand_template(&config.receiver, &placeholders);

    generate_reference_ts(&reference_ts, config.duration)?;

    if let Some(parent) = output_ts.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }

    let mut receiver = if config.receiver.is_empty() {
        None
    } else {
        Some(spawn_shell(&receiver_command, "receiver")?)
    };
    thread::sleep(config.startup_delay);

    let sender_result = if config.skip_send {
        Ok(())
    } else {
        send_reference_ts(&reference_ts, &sender_url)
    };
    let receiver_result = match &mut receiver {
        Some(receiver) => finish_receiver(receiver, config.receiver_timeout),
        None => wait_for_receiver_done(
            config
                .receiver_done
                .as_deref()
                .expect("external receiver requires receiver_done"),
            config.receiver_timeout,
        ),
    };

    if sender_result.is_err() && !config.ignore_sender_exit_status {
        if let Some(receiver) = &mut receiver {
            kill_child(receiver);
        }
    }

    if !config.ignore_sender_exit_status {
        sender_result?;
    }
    let receiver_was_terminated = receiver_result?;

    if !output_ts.exists() {
        bail!(
            "receiver completed but output file was not written: {}",
            output_ts.display()
        );
    }

    let video_fingerprint = if config.compare.includes_video() {
        let expected = fingerprint_stream(&reference_ts, StreamKind::Video)?;
        let actual = fingerprint_stream(&output_ts, StreamKind::Video)?;
        ensure_same_fingerprint("video", &expected, &actual)?;
        Some(actual)
    } else {
        None
    };

    let audio_fingerprint = if config.compare.includes_audio() {
        let expected = fingerprint_stream(&reference_ts, StreamKind::Audio)?;
        let actual = fingerprint_stream(&output_ts, StreamKind::Audio)?;
        ensure_same_fingerprint("audio", &expected, &actual)?;
        Some(actual)
    } else {
        None
    };

    let report = ProtocolE2eReport {
        work_dir: work_dir.path().to_owned(),
        reference_ts,
        output_ts,
        sender_url,
        receiver_url,
        receiver_done: config.receiver_done.clone(),
        receiver_was_terminated,
        video_fingerprint,
        audio_fingerprint,
    };

    if config.keep_work_dir {
        let _ = work_dir.keep();
    }

    Ok(report)
}

pub fn expand_template(template: &str, placeholders: &BTreeMap<&'static str, String>) -> String {
    placeholders
        .iter()
        .fold(template.to_owned(), |expanded, (key, value)| {
            expanded.replace(&format!("{{{key}}}"), value)
        })
}

fn validate_config(config: &ProtocolE2eConfig) -> Result<()> {
    if config.sender_url.is_empty() && !config.skip_send {
        bail!("--sender-url is required");
    }
    if config.receiver.is_empty() && config.receiver_done.is_none() {
        bail!("--receiver or --receiver-done is required");
    }
    if config.receiver_done.is_none() && config.skip_send {
        bail!("--skip-send requires --receiver-done");
    }
    if config.duration.is_zero() {
        bail!("--duration must be greater than zero");
    }
    if config.receiver_timeout.is_zero() {
        bail!("--receiver-timeout must be greater than zero");
    }
    Ok(())
}

fn base_placeholders(
    config: &ProtocolE2eConfig,
    reference_ts: &Path,
    output_ts: &Path,
) -> BTreeMap<&'static str, String> {
    let mut values = BTreeMap::new();
    values.insert("duration", duration_secs(config.duration).to_string());
    values.insert("host", config.host.clone());
    values.insert("output", shell_quote_path(output_ts));
    values.insert("port", config.port.expect("port assigned").to_string());
    values.insert("reference", shell_quote_path(reference_ts));
    values
}

fn reserve_udp_port(host: &str) -> Result<u16> {
    let socket = UdpSocket::bind((host, 0))
        .with_context(|| format!("failed to bind UDP socket on {host}"))?;
    Ok(socket.local_addr()?.port())
}

fn create_work_dir(parent: Option<&Path>) -> Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("kinugasa-test-tools-e2e-");
    match parent {
        Some(parent) => builder
            .tempdir_in(parent)
            .with_context(|| format!("failed to create work directory in {}", parent.display())),
        None => builder
            .tempdir()
            .context("failed to create temporary work directory"),
    }
}

fn generate_reference_ts(path: &Path, duration: Duration) -> Result<()> {
    let duration = duration_secs(duration).to_string();
    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000",
            "-t",
            &duration,
            "-c:v",
            "mpeg2video",
            "-g",
            "15",
            "-bf",
            "0",
            "-c:a",
            "mp2",
            "-b:a",
            "128k",
            "-f",
            "mpegts",
        ])
        .arg(path)
        .output()
        .context("failed to execute ffmpeg while generating reference TS")?;
    ensure_success("ffmpeg reference generator", output).map(|_| ())
}

fn send_reference_ts(reference_ts: &Path, sender_url: &str) -> Result<()> {
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-re", "-i"])
        .arg(reference_ts)
        .args(["-c", "copy", "-f", "mpegts", sender_url])
        .output()
        .context("failed to execute ffmpeg sender")?;
    ensure_success("ffmpeg sender", output).map(|_| ())
}

fn spawn_shell(command: &str, label: &str) -> Result<Child> {
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {label}: {command}"))
}

fn finish_receiver(child: &mut Child, timeout: Duration) -> Result<bool> {
    let started_at = Instant::now();
    loop {
        if let Some(status) = child.try_wait().context("failed to poll receiver")? {
            let mut stdout = String::new();
            let mut stderr = String::new();
            read_child_output(child, &mut stdout, &mut stderr);
            if status.success() {
                return Ok(false);
            }
            bail!(
                "receiver exited with status {status}\nstdout:\n{}\nstderr:\n{}",
                stdout,
                stderr
            );
        }

        if started_at.elapsed() >= timeout {
            terminate_child(child);
            return Ok(true);
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn wait_for_receiver_done(path: &Path, timeout: Duration) -> Result<bool> {
    let started_at = Instant::now();
    loop {
        if path.exists() {
            return Ok(false);
        }

        if started_at.elapsed() >= timeout {
            bail!(
                "external receiver did not write done marker before timeout: {}",
                path.display()
            );
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn read_child_output(child: &mut Child, stdout: &mut String, stderr: &mut String) {
    if let Some(pipe) = &mut child.stdout {
        let _ = pipe.read_to_string(stdout);
    }
    if let Some(pipe) = &mut child.stderr {
        let _ = pipe.read_to_string(stderr);
    }
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn terminate_child(child: &mut Child) {
    let pid = child.id().to_string();
    let _ = Command::new("kill").arg("-TERM").arg(pid).status();

    let started_at = Instant::now();
    while started_at.elapsed() < Duration::from_secs(2) {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(_) => return,
        }
    }

    kill_child(child);
}

#[derive(Debug, Clone, Copy)]
enum StreamKind {
    Video,
    Audio,
}

fn fingerprint_stream(path: &Path, stream: StreamKind) -> Result<StreamFingerprint> {
    let mut command = Command::new("ffmpeg");
    command.args(["-hide_banner", "-loglevel", "error", "-i"]);
    command.arg(path);

    match stream {
        StreamKind::Video => {
            command.args([
                "-map", "0:v:0", "-f", "rawvideo", "-pix_fmt", "yuv420p", "-",
            ]);
        }
        StreamKind::Audio => {
            command.args([
                "-map",
                "0:a:0",
                "-f",
                "s16le",
                "-acodec",
                "pcm_s16le",
                "-ac",
                "2",
                "-ar",
                "48000",
                "-",
            ]);
        }
    }

    let output = command
        .output()
        .with_context(|| format!("failed to execute ffmpeg while decoding {}", path.display()))?;
    ensure_success("ffmpeg stream decoder", output).map(|stdout| {
        let hash = fnv1a64(&stdout);
        StreamFingerprint {
            bytes: stdout.len(),
            hash,
        }
    })
}

fn ensure_same_fingerprint(
    label: &str,
    expected: &StreamFingerprint,
    actual: &StreamFingerprint,
) -> Result<()> {
    if expected != actual {
        bail!(
            "{label} fingerprint mismatch: expected {} bytes hash {:016x}, got {} bytes hash {:016x}",
            expected.bytes,
            expected.hash,
            actual.bytes,
            actual.hash
        );
    }
    Ok(())
}

fn ensure_success(label: &str, output: Output) -> Result<Vec<u8>> {
    if output.status.success() {
        return Ok(output.stdout);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(anyhow!(
        "{label} exited with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout,
        stderr
    ))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

fn duration_secs(duration: Duration) -> f64 {
    duration.as_secs_f64()
}

fn shell_quote_path(path: &Path) -> String {
    shell_quote(path.as_os_str().to_string_lossy().as_ref())
}

fn shell_quote(value: &str) -> String {
    let mut quoted = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

pub fn unique_path(prefix: &str, extension: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut filename = OsString::from(prefix);
    filename.push("-");
    filename.push(std::process::id().to_string());
    filename.push("-");
    filename.push(millis.to_string());
    filename.push(".");
    filename.push(extension);
    std::env::temp_dir().join(filename)
}
