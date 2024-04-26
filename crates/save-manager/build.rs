fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("../../assets/save-manager/icon.ico")
        .compile()
        .unwrap();
}
