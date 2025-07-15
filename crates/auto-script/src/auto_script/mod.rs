pub(crate) mod binding;
pub(crate) mod script_editor;
pub(crate) mod script_executor;
pub(crate) mod script_manager;

pub const DEFAULT_SCRIPT_CONTENTS: &str = r#"local gui = AutoScript.new()
"#;

pub const GUI_METHODS: &[(&str, &str)] = &[
    ("get_screen_size", "get_screen_size() -> (w, h)"),
    ("move_mouse_to", "move_mouse_to(x, y, speed)"),
    ("sleep", "sleep(seconds)"),
    ("click", "click(x, y)"),
    ("store_image", "store_image(path, tbl, mode, alias)"),
];
