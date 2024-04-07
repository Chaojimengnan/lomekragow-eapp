fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/lonote/icon.ico")
        .compile()
        .unwrap();
}
