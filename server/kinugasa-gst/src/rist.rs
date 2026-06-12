use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::ptr;
use std::time::{Duration, Instant};
use std::{io::Write, slice};

use librist::{
    rist_ctx, rist_data_block, rist_data_fd_stats, rist_data_fd_stats_get, rist_destroy,
    rist_parse_address2, rist_peer, rist_peer_config, rist_peer_config_free2, rist_peer_create,
    rist_profile, rist_profile_RIST_PROFILE_ADVANCED, rist_profile_RIST_PROFILE_MAIN,
    rist_profile_RIST_PROFILE_SIMPLE, rist_receiver_create, rist_receiver_data_block_free2,
    rist_receiver_data_read2, rist_start,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("RIST URL contains an interior NUL byte")]
    UrlContainsNul,
    #[error("failed to open output file {path}")]
    OpenOutput {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write RIST payload")]
    WriteOutput(#[source] std::io::Error),
    #[error("librist call failed: {operation}")]
    Librist { operation: &'static str },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub enum Profile {
    Simple,
    Main,
    Advanced,
}

impl Profile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "simple" => Some(Self::Simple),
            "main" => Some(Self::Main),
            "advanced" => Some(Self::Advanced),
            _ => None,
        }
    }

    fn as_raw(self) -> rist_profile {
        match self {
            Self::Simple => rist_profile_RIST_PROFILE_SIMPLE,
            Self::Main => rist_profile_RIST_PROFILE_MAIN,
            Self::Advanced => rist_profile_RIST_PROFILE_ADVANCED,
        }
    }
}

pub struct FileReceiver {
    _peer: Peer,
    context: ReceiverContext,
    output: File,
}

impl FileReceiver {
    pub fn bind_to_file(url: &str, output: impl AsRef<Path>, profile: Profile) -> Result<Self> {
        let output = open_output(output.as_ref())?;
        let context = ReceiverContext::new(profile)?;
        let peer_config = PeerConfig::parse_url(url)?;
        let peer = context.create_peer(&peer_config)?;

        Ok(Self {
            _peer: peer,
            context,
            output,
        })
    }

    pub fn start(&self) -> Result<()> {
        self.context.start()
    }

    pub fn run_for(self, duration: Duration) -> Result<()> {
        self.start()?;
        let deadline = Instant::now() + duration;
        let mut output = self.output;
        while Instant::now() < deadline {
            if let Some(block) = self.context.read(Duration::from_millis(50))? {
                output
                    .write_all(block.payload())
                    .map_err(Error::WriteOutput)?;
            }
        }
        output.sync_all().ok();
        Ok(())
    }

    pub fn stats(&self) -> Result<DataFdStats> {
        self.context.stats()
    }
}

fn open_output(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|source| Error::OpenOutput {
            path: path.display().to_string(),
            source,
        })
}

struct ReceiverContext {
    raw: *mut rist_ctx,
}

impl ReceiverContext {
    fn new(profile: Profile) -> Result<Self> {
        let mut raw = ptr::null_mut();
        // SAFETY: librist initializes `raw` when the call succeeds. Logging settings
        // are optional, and a null pointer requests librist defaults.
        let status = unsafe { rist_receiver_create(&mut raw, profile.as_raw(), ptr::null_mut()) };
        ensure_success(status, "rist_receiver_create")?;

        if raw.is_null() {
            return Err(Error::Librist {
                operation: "rist_receiver_create returned null context",
            });
        }

        Ok(Self { raw })
    }

    fn create_peer(&self, config: &PeerConfig) -> Result<Peer> {
        let mut raw = ptr::null_mut();
        // SAFETY: `self.raw` is a live receiver context and `config.raw` is a valid
        // peer config allocated by librist.
        let status = unsafe { rist_peer_create(self.raw, &mut raw, config.raw) };
        ensure_success(status, "rist_peer_create")?;

        if raw.is_null() {
            return Err(Error::Librist {
                operation: "rist_peer_create returned null peer",
            });
        }

        Ok(Peer { raw })
    }

    fn start(&self) -> Result<()> {
        // SAFETY: `self.raw` is a live receiver context with at least one peer.
        let status = unsafe { rist_start(self.raw) };
        ensure_success(status, "rist_start")
    }

    fn read(&self, timeout: Duration) -> Result<Option<DataBlock>> {
        let mut raw = ptr::null_mut();
        // SAFETY: `self.raw` is live and `raw` is a valid out pointer. librist
        // returns an owned reference-counted block that `DataBlock` frees.
        let status =
            unsafe { rist_receiver_data_read2(self.raw, &mut raw, timeout.as_millis() as i32) };
        if status < 0 {
            return Err(Error::Librist {
                operation: "rist_receiver_data_read2",
            });
        }
        if raw.is_null() {
            return Ok(None);
        }
        Ok(Some(DataBlock { raw }))
    }

    fn stats(&self) -> Result<DataFdStats> {
        let mut stats = rist_data_fd_stats {
            tx_packets: 0,
            tx_bytes: 0,
            rx_packets: 0,
            rx_bytes: 0,
        };
        // SAFETY: `stats` is a valid output pointer and `self.raw` is live.
        let status = unsafe { rist_data_fd_stats_get(self.raw, &mut stats) };
        ensure_success(status, "rist_data_fd_stats_get")?;
        Ok(DataFdStats {
            tx_packets: stats.tx_packets,
            tx_bytes: stats.tx_bytes,
            rx_packets: stats.rx_packets,
            rx_bytes: stats.rx_bytes,
        })
    }
}

struct DataBlock {
    raw: *mut rist_data_block,
}

impl DataBlock {
    fn payload(&self) -> &[u8] {
        // SAFETY: librist gives a valid payload pointer and length for the
        // lifetime of the data block. The block is freed in Drop after use.
        unsafe {
            let block = &*self.raw;
            slice::from_raw_parts(block.payload.cast::<u8>(), block.payload_len)
        }
    }
}

impl Drop for DataBlock {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: `raw` is an owned librist data block returned by read2.
            unsafe {
                rist_receiver_data_block_free2(&mut self.raw);
            }
        }
    }
}

impl Drop for ReceiverContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: `raw` is owned by this RAII wrapper and is destroyed once.
            unsafe {
                let _ = rist_destroy(self.raw);
            }
            self.raw = ptr::null_mut();
        }
    }
}

struct Peer {
    raw: *mut rist_peer,
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.raw = ptr::null_mut();
    }
}

struct PeerConfig {
    raw: *mut rist_peer_config,
}

impl PeerConfig {
    fn parse_url(url: &str) -> Result<Self> {
        let url = CString::new(url).map_err(|_| Error::UrlContainsNul)?;
        let mut raw = ptr::null_mut();
        // SAFETY: `url` is a valid NUL-terminated string. Passing a null config
        // pointer lets librist allocate and populate one for this peer.
        let status = unsafe { rist_parse_address2(url.as_ptr(), &mut raw) };
        if status < 0 {
            return Err(Error::Librist {
                operation: "rist_parse_address2",
            });
        }

        if raw.is_null() {
            return Err(Error::Librist {
                operation: "rist_parse_address2 returned null config",
            });
        }

        Ok(Self { raw })
    }
}

impl Drop for PeerConfig {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: `raw` was allocated by librist and is owned by this wrapper.
            unsafe {
                let _ = rist_peer_config_free2(&mut self.raw);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataFdStats {
    pub tx_packets: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub rx_bytes: u64,
}

fn ensure_success(status: i32, operation: &'static str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(Error::Librist { operation })
    }
}
