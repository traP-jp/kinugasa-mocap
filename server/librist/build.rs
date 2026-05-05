fn main() {
    let librist_src_path = load_path("LIBRIST_SRC");
    let out_path = load_path("OUT_DIR");
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let build_path = temp_dir.path().join("build");
    let lib_path = out_path.join("lib");
    std::fs::create_dir_all(&lib_path).expect("Failed to create lib output directory");

    std::process::Command::new("meson")
        .arg("setup")
        .arg("build")
        .arg(&librist_src_path)
        .arg("--default-library=static")
        .arg("-Dtest=false")
        .arg("-Dbuilt_tools=false")
        .current_dir(temp_dir.path())
        .status()
        .expect("Failed to execute meson")
        .success()
        .then_some(())
        .expect("Meson returned non-zero exit code");
    std::process::Command::new("ninja")
        .arg("librist.a")
        .current_dir(&build_path)
        .status()
        .expect("Failed to execute ninja")
        .success()
        .then_some(())
        .expect("Ninja returned non-zero exit code");
    std::fs::copy(build_path.join("librist.a"), lib_path.join("librist.a"))
        .expect("Failed to copy librist static library");

    bindgen::Builder::default()
        .header_contents("wrapper.h", "#include <librist/librist.h>")
        .clang_arg(format!("-I{}/include", librist_src_path.display()))
        .clang_arg(format!("-I{}/include/librist", build_path.display()))
        .allowlist_type("rist_.*")
        .allowlist_function("rist_.*")
        .allowlist_var("RIST_.*")
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file(out_path.join("librist.rs"))
        .expect("Failed to write bindings to file");

    println!("cargo:rerun-if-env-changed=LIBRIST_SRC");
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=rist");
}

fn load_path(env_var: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var(env_var)
            .unwrap_or_else(|_| panic!("{} environment variable is not set", env_var)),
    )
}
