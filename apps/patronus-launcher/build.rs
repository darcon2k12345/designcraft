//! Windows only: embed the launcher icon and version info (VERSIONINFO) into both executables, so
//! they show in Explorer, the taskbar and Alt-Tab. A missing resource compiler is only a warning
//! (a cross-compile still links) unless `PATRONUS_REQUIRE_WINRES=1` makes it an error.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/launcher.ico");
    println!("cargo:rerun-if-env-changed=PATRONUS_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/launcher.ico")
        .set("ProductName", "Patronus Launcher")
        .set("FileDescription", "Patronus Launcher: install and update the Patronus apps")
        .set("CompanyName", "Patronus")
        .set("LegalCopyright", "Copyright (c) the Patronus authors. MIT OR Apache-2.0.")
        .set("InternalName", "patronus-launcher");
    if let Err(e) = res.compile() {
        if std::env::var_os("PATRONUS_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=the launcher was built without icon/version resources: {e}");
    }
}
