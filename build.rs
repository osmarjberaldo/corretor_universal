fn main() {
    #[cfg(windows)]
    {
        let out_dir = match std::env::var("OUT_DIR") {
            Ok(d) => d,
            Err(_) => return,
        };
        let out_obj = std::path::Path::new(&out_dir).join("icone.o");

        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let w64_windres = format!(r"{}\tools\w64devkit\w64devkit\bin\x86_64-w64-mingw32-windres.exe", home);

        let windres_cmd = if std::path::Path::new(&w64_windres).exists() {
            w64_windres
        } else if std::process::Command::new("x86_64-w64-mingw32-windres").arg("--version").output().is_ok() {
            "x86_64-w64-mingw32-windres".to_string()
        } else {
            "windres".to_string()
        };

        let status = std::process::Command::new(&windres_cmd)
            .args(&["-i", "icone.rc", "-O", "coff", "-o"])
            .arg(&out_obj)
            .status();

        if let Ok(s) = status {
            if s.success() {
                println!("cargo:rustc-link-arg={}", out_obj.display());
            }
        }
    }
    println!("cargo:rerun-if-changed=icone.rc");
    println!("cargo:rerun-if-changed=icone.ico");
}
