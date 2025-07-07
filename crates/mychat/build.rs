fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/mychat/icon.ico")
        .compile()
        .unwrap();
}
