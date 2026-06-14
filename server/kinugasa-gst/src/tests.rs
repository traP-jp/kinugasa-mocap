use gst::glib;
use gst::prelude::*;

#[test]
fn registers_rist_server_src_element() {
    gst::init().unwrap();
    crate::rist::register(None).unwrap();

    let element = gst::ElementFactory::make("ristserversrc")
        .property("url", "rist://@127.0.0.1:1234")
        .build()
        .unwrap();

    assert!(element.is::<crate::rist::RistServerSrc>());
    assert_eq!(element.property::<String>("profile"), "simple");
}

#[test]
fn configures_rist_server_src_properties() {
    gst::init().unwrap();

    let element = glib::Object::builder::<crate::rist::RistServerSrc>()
        .property("url", "rist://@127.0.0.1:1234")
        .property("profile", "main")
        .property("read-timeout-ms", 25_u32)
        .build();

    assert_eq!(element.property::<String>("url"), "rist://@127.0.0.1:1234");
    assert_eq!(element.property::<String>("profile"), "main");
    assert_eq!(element.property::<u32>("read-timeout-ms"), 25);
}

#[test]
fn registers_srt_server_src_element() {
    gst::init().unwrap();
    crate::srt::register(None).unwrap();

    let element = gst::ElementFactory::make("kinugasasrtserversrc")
        .property("uri", "srt://0.0.0.0:1234?mode=listener")
        .property("stream-ids", "camera-a")
        .build()
        .unwrap();

    assert!(element.is::<crate::srt::SrtServerSrc>());
    assert_eq!(
        element.property::<String>("uri"),
        "srt://0.0.0.0:1234?mode=listener"
    );
    assert_eq!(element.property::<String>("stream-ids"), "camera-a");
    assert_eq!(element.property::<u32>("pbkeylen"), 32);
}

#[test]
fn updates_srt_server_src_stream_ids_dynamically() {
    gst::init().unwrap();

    let element = glib::Object::builder::<crate::srt::SrtServerSrc>()
        .property("stream-ids", "camera-b,camera-a")
        .build();

    assert_eq!(element.stream_ids(), ["camera-a", "camera-b"]);

    element.add_stream_id("camera-c");
    assert_eq!(element.stream_ids(), ["camera-a", "camera-b", "camera-c"]);

    assert!(element.remove_stream_id("camera-b"));
    assert_eq!(element.stream_ids(), ["camera-a", "camera-c"]);

    element.clear_stream_ids();
    assert!(element.stream_ids().is_empty());
}

#[test]
fn links_srt_server_src_ghost_pad_to_child_source() {
    gst::init().unwrap();

    let source = glib::Object::builder::<crate::srt::SrtServerSrc>()
        .property("stream-ids", "camera-a")
        .build();
    let sink = gst::ElementFactory::make("fakesink").build().unwrap();
    let pipeline = gst::Pipeline::new();
    pipeline.add_many([source.upcast_ref(), &sink]).unwrap();

    source.link(&sink).unwrap();

    assert!(source.static_pad("src").unwrap().is_linked());
    assert_eq!(source.pad_for_stream_id("camera-a").unwrap().name(), "src");
}

#[test]
fn creates_srt_demux_pad_per_stream_id() {
    gst::init().unwrap();

    let source = glib::Object::builder::<crate::srt::SrtServerSrc>()
        .property("stream-ids", "camera-a,camera-b")
        .build();

    assert_eq!(source.pad_for_stream_id("camera-a").unwrap().name(), "src");
    assert_eq!(
        source.pad_for_stream_id("camera-b").unwrap().name(),
        "src_camera-b"
    );
}
