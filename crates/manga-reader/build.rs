fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/manga-reader/icon.ico")
        .compile()
        .unwrap();
}
