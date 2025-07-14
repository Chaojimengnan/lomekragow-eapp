fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/auto-script/icon.ico")
        .compile()
        .unwrap();
}
