use std::{path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-env-changed=DEVELOPER_DIR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    // ScreenCaptureKit's Swift dependencies assume a full Xcode directory.
    // Resolve the actual active toolchain for both Xcode and Command Line Tools.
    let output = Command::new("xcrun")
        .args(["--find", "swiftc"])
        .output()
        .expect("macOS capture needs Apple Command Line Tools");
    assert!(output.status.success(), "xcrun could not locate swiftc");
    let swiftc = PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("UTF-8 Swift path")
            .trim(),
    );
    let libraries = swiftc
        .parent()
        .and_then(|path| path.parent())
        .expect("Swift toolchain path")
        .join("lib/swift/macosx");
    assert!(
        libraries.is_dir(),
        "Swift runtime libraries missing at {}",
        libraries.display()
    );
    println!("cargo:rustc-link-search=native={}", libraries.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libraries.display());
}
