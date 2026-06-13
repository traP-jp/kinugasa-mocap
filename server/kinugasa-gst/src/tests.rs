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
