fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/syncer/icon.ico")
        .compile()
        .unwrap();
}
