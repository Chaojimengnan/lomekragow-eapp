fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/image-viewer/icon.ico")
        .compile()
        .unwrap();
}
