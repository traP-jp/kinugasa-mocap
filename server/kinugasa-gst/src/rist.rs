use std::ffi::CString;
use std::ptr::{self, NonNull};
use std::slice;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_base::prelude::*;
use gst_base::subclass::base_src::CreateSuccess;
use gst_base::subclass::prelude::*;
use librist::{
    rist_ctx, rist_data_block, rist_destroy, rist_parse_address2, rist_peer, rist_peer_config,
    rist_peer_config_free2, rist_peer_create, rist_profile, rist_profile_RIST_PROFILE_ADVANCED,
    rist_profile_RIST_PROFILE_MAIN, rist_profile_RIST_PROFILE_SIMPLE, rist_receiver_create,
    rist_receiver_data_block_free2, rist_receiver_data_read2, rist_start,
};
use thiserror::Error;

#[derive(Debug, Error)]
enum Error {
    #[error("RIST URL contains an interior NUL byte")]
    UrlContainsNul,
    #[error("invalid RIST data block: {reason}")]
    InvalidDataBlock { reason: &'static str },
    #[error("librist call failed: {operation}")]
    Librist { operation: &'static str },
}

type Result<T> = std::result::Result<T, Error>;

const DEFAULT_READ_TIMEOUT: Duration = Duration::from_millis(50);
const DEFAULT_READ_TIMEOUT_MS: u32 = 50;
const RIST_SERVER_SRC_FACTORY_NAME: &str = "ristserversrc";

#[derive(Debug, Clone, Copy)]
enum Profile {
    Simple,
    Main,
    Advanced,
}

impl Profile {
    fn parse(value: &str) -> Option<Self> {
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

glib::wrapper! {
    pub struct RistServerSrc(ObjectSubclass<imp::RistServerSrc>)
        @extends gst_base::PushSrc, gst_base::BaseSrc, gst::Element, gst::Object;
}

impl RistServerSrc {
    pub fn register(plugin: Option<&gst::Plugin>) -> std::result::Result<(), glib::BoolError> {
        if gst::ElementFactory::find(RIST_SERVER_SRC_FACTORY_NAME).is_some() {
            return Ok(());
        }

        gst::Element::register(
            plugin,
            RIST_SERVER_SRC_FACTORY_NAME,
            gst::Rank::NONE,
            Self::static_type(),
        )
    }
}

pub fn register(plugin: Option<&gst::Plugin>) -> std::result::Result<(), glib::BoolError> {
    RistServerSrc::register(plugin)
}

mod imp {
    use super::*;

    #[derive(Debug, Clone)]
    struct Settings {
        url: String,
        profile: String,
        read_timeout_ms: u32,
    }

    impl Default for Settings {
        fn default() -> Self {
            Self {
                url: String::new(),
                profile: "simple".to_string(),
                read_timeout_ms: DEFAULT_READ_TIMEOUT_MS,
            }
        }
    }

    #[derive(Default)]
    pub struct RistServerSrc {
        settings: Mutex<Settings>,
        receiver: Mutex<Option<StartedReceiver>>,
        flushing: AtomicBool,
    }

    struct StartedReceiver {
        _peer: Peer,
        context: ReceiverContext,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RistServerSrc {
        const NAME: &'static str = "KinugasaRistServerSrc";
        type Type = super::RistServerSrc;
        type ParentType = gst_base::PushSrc;
    }

    impl ObjectImpl for RistServerSrc {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_live(true);
            obj.set_format(gst::Format::Time);
            obj.set_do_timestamp(true);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("url")
                        .nick("URL")
                        .blurb("RIST receiver URL, for example rist://@127.0.0.1:1234")
                        .build(),
                    glib::ParamSpecString::builder("profile")
                        .nick("Profile")
                        .blurb("RIST profile: simple, main, or advanced")
                        .default_value("simple")
                        .build(),
                    glib::ParamSpecUInt::builder("read-timeout-ms")
                        .nick("Read timeout")
                        .blurb("librist read timeout in milliseconds")
                        .minimum(1)
                        .maximum(60_000)
                        .default_value(DEFAULT_READ_TIMEOUT_MS)
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let mut settings = self.settings.lock().unwrap();
            match pspec.name() {
                "url" => {
                    settings.url = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by GObject")
                        .unwrap_or_default();
                }
                "profile" => {
                    settings.profile = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by GObject")
                        .unwrap_or_else(|| "simple".to_string());
                }
                "read-timeout-ms" => {
                    settings.read_timeout_ms =
                        value.get().expect("type conformity checked by GObject");
                }
                _ => unreachable!("unknown property {}", pspec.name()),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let settings = self.settings.lock().unwrap();
            match pspec.name() {
                "url" => settings.url.to_value(),
                "profile" => settings.profile.to_value(),
                "read-timeout-ms" => settings.read_timeout_ms.to_value(),
                _ => unreachable!("unknown property {}", pspec.name()),
            }
        }
    }

    impl GstObjectImpl for RistServerSrc {}

    impl ElementImpl for RistServerSrc {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static METADATA: OnceLock<gst::subclass::ElementMetadata> = OnceLock::new();
            Some(METADATA.get_or_init(|| {
                gst::subclass::ElementMetadata::new(
                    "RIST Server Source",
                    "Source/Network",
                    "Receives RIST payloads with librist",
                    "Kinugasa contributors",
                )
            }))
        }

        fn pad_templates() -> &'static [gst::PadTemplate] {
            static PAD_TEMPLATES: OnceLock<Vec<gst::PadTemplate>> = OnceLock::new();
            PAD_TEMPLATES.get_or_init(|| {
                let caps = gst::Caps::new_any();
                vec![
                    gst::PadTemplate::new(
                        "src",
                        gst::PadDirection::Src,
                        gst::PadPresence::Always,
                        &caps,
                    )
                    .unwrap(),
                ]
            })
        }
    }

    impl BaseSrcImpl for RistServerSrc {
        fn start(&self) -> std::result::Result<(), gst::ErrorMessage> {
            let settings = self.settings.lock().unwrap().clone();
            if settings.url.is_empty() {
                return Err(gst::error_msg!(
                    gst::ResourceError::Settings,
                    ["RIST url property is required"]
                ));
            }

            let profile = Profile::parse(&settings.profile).ok_or_else(|| {
                gst::error_msg!(
                    gst::ResourceError::Settings,
                    ["unknown RIST profile: {}", settings.profile]
                )
            })?;

            let context = ReceiverContext::new(profile).map_err(to_state_change_error)?;
            let peer_config =
                PeerConfig::parse_url(&settings.url).map_err(to_state_change_error)?;
            let peer = context
                .create_peer(&peer_config)
                .map_err(to_state_change_error)?;
            context.start().map_err(to_state_change_error)?;

            self.flushing.store(false, Ordering::Release);
            *self.receiver.lock().unwrap() = Some(StartedReceiver {
                _peer: peer,
                context,
            });

            Ok(())
        }

        fn stop(&self) -> std::result::Result<(), gst::ErrorMessage> {
            self.flushing.store(true, Ordering::Release);
            self.receiver.lock().unwrap().take();
            Ok(())
        }

        fn is_seekable(&self) -> bool {
            false
        }

        fn size(&self) -> Option<u64> {
            None
        }

        fn unlock(&self) -> std::result::Result<(), gst::ErrorMessage> {
            self.flushing.store(true, Ordering::Release);
            Ok(())
        }

        fn unlock_stop(&self) -> std::result::Result<(), gst::ErrorMessage> {
            self.flushing.store(false, Ordering::Release);
            Ok(())
        }
    }

    impl PushSrcImpl for RistServerSrc {
        fn create(
            &self,
            _buffer: Option<&mut gst::BufferRef>,
        ) -> std::result::Result<CreateSuccess, gst::FlowError> {
            loop {
                if self.flushing.load(Ordering::Acquire) {
                    return Err(gst::FlowError::Flushing);
                }

                let timeout = self.read_timeout();
                let block = {
                    let receiver = self.receiver.lock().unwrap();
                    let Some(receiver) = receiver.as_ref() else {
                        return Err(gst::FlowError::Flushing);
                    };
                    receiver.context.read(timeout)
                };

                match block {
                    Ok(Some(block)) => {
                        let payload = block.payload().map_err(|err| self.post_read_error(err))?;
                        return Ok(CreateSuccess::NewBuffer(gst::Buffer::from_slice(
                            payload.to_vec(),
                        )));
                    }
                    Ok(None) => continue,
                    Err(err) => return Err(self.post_read_error(err)),
                }
            }
        }
    }

    impl RistServerSrc {
        fn read_timeout(&self) -> Duration {
            let settings = self.settings.lock().unwrap();
            if settings.read_timeout_ms == DEFAULT_READ_TIMEOUT_MS {
                DEFAULT_READ_TIMEOUT
            } else {
                Duration::from_millis(settings.read_timeout_ms.into())
            }
        }

        fn post_read_error(&self, err: Error) -> gst::FlowError {
            gst::element_error!(
                self.obj(),
                gst::ResourceError::Read,
                ["failed to read RIST payload: {}", err]
            );
            gst::FlowError::Error
        }
    }

    fn to_state_change_error(err: Error) -> gst::ErrorMessage {
        gst::error_msg!(
            gst::ResourceError::OpenRead,
            ["failed to start RIST receiver: {}", err]
        )
    }
}

struct ReceiverContext {
    raw: *mut rist_ctx,
}

// librist contexts are owned by this wrapper and all access from the GStreamer
// source is serialized through the element's mutex.
unsafe impl Send for ReceiverContext {}

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
        Ok(Some(DataBlock::new(raw)?))
    }
}

struct DataBlock {
    raw: NonNull<rist_data_block>,
}

impl DataBlock {
    fn new(raw: *mut rist_data_block) -> Result<Self> {
        let raw = NonNull::new(raw).ok_or(Error::InvalidDataBlock {
            reason: "librist returned a null data block",
        })?;
        Ok(Self { raw })
    }

    fn payload(&self) -> Result<&[u8]> {
        // SAFETY: `raw` is a non-null block returned by librist and owned by
        // this wrapper until Drop calls `rist_receiver_data_block_free2`.
        let block = unsafe { self.raw.as_ref() };
        if block.payload_len == 0 {
            return Ok(&[]);
        }

        let payload =
            NonNull::new(block.payload.cast_mut().cast::<u8>()).ok_or(Error::InvalidDataBlock {
                reason: "librist returned a null payload for a non-empty data block",
            })?;

        // SAFETY: librist guarantees the payload pointer and length are valid
        // for the lifetime of this data block.
        Ok(unsafe { slice::from_raw_parts(payload.as_ptr(), block.payload_len) })
    }
}

impl Drop for DataBlock {
    fn drop(&mut self) {
        let mut raw = self.raw.as_ptr();
        // SAFETY: `raw` is an owned librist data block returned by read2 and is
        // freed exactly once by this RAII wrapper.
        unsafe {
            rist_receiver_data_block_free2(&mut raw);
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

unsafe impl Send for Peer {}

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

fn ensure_success(status: i32, operation: &'static str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(Error::Librist { operation })
    }
}
