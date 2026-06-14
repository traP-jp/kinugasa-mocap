use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::net::{Ipv4Addr, SocketAddr};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;

type SrtSocket = c_int;

const DEFAULT_URI: &str = "srt://0.0.0.0:7001?mode=listener";
const DEFAULT_BLOCKSIZE: u32 = 1316 * 8;
const DEFAULT_LATENCY: i32 = 125;
const DEFAULT_POLL_TIMEOUT: i32 = 1000;
const DEFAULT_PBKEYLEN: u32 = 32;
const SRT_INVALID_SOCK: SrtSocket = -1;
const SRT_ERROR: c_int = -1;
const SRT_EPOLL_IN: c_int = 0x1;
const SRT_EPOLL_ERR: c_int = 0x8;
const SRTO_RCVTIMEO: c_int = 14;
const SRTO_REUSEADDR: c_int = 15;
const SRTO_PASSPHRASE: c_int = 26;
const SRTO_PBKEYLEN: c_int = 27;
const SRTO_LATENCY: c_int = 23;
const SRTO_TRANSTYPE: c_int = 50;
const SRTO_STREAMID: c_int = 46;
const SRTT_LIVE: c_int = 0;
const SRT_SERVER_SRC_FACTORY_NAME: &str = "kinugasasrtserversrc";

unsafe extern "C" {
    fn srt_startup() -> c_int;
    fn srt_create_socket() -> SrtSocket;
    fn srt_bind(sock: SrtSocket, name: *const libc::sockaddr, namelen: c_int) -> c_int;
    fn srt_listen(sock: SrtSocket, backlog: c_int) -> c_int;
    fn srt_accept(sock: SrtSocket, addr: *mut libc::sockaddr, addrlen: *mut c_int) -> SrtSocket;
    fn srt_listen_callback(
        sock: SrtSocket,
        hook_fn: extern "C" fn(
            *mut c_void,
            SrtSocket,
            c_int,
            *const libc::sockaddr,
            *const c_char,
        ) -> c_int,
        hook_opaque: *mut c_void,
    ) -> c_int;
    fn srt_close(sock: SrtSocket) -> c_int;
    fn srt_setsockflag(sock: SrtSocket, opt: c_int, optval: *const c_void, optlen: c_int) -> c_int;
    fn srt_recvmsg(sock: SrtSocket, buf: *mut c_char, len: c_int) -> c_int;
    fn srt_getlasterror_str() -> *const c_char;
    fn srt_getsockflag(
        sock: SrtSocket,
        opt: c_int,
        optval: *mut c_void,
        optlen: *mut c_int,
    ) -> c_int;
    fn srt_epoll_create() -> c_int;
    fn srt_epoll_add_usock(eid: c_int, sock: SrtSocket, events: *const c_int) -> c_int;
    fn srt_epoll_wait(
        eid: c_int,
        readfds: *mut SrtSocket,
        rnum: *mut c_int,
        writefds: *mut SrtSocket,
        wnum: *mut c_int,
        ms_timeout: i64,
        lrfds: *mut libc::c_int,
        lrnum: *mut c_int,
        lwfds: *mut libc::c_int,
        lwnum: *mut c_int,
    ) -> c_int;
    fn srt_epoll_release(eid: c_int) -> c_int;
}

glib::wrapper! {
    pub struct SrtServerSrc(ObjectSubclass<imp::SrtServerSrc>)
        @extends gst::Element, gst::Object;
}

impl SrtServerSrc {
    pub fn register(plugin: Option<&gst::Plugin>) -> std::result::Result<(), glib::BoolError> {
        if gst::ElementFactory::find(SRT_SERVER_SRC_FACTORY_NAME).is_some() {
            return Ok(());
        }

        gst::Element::register(
            plugin,
            SRT_SERVER_SRC_FACTORY_NAME,
            gst::Rank::NONE,
            Self::static_type(),
        )
    }

    pub fn add_stream_id(&self, stream_id: &str) {
        self.emit_by_name::<()>("add-stream-id", &[&stream_id]);
    }

    pub fn remove_stream_id(&self, stream_id: &str) -> bool {
        self.emit_by_name::<bool>("remove-stream-id", &[&stream_id])
    }

    pub fn clear_stream_ids(&self) {
        self.emit_by_name::<()>("clear-stream-ids", &[]);
    }

    pub fn stream_ids(&self) -> Vec<String> {
        self.imp().stream_ids()
    }

    pub fn pad_for_stream_id(&self, stream_id: &str) -> Option<gst::Pad> {
        self.imp().pad_for_stream_id(stream_id)
    }
}

pub fn register(plugin: Option<&gst::Plugin>) -> std::result::Result<(), glib::BoolError> {
    SrtServerSrc::register(plugin)
}

mod imp {
    use super::*;

    #[derive(Debug, Clone)]
    struct Settings {
        uri: String,
        passphrase: String,
        pbkeylen: u32,
        latency: i32,
        poll_timeout: i32,
        blocksize: u32,
        do_timestamp: bool,
    }

    impl Default for Settings {
        fn default() -> Self {
            Self {
                uri: DEFAULT_URI.to_string(),
                passphrase: String::new(),
                pbkeylen: DEFAULT_PBKEYLEN,
                latency: DEFAULT_LATENCY,
                poll_timeout: DEFAULT_POLL_TIMEOUT,
                blocksize: DEFAULT_BLOCKSIZE,
                do_timestamp: true,
            }
        }
    }

    #[derive(Debug, Clone)]
    struct StreamRoute {
        stream_id: String,
        pad_name: String,
    }

    #[derive(Default)]
    pub struct SrtServerSrc {
        settings: Mutex<Settings>,
        allowed_stream_ids: Arc<Mutex<HashSet<String>>>,
        pads: Arc<Mutex<HashMap<String, gst::Pad>>>,
        pad_names: Mutex<HashMap<String, String>>,
        runtime: Mutex<Option<SrtRuntime>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SrtServerSrc {
        const NAME: &'static str = "KinugasaSrtServerSrc";
        type Type = super::SrtServerSrc;
        type ParentType = gst::Element;
    }

    impl ObjectImpl for SrtServerSrc {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("uri")
                        .nick("URI")
                        .blurb("SRT listener URI, for example srt://0.0.0.0:7001?mode=listener")
                        .default_value(DEFAULT_URI)
                        .build(),
                    glib::ParamSpecString::builder("url")
                        .nick("URL")
                        .blurb("Alias of uri")
                        .default_value(DEFAULT_URI)
                        .build(),
                    glib::ParamSpecString::builder("stream-ids")
                        .nick("Stream IDs")
                        .blurb("Comma-separated list of accepted SRT stream IDs")
                        .default_value("")
                        .build(),
                    glib::ParamSpecString::builder("passphrase")
                        .nick("Passphrase")
                        .blurb("SRT encryption passphrase")
                        .default_value("")
                        .build(),
                    glib::ParamSpecUInt::builder("pbkeylen")
                        .nick("PB key length")
                        .blurb("SRT crypto key length in bytes: 0, 16, 24, or 32")
                        .minimum(0)
                        .maximum(32)
                        .default_value(DEFAULT_PBKEYLEN)
                        .build(),
                    glib::ParamSpecInt::builder("latency")
                        .nick("Latency")
                        .blurb("SRT latency in milliseconds")
                        .minimum(0)
                        .maximum(i32::MAX)
                        .default_value(DEFAULT_LATENCY)
                        .build(),
                    glib::ParamSpecInt::builder("poll-timeout")
                        .nick("Poll timeout")
                        .blurb("SRT accept/read timeout in milliseconds")
                        .minimum(1)
                        .maximum(i32::MAX)
                        .default_value(DEFAULT_POLL_TIMEOUT)
                        .build(),
                    glib::ParamSpecUInt::builder("blocksize")
                        .nick("Block size")
                        .blurb("Maximum read block size")
                        .minimum(1)
                        .maximum(u32::MAX)
                        .default_value(DEFAULT_BLOCKSIZE)
                        .build(),
                    glib::ParamSpecBoolean::builder("do-timestamp")
                        .nick("Do timestamp")
                        .blurb("Apply current stream time to buffers")
                        .default_value(true)
                        .build(),
                ]
            })
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("add-stream-id")
                        .param_types([String::static_type()])
                        .action()
                        .class_handler(|args| {
                            let obj = args[0]
                                .get::<super::SrtServerSrc>()
                                .expect("signal receiver must be SrtServerSrc");
                            let stream_id =
                                args[1].get::<String>().expect("stream ID must be a string");
                            obj.imp().add_stream_id(&stream_id);
                            None
                        })
                        .build(),
                    glib::subclass::Signal::builder("remove-stream-id")
                        .param_types([String::static_type()])
                        .return_type::<bool>()
                        .action()
                        .class_handler(|args| {
                            let obj = args[0]
                                .get::<super::SrtServerSrc>()
                                .expect("signal receiver must be SrtServerSrc");
                            let stream_id =
                                args[1].get::<String>().expect("stream ID must be a string");
                            Some(obj.imp().remove_stream_id(&stream_id).to_value())
                        })
                        .build(),
                    glib::subclass::Signal::builder("clear-stream-ids")
                        .action()
                        .class_handler(|args| {
                            let obj = args[0]
                                .get::<super::SrtServerSrc>()
                                .expect("signal receiver must be SrtServerSrc");
                            obj.imp().clear_stream_ids();
                            None
                        })
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "uri" | "url" => {
                    self.settings.lock().unwrap().uri = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by GObject")
                        .unwrap_or_else(|| DEFAULT_URI.to_string());
                }
                "stream-ids" => {
                    let stream_ids = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by GObject")
                        .unwrap_or_default();
                    self.set_stream_ids(parse_stream_ids(&stream_ids));
                }
                "passphrase" => {
                    self.settings.lock().unwrap().passphrase = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by GObject")
                        .unwrap_or_default();
                }
                "pbkeylen" => {
                    let pbkeylen = value
                        .get::<u32>()
                        .expect("type conformity checked by GObject");
                    if matches!(pbkeylen, 0 | 16 | 24 | 32) {
                        self.settings.lock().unwrap().pbkeylen = pbkeylen;
                    } else {
                        gst::warning!(
                            gst::CAT_RUST,
                            imp = self,
                            "ignoring unsupported SRT pbkeylen {pbkeylen}"
                        );
                    }
                }
                "latency" => {
                    self.settings.lock().unwrap().latency =
                        value.get().expect("type conformity checked by GObject");
                }
                "poll-timeout" => {
                    self.settings.lock().unwrap().poll_timeout =
                        value.get().expect("type conformity checked by GObject");
                }
                "blocksize" => {
                    self.settings.lock().unwrap().blocksize =
                        value.get().expect("type conformity checked by GObject");
                }
                "do-timestamp" => {
                    self.settings.lock().unwrap().do_timestamp =
                        value.get().expect("type conformity checked by GObject");
                }
                _ => unreachable!("unknown property {}", pspec.name()),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "uri" | "url" => self.settings.lock().unwrap().uri.to_value(),
                "stream-ids" => join_stream_ids(&self.stream_ids()).to_value(),
                "passphrase" => self.settings.lock().unwrap().passphrase.to_value(),
                "pbkeylen" => self.settings.lock().unwrap().pbkeylen.to_value(),
                "latency" => self.settings.lock().unwrap().latency.to_value(),
                "poll-timeout" => self.settings.lock().unwrap().poll_timeout.to_value(),
                "blocksize" => self.settings.lock().unwrap().blocksize.to_value(),
                "do-timestamp" => self.settings.lock().unwrap().do_timestamp.to_value(),
                _ => unreachable!("unknown property {}", pspec.name()),
            }
        }
    }

    impl GstObjectImpl for SrtServerSrc {}

    impl ElementImpl for SrtServerSrc {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static METADATA: OnceLock<gst::subclass::ElementMetadata> = OnceLock::new();
            Some(METADATA.get_or_init(|| {
                gst::subclass::ElementMetadata::new(
                    "Kinugasa SRT Server Source",
                    "Source/Network",
                    "Receives SRT payloads and demuxes callers by stream ID",
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
                        gst::PadPresence::Sometimes,
                        &caps,
                    )
                    .unwrap(),
                    gst::PadTemplate::new(
                        "src_%s",
                        gst::PadDirection::Src,
                        gst::PadPresence::Sometimes,
                        &caps,
                    )
                    .unwrap(),
                ]
            })
        }

        fn change_state(
            &self,
            transition: gst::StateChange,
        ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
            if transition == gst::StateChange::NullToReady {
                self.start_runtime().map_err(|err| {
                    gst::element_error!(
                        self.obj(),
                        gst::ResourceError::OpenRead,
                        ["failed to start SRT demux source: {err}"]
                    );
                    gst::StateChangeError
                })?;
            }

            let result = self.parent_change_state(transition);

            if transition == gst::StateChange::ReadyToNull {
                self.stop_runtime();
            }

            result
        }
    }

    impl SrtServerSrc {
        fn start_runtime(&self) -> Result<(), String> {
            let mut runtime = self.runtime.lock().unwrap();
            if runtime.is_some() {
                return Ok(());
            }

            let settings = self.settings.lock().unwrap().clone();
            ensure_srt_startup()?;
            let runtime_started = SrtRuntime::start(
                settings,
                Arc::clone(&self.allowed_stream_ids),
                Arc::clone(&self.pads),
            )?;
            *runtime = Some(runtime_started);
            Ok(())
        }

        fn stop_runtime(&self) {
            self.runtime.lock().unwrap().take();
        }

        fn add_stream_id(&self, stream_id: &str) {
            let stream_id = stream_id.trim();
            if stream_id.is_empty() {
                return;
            }
            self.allowed_stream_ids
                .lock()
                .unwrap()
                .insert(stream_id.to_string());
            self.ensure_pad(stream_id);
            self.obj().notify("stream-ids");
        }

        fn remove_stream_id(&self, stream_id: &str) -> bool {
            let stream_id = stream_id.trim();
            let removed = self.allowed_stream_ids.lock().unwrap().remove(stream_id);
            if removed {
                if let Some(pad) = self.pads.lock().unwrap().remove(stream_id) {
                    let _ = pad.set_active(false);
                    let _ = self.obj().remove_pad(&pad);
                }
                self.pad_names.lock().unwrap().remove(stream_id);
                self.obj().notify("stream-ids");
            }
            removed
        }

        fn clear_stream_ids(&self) {
            for stream_id in self.stream_ids() {
                self.remove_stream_id(&stream_id);
            }
            self.obj().notify("stream-ids");
        }

        fn set_stream_ids(&self, stream_ids: Vec<String>) {
            let requested = stream_ids.iter().cloned().collect::<HashSet<_>>();
            let current = self.stream_ids().into_iter().collect::<HashSet<_>>();
            for stream_id in current.difference(&requested) {
                self.remove_stream_id(stream_id);
            }
            for stream_id in stream_ids {
                self.add_stream_id(&stream_id);
            }
        }

        pub(super) fn stream_ids(&self) -> Vec<String> {
            let mut stream_ids = self
                .allowed_stream_ids
                .lock()
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            stream_ids.sort();
            stream_ids
        }

        pub(super) fn pad_for_stream_id(&self, stream_id: &str) -> Option<gst::Pad> {
            self.pads.lock().unwrap().get(stream_id).cloned()
        }

        fn ensure_pad(&self, stream_id: &str) -> gst::Pad {
            if let Some(pad) = self.pads.lock().unwrap().get(stream_id).cloned() {
                return pad;
            }

            let pad_name = self.next_pad_name(stream_id);
            let pad = gst::Pad::builder(gst::PadDirection::Src)
                .name(&pad_name)
                .build();
            pad.set_active(true)
                .expect("failed to activate SRT demux src pad");
            self.obj()
                .add_pad(&pad)
                .expect("failed to add SRT demux src pad");
            self.pads
                .lock()
                .unwrap()
                .insert(stream_id.to_string(), pad.clone());
            self.pad_names
                .lock()
                .unwrap()
                .insert(stream_id.to_string(), pad_name);
            pad
        }

        fn next_pad_name(&self, stream_id: &str) -> String {
            if self.pad_names.lock().unwrap().is_empty() {
                return "src".to_string();
            }

            let base = format!("src_{}", sanitize_pad_name(stream_id));
            let existing = self
                .pad_names
                .lock()
                .unwrap()
                .values()
                .cloned()
                .collect::<HashSet<_>>();
            if !existing.contains(&base) {
                return base;
            }
            let mut index = 1;
            loop {
                let candidate = format!("{base}_{index}");
                if !existing.contains(&candidate) {
                    return candidate;
                }
                index += 1;
            }
        }
    }

    fn parse_stream_ids(value: &str) -> Vec<String> {
        let mut stream_ids = Vec::new();
        for stream_id in value.split(',').map(str::trim) {
            if !stream_id.is_empty() && !stream_ids.iter().any(|known| known == stream_id) {
                stream_ids.push(stream_id.to_string());
            }
        }
        stream_ids
    }

    fn join_stream_ids(stream_ids: &[String]) -> String {
        stream_ids.join(",")
    }

    fn sanitize_pad_name(stream_id: &str) -> String {
        stream_id
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '_'
                }
            })
            .collect()
    }

    struct SrtRuntime {
        listener: SrtSocket,
        epoll: c_int,
        stop: Arc<AtomicBool>,
        accept_thread: Option<thread::JoinHandle<()>>,
        read_threads: Arc<Mutex<Vec<thread::JoinHandle<()>>>>,
        callback_state: Box<ListenState>,
    }

    impl SrtRuntime {
        fn start(
            settings: Settings,
            allowed_stream_ids: Arc<Mutex<HashSet<String>>>,
            pads: Arc<Mutex<HashMap<String, gst::Pad>>>,
        ) -> Result<Self, String> {
            let bind_addr = parse_listener_addr(&settings.uri)?;
            let listener = create_listener(&settings, bind_addr)?;
            let listener_guard = SrtSocketGuard(listener);
            let stop = Arc::new(AtomicBool::new(false));
            let read_threads = Arc::new(Mutex::new(Vec::new()));
            let pending = Arc::new(Mutex::new(HashMap::new()));
            let mut callback_state = Box::new(ListenState {
                allowed_stream_ids,
                pending: Arc::clone(&pending),
            });

            ensure_srt_ok(
                unsafe {
                    srt_listen_callback(
                        listener,
                        listen_callback,
                        (&mut *callback_state as *mut ListenState).cast::<c_void>(),
                    )
                },
                "failed to set SRT listen callback",
            )?;
            ensure_srt_ok(
                unsafe { srt_listen(listener, 64) },
                "failed to listen for SRT callers",
            )?;
            let epoll = create_epoll(listener)?;
            let listener = listener_guard.into_inner();

            let accept_stop = Arc::clone(&stop);
            let accept_pending = Arc::clone(&pending);
            let accept_pads = Arc::clone(&pads);
            let accept_read_threads = Arc::clone(&read_threads);
            let accept_settings = settings.clone();

            let accept_thread = thread::spawn(move || {
                accept_loop(
                    listener,
                    epoll,
                    accept_settings,
                    accept_pending,
                    accept_pads,
                    accept_read_threads,
                    accept_stop,
                );
            });

            Ok(Self {
                listener,
                epoll,
                stop,
                accept_thread: Some(accept_thread),
                read_threads,
                callback_state,
            })
        }
    }

    impl Drop for SrtRuntime {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            unsafe {
                srt_close(self.listener);
            }
            if let Some(thread) = self.accept_thread.take() {
                let _ = thread.join();
            }
            for thread in self.read_threads.lock().unwrap().drain(..) {
                let _ = thread.join();
            }
            unsafe {
                srt_epoll_release(self.epoll);
            }
            let _ = &self.callback_state;
        }
    }

    struct ListenState {
        allowed_stream_ids: Arc<Mutex<HashSet<String>>>,
        pending: Arc<Mutex<HashMap<SrtSocket, String>>>,
    }

    extern "C" fn listen_callback(
        opaque: *mut c_void,
        sock: SrtSocket,
        _hs_version: c_int,
        _peeraddr: *const libc::sockaddr,
        stream_id: *const c_char,
    ) -> c_int {
        let Some(state) = (unsafe { opaque.cast::<ListenState>().as_ref() }) else {
            return -1;
        };
        let stream_id = if stream_id.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(stream_id) }.to_str().unwrap_or("")
        };
        if state.allowed_stream_ids.lock().unwrap().contains(stream_id) {
            state
                .pending
                .lock()
                .unwrap()
                .insert(sock, stream_id.to_owned());
            0
        } else {
            -1
        }
    }

    fn accept_loop(
        listener: SrtSocket,
        epoll: c_int,
        settings: Settings,
        pending: Arc<Mutex<HashMap<SrtSocket, String>>>,
        pads: Arc<Mutex<HashMap<String, gst::Pad>>>,
        read_threads: Arc<Mutex<Vec<thread::JoinHandle<()>>>>,
        stop: Arc<AtomicBool>,
    ) {
        while !stop.load(Ordering::Acquire) {
            let Some(sock) = accept_once(listener, epoll, settings.poll_timeout) else {
                continue;
            };
            let stream_id = pending
                .lock()
                .unwrap()
                .remove(&sock)
                .or_else(|| get_stream_id(sock));
            let Some(stream_id) = stream_id else {
                unsafe {
                    srt_close(sock);
                }
                continue;
            };
            let Some(pad) = pads.lock().unwrap().get(&stream_id).cloned() else {
                unsafe {
                    srt_close(sock);
                }
                continue;
            };
            let route = StreamRoute {
                stream_id,
                pad_name: pad.name().to_string(),
            };
            let _ = set_srt_flag(sock, SRTO_RCVTIMEO, &settings.poll_timeout);
            let read_stop = Arc::clone(&stop);
            let read_settings = settings.clone();
            read_threads.lock().unwrap().push(thread::spawn(move || {
                read_socket_to_pad(sock, route, pad, read_settings, read_stop);
            }));
        }
    }

    fn accept_once(listener: SrtSocket, epoll: c_int, timeout_ms: i32) -> Option<SrtSocket> {
        let mut read_sock = SRT_INVALID_SOCK;
        let mut read_count = 1;
        let mut write_count = 0;
        let mut local_read_count = 0;
        let mut local_write_count = 0;
        let poll_result = unsafe {
            srt_epoll_wait(
                epoll,
                &mut read_sock,
                &mut read_count,
                ptr::null_mut(),
                &mut write_count,
                timeout_ms.into(),
                ptr::null_mut(),
                &mut local_read_count,
                ptr::null_mut(),
                &mut local_write_count,
            )
        };
        if poll_result == SRT_ERROR || read_count == 0 {
            return None;
        }

        let mut addr: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
        let mut addr_len = std::mem::size_of::<libc::sockaddr_storage>() as c_int;
        let accepted = unsafe {
            srt_accept(
                listener,
                (&mut addr as *mut libc::sockaddr_storage).cast::<libc::sockaddr>(),
                &mut addr_len,
            )
        };
        if accepted == SRT_INVALID_SOCK {
            None
        } else {
            Some(accepted)
        }
    }

    fn read_socket_to_pad(
        sock: SrtSocket,
        route: StreamRoute,
        pad: gst::Pad,
        settings: Settings,
        stop: Arc<AtomicBool>,
    ) {
        push_initial_events(&pad, &route);
        let mut buffer = vec![0u8; settings.blocksize as usize];

        while !stop.load(Ordering::Acquire) {
            let len =
                unsafe { srt_recvmsg(sock, buffer.as_mut_ptr().cast(), buffer.len() as c_int) };
            if len > 0 {
                let mut gst_buffer = gst::Buffer::from_slice(buffer[..len as usize].to_vec());
                if settings.do_timestamp {
                    let buffer_ref = gst_buffer.get_mut().expect("new buffer is writable");
                    buffer_ref.set_pts(gst::ClockTime::ZERO);
                }
                match pad.push(gst_buffer) {
                    Ok(_) => {}
                    Err(
                        gst::FlowError::Flushing | gst::FlowError::Eos | gst::FlowError::NotLinked,
                    ) => break,
                    Err(_) => break,
                }
            } else if len == 0 {
                break;
            } else {
                let error = last_srt_error();
                if stop.load(Ordering::Acquire)
                    || error.contains("Connection was broken")
                    || error.contains("Connection does not exist")
                    || error.contains("Timeout")
                {
                    break;
                }
                break;
            }
        }

        unsafe {
            srt_close(sock);
        }
    }

    fn push_initial_events(pad: &gst::Pad, route: &StreamRoute) {
        let stream_start = format!(
            "srt-{}-{}",
            route.pad_name,
            sanitize_pad_name(&route.stream_id)
        );
        let _ = pad.push_event(gst::event::StreamStart::new(&stream_start));
        let segment = gst::FormattedSegment::<gst::ClockTime>::new();
        let _ = pad.push_event(gst::event::Segment::new(segment.as_ref()));
    }

    fn ensure_srt_startup() -> Result<(), String> {
        static STARTUP: OnceLock<Result<(), String>> = OnceLock::new();
        STARTUP
            .get_or_init(|| ensure_srt_ok(unsafe { srt_startup() }, "failed to initialize SRT"))
            .clone()
    }

    fn create_listener(settings: &Settings, bind_addr: SocketAddr) -> Result<SrtSocket, String> {
        let sock = unsafe { srt_create_socket() };
        if sock == SRT_INVALID_SOCK {
            return Err(format!("failed to create SRT socket: {}", last_srt_error()));
        }
        let guard = SrtSocketGuard(sock);

        let reuse = 1_i32;
        set_srt_flag(sock, SRTO_REUSEADDR, &reuse)?;
        let live = SRTT_LIVE;
        set_srt_flag(sock, SRTO_TRANSTYPE, &live)?;
        set_srt_flag(sock, SRTO_LATENCY, &settings.latency)?;
        set_srt_flag(sock, SRTO_RCVTIMEO, &settings.poll_timeout)?;
        if !settings.passphrase.is_empty() {
            let passphrase = CString::new(settings.passphrase.as_str())
                .map_err(|_| "SRT passphrase contains an interior NUL byte".to_string())?;
            ensure_srt_ok(
                unsafe {
                    srt_setsockflag(
                        sock,
                        SRTO_PASSPHRASE,
                        passphrase.as_ptr().cast::<c_void>(),
                        settings.passphrase.len() as c_int,
                    )
                },
                "failed to set SRT passphrase",
            )?;
            set_srt_flag(sock, SRTO_PBKEYLEN, &(settings.pbkeylen as i32))?;
        }

        let sockaddr = sockaddr_in(bind_addr)?;
        ensure_srt_ok(
            unsafe {
                srt_bind(
                    sock,
                    (&sockaddr as *const libc::sockaddr_in).cast::<libc::sockaddr>(),
                    std::mem::size_of::<libc::sockaddr_in>() as c_int,
                )
            },
            "failed to bind SRT listener",
        )?;
        Ok(guard.into_inner())
    }

    fn create_epoll(listener: SrtSocket) -> Result<c_int, String> {
        let epoll = unsafe { srt_epoll_create() };
        if epoll == SRT_ERROR {
            return Err(format!("failed to create SRT epoll: {}", last_srt_error()));
        }
        let guard = SrtEpollGuard(epoll);
        let events = SRT_EPOLL_IN | SRT_EPOLL_ERR;
        ensure_srt_ok(
            unsafe { srt_epoll_add_usock(epoll, listener, &events) },
            "failed to add SRT listener to epoll",
        )?;
        Ok(guard.into_inner())
    }

    struct SrtSocketGuard(SrtSocket);

    impl SrtSocketGuard {
        fn into_inner(mut self) -> SrtSocket {
            let sock = self.0;
            self.0 = SRT_INVALID_SOCK;
            sock
        }
    }

    impl Drop for SrtSocketGuard {
        fn drop(&mut self) {
            if self.0 != SRT_INVALID_SOCK {
                unsafe {
                    srt_close(self.0);
                }
            }
        }
    }

    struct SrtEpollGuard(c_int);

    impl SrtEpollGuard {
        fn into_inner(mut self) -> c_int {
            let epoll = self.0;
            self.0 = SRT_ERROR;
            epoll
        }
    }

    impl Drop for SrtEpollGuard {
        fn drop(&mut self) {
            if self.0 != SRT_ERROR {
                unsafe {
                    srt_epoll_release(self.0);
                }
            }
        }
    }

    fn parse_listener_addr(url: &str) -> Result<SocketAddr, String> {
        let rest = url
            .strip_prefix("srt://")
            .ok_or_else(|| "uri must start with srt://".to_string())?;
        let authority = rest.split('?').next().unwrap_or(rest);
        let authority = authority.strip_suffix('/').unwrap_or(authority);
        let (host, port) = authority
            .rsplit_once(':')
            .ok_or_else(|| "uri must include a listener port".to_string())?;
        let host = if host.is_empty() { "0.0.0.0" } else { host };
        let port = port
            .parse::<u16>()
            .map_err(|_| "uri port must be a u16".to_string())?;
        let ip = host
            .parse::<Ipv4Addr>()
            .map_err(|_| format!("SRT demux listener only supports IPv4 hosts: {host}"))?;
        Ok(SocketAddr::from((ip, port)))
    }

    fn sockaddr_in(addr: SocketAddr) -> Result<libc::sockaddr_in, String> {
        let SocketAddr::V4(addr) = addr else {
            return Err("SRT demux listener only supports IPv4 addresses".to_string());
        };
        Ok(libc::sockaddr_in {
            sin_family: libc::AF_INET as libc::sa_family_t,
            sin_port: addr.port().to_be(),
            sin_addr: libc::in_addr {
                s_addr: u32::from_ne_bytes(addr.ip().octets()),
            },
            sin_zero: [0; 8],
        })
    }

    fn set_srt_flag<T>(sock: SrtSocket, option: c_int, value: &T) -> Result<(), String> {
        ensure_srt_ok(
            unsafe {
                srt_setsockflag(
                    sock,
                    option,
                    (value as *const T).cast::<c_void>(),
                    std::mem::size_of::<T>() as c_int,
                )
            },
            "failed to set SRT socket option",
        )
    }

    fn get_stream_id(sock: SrtSocket) -> Option<String> {
        let mut buffer = [0_i8; 512];
        let mut len = buffer.len() as c_int;
        let result = unsafe {
            srt_getsockflag(
                sock,
                SRTO_STREAMID,
                buffer.as_mut_ptr().cast::<c_void>(),
                &mut len,
            )
        };
        if result == SRT_ERROR || len <= 0 {
            return None;
        }
        let len = buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(len as usize);
        if len == 0 {
            return None;
        }
        Some(
            buffer[..len]
                .iter()
                .map(|byte| *byte as u8)
                .collect::<Vec<_>>(),
        )
        .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    fn ensure_srt_ok(code: c_int, context: &str) -> Result<(), String> {
        if code == SRT_ERROR {
            Err(format!("{context}: {}", last_srt_error()))
        } else {
            Ok(())
        }
    }

    fn last_srt_error() -> String {
        let message = unsafe { srt_getlasterror_str() };
        if message.is_null() {
            return "unknown SRT error".to_string();
        }
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }
}
