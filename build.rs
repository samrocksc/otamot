fn main() {
    // Embed icon in Windows executable
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");
        res.compile().unwrap();
    }

    // Tell Cargo to re-run this script if the icon changes
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.png");
}
