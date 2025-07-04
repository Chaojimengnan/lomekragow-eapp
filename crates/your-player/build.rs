fn main() {
    #[cfg(windows)]
    {
        winres::WindowsResource::new()
            .set_icon("../../assets/your-player/icon.ico")
            .compile()
            .unwrap();

        let mpv_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo::rustc-link-search={mpv_path}/mpv/");
    }
}
