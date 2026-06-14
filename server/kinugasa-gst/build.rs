fn main() {
    pkg_config::Config::new()
        .probe("srt")
        .expect("failed to find SRT with pkg-config");
}
