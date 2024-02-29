fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/script-caller/icon.ico")
        .compile()
        .unwrap();
}
