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
        let ico_path = out.join("shorashim.ico");
        let img = image::open("assets/icon/shorashim-256.webp").expect("icon webp");
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
    res.set("ProductName", "Шорашим (Shorashim)");
    res.set(
        "FileDescription",
        "Шорашим — основы иудаизма и подготовка к гиюру",
    );
    res.set("OriginalFilename", "shorashim.exe");
    res.set("InternalName", "shorashim");
    res.compile().expect("winresource");
}

/// Cross-compiling from Linux with mingw: compile a tiny .rc with windres.
#[cfg(not(windows))]
fn embed_cross(ico: &std::path::Path, out: &std::path::Path) {
    let rc = out.join("app.rc");
    std::fs::write(
        &rc,
        format!(
            "#pragma code_page(65001)\n1 ICON \"{ico}\"\n\
1 VERSIONINFO\nFILEVERSION {v},0\nPRODUCTVERSION {v},0\nBEGIN\n\
BLOCK \"StringFileInfo\"\nBEGIN\nBLOCK \"040904B0\"\nBEGIN\n\
VALUE \"ProductName\", \"Шорашим (Shorashim)\"\n\
VALUE \"FileDescription\", \"Шорашим — основы иудаизма и подготовка к гиюру\"\n\
VALUE \"OriginalFilename\", \"shorashim.exe\"\n\
VALUE \"InternalName\", \"shorashim\"\n\
VALUE \"FileVersion\", \"{ver}\"\nVALUE \"ProductVersion\", \"{ver}\"\n\
VALUE \"LegalCopyright\", \"Apache-2.0\"\nEND\nEND\n\
BLOCK \"VarFileInfo\"\nBEGIN\nVALUE \"Translation\", 0x409, 1200\nEND\nEND\n",
            ico = ico.display().to_string().replace('\\', "/"),
            ver = env!("CARGO_PKG_VERSION"),
            v = env!("CARGO_PKG_VERSION").replace('.', ","),
        ),
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
