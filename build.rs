//! Build script: re-run when content/assets change, and on Windows convert the
//! WebP app icon to a .ico (in OUT_DIR only, nothing committed) and embed it.
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=content");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let ico_path = out.join("giyur-study.ico");
        let img = image::open("assets/icon/giyur-study-256.webp").expect("icon webp");
        img.save_with_format(&ico_path, image::ImageFormat::Ico)
            .expect("write ico");
        #[cfg(windows)]
        embed(&ico_path);
        #[cfg(not(windows))]
        embed_cross(&ico_path, &out);
    }
}

#[cfg(windows)]
fn embed(ico: &std::path::Path) {
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().unwrap());
    res.set("ProductName", "Giyur Study");
    res.set("FileDescription", "Giyur Study");
    res.compile().expect("winresource");
}

/// Cross-compiling from Linux with mingw: compile a tiny .rc with windres.
#[cfg(not(windows))]
fn embed_cross(ico: &std::path::Path, out: &std::path::Path) {
    let rc = out.join("app.rc");
    std::fs::write(
        &rc,
        format!("1 ICON \"{}\"\n", ico.display().to_string().replace('\\', "/")),
    )
    .unwrap();
    let windres = std::env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".into());
    let obj = out.join("app-res.o");
    let ok = std::process::Command::new(&windres)
        .arg(&rc)
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(&obj)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("cargo:rustc-link-arg-bins={}", obj.display());
    } else {
        println!("cargo:warning=windres not found: .exe built without an embedded icon");
    }
}
