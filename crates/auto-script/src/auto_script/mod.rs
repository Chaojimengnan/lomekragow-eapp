pub(crate) mod binding;
pub(crate) mod console;
pub(crate) mod script_editor;
pub(crate) mod script_executor;
pub(crate) mod script_manager;

pub const DEFAULT_SCRIPT_CONTENTS: &str = r#"local gui = AutoScript.new()
"#;

pub const GUI_METHODS: &[(&str, &str, &str)] = &[
    // ----- Mouse clicks -----
    (
        "click",
        "click(button: \"left\" | \"right\" | \"middle\")",
        "Perform a single click with the specified mouse button",
    ),
    (
        "click_down",
        "click_down(button: \"left\" | \"right\" | \"middle\")",
        "Press down the specified mouse button (without release)",
    ),
    (
        "click_up",
        "click_up(button: \"left\" | \"right\" | \"middle\")",
        "Release the specified mouse button",
    ),
    (
        "double_click",
        "double_click()",
        "Perform a double-click with the left mouse button",
    ),
    ("left_click", "left_click()", "Perform a left-click"),
    ("right_click", "right_click()", "Perform a right-click"),
    ("middle_click", "middle_click()", "Perform a middle-click"),
    // ----- Mouse movement -----
    (
        "move_mouse",
        "move_mouse(x: integer, y: integer, t: float)",
        "Move mouse by an offset (x, y) over t seconds",
    ),
    (
        "move_mouse_to_pos",
        "move_mouse_to_pos(x: integer, y: integer, t: float)",
        "Move mouse to absolute position (x, y) over t seconds",
    ),
    (
        "move_mouse_to",
        "move_mouse_to(x?: integer, y?: integer, t: float)",
        "Move mouse to the specified x and/or y over t seconds; if x or y is None, that coordinate remains unchanged",
    ),
    (
        "drag_mouse",
        "drag_mouse(x: integer, y: integer, t: float)",
        "Drag mouse by an offset (x, y) over t seconds",
    ),
    (
        "drag_mouse_to_pos",
        "drag_mouse_to_pos(x: integer, y: integer, t: float)",
        "Drag mouse to absolute position (x, y) over t seconds",
    ),
    (
        "drag_mouse_to",
        "drag_mouse_to(x?: integer, y?: integer, t: float)",
        "Drag mouse to the specified x and/or y over t seconds; if x or y is None, that coordinate remains unchanged",
    ),
    // ----- Scrolling -----
    ("scroll_up", "scroll_up(n: integer)", "Scroll up by n units"),
    (
        "scroll_down",
        "scroll_down(n: integer)",
        "Scroll down by n units",
    ),
    (
        "scroll_left",
        "scroll_left(n: integer)",
        "Scroll left by n units",
    ),
    (
        "scroll_right",
        "scroll_right(n: integer)",
        "Scroll right by n units",
    ),
    // ----- Keyboard -----
    (
        "key_down",
        "key_down(key: string)",
        "Press and hold the specified key (e.g., \"a\", \"Enter\")",
    ),
    ("key_up", "key_up(key: string)", "Release the specified key"),
    (
        "keyboard_input",
        "keyboard_input(text: string)",
        "Type the given text string",
    ),
    (
        "keyboard_command",
        "keyboard_command(cmd: string)",
        "Send a key combination command (e.g., \"Control+C\")",
    ),
    (
        "keyboard_multi_key",
        "keyboard_multi_key(a: string, b: string, c?: string)",
        "Press multiple keys in combination (a + b [+ c])",
    ),
    // ----- Utilities -----
    (
        "get_mouse_position",
        "get_mouse_position() -> (x: integer, y: integer)",
        "Get the current mouse cursor position",
    ),
    (
        "get_screen_size",
        "get_screen_size() -> (width: integer, height: integer)",
        "Get the screen resolution",
    ),
    (
        "sleep",
        "sleep(seconds: float)",
        "Pause script execution for the given number of seconds",
    ),
    // ----- Image templates -----
    (
        "store_image",
        "store_image(path: string, region?: table, mode: string, alias: string)",
        "Load an image template from file. \
         `region` is an optional table `{x, y, w, h}` specifying the sub‐rectangle to use. \
         `mode` must be \"FFT\" or \"Segmented\". \
         Stores the template under the given alias.",
    ),
    // ----- Image finding -----
    (
        "find_image_on_screen",
        "find_image_on_screen(precision: float, alias: string) -> table?",
        "Search the screen for a stored template by alias at given precision. \
         Returns a Lua array of match tables, or nil if no match. \
         Each match table has fields:\n\
         • `x`: left coordinate of match (u32)\n\
         • `y`: top coordinate of match (u32)\n\
         • `score`: match confidence (0.0–1.0 float).",
    ),
    (
        "find_image_on_screen_and_move",
        "find_image_on_screen_and_move(precision: float, time: float, alias: string) -> table?",
        "Same as `find_image_on_screen`, but also moves the mouse to the first match over `time` seconds. \
         Returns the same array of `{ x, y, score }` tables or nil.",
    ),
    (
        "loop_find_image_on_screen",
        "loop_find_image_on_screen(precision: float, timeout: integer, alias: string) -> table?",
        "Repeatedly search the screen until a match is found or `timeout` ms elapses. \
         Returns the array of `{ x, y, score }` tables or nil on timeout.",
    ),
    (
        "loop_find_image_on_screen_and_move",
        "loop_find_image_on_screen_and_move(precision: float, time: float, timeout: integer, alias: string) -> table?",
        "Combined behavior of `loop_find_image_on_screen` and moving the mouse: \
         keep searching until success or timeout, then move the mouse to the first match over `time` seconds. \
         Returns the array of `{ x, y, score }` tables or nil.",
    ),
];

pub static SNIPPETS: &[(&str, &str, &str)] = &[
    // === Variables ===
    ("local", "local name = value", "Define a local variable"),
    ("assign", "name = value", "Assign a value to a variable"),
    // === Control Flow ===
    ("if", "if condition then\n    \nend", "If statement"),
    (
        "ifelse",
        "if condition then\n    \nelse\n    \nend",
        "If-else statement",
    ),
    (
        "elseif",
        "elseif condition then\n    \n",
        "Else-if branch (used inside if)",
    ),
    ("while", "while condition do\n    \nend", "While loop"),
    (
        "repeat",
        "repeat\n    \nuntil condition",
        "Repeat-until loop (runs at least once)",
    ),
    ("fori", "for i = 1, 10 do\n    \nend", "Numeric for loop"),
    (
        "forpairs",
        "for k, v in pairs(tbl) do\n    \nend",
        "Generic for loop (table traversal)",
    ),
    (
        "ipairs",
        "for i, v in ipairs(tbl) do\n    \nend",
        "Array-style table traversal",
    ),
    // === Functions ===
    (
        "function",
        "function name(...)\n    \nend",
        "Function definition",
    ),
    (
        "localfunc",
        "local function name(...)\n    \nend",
        "Local function definition",
    ),
    ("return", "return value", "Return a value from a function"),
    // === Printing / Debugging ===
    ("print", "print(\"value\")", "Print to console"),
    ("tostring", "tostring(value)", "Convert a value to string"),
    // === Table Operations ===
    (
        "table",
        "local t = {\n    key = value,\n}",
        "Define a table",
    ),
    ("table_set", "t.key = value", "Set a field in a table"),
    (
        "table_get",
        "local val = t.key",
        "Access a field in a table",
    ),
    (
        "table_insert",
        "table.insert(t, value)",
        "Insert value into table (append)",
    ),
    (
        "table_remove",
        "table.remove(t, index)",
        "Remove a value from table by index",
    ),
    ("table_len", "#t", "Get the length of a table (array part)"),
    // === Modules / Imports ===
    (
        "require",
        "local m = require(\"module\")",
        "Import a module",
    ),
    (
        "module",
        "local M = {}\n\nfunction M.foo()\n    \nend\n\nreturn M",
        "Define a simple Lua module",
    ),
    // === Error Handling ===
    (
        "pcall",
        "local ok, result = pcall(function()\n    \nend)",
        "Call a function with error protection (pcall)",
    ),
    // === Misc Utilities ===
    ("type", "type(value)", "Get the type of a value"),
    (
        "pairs",
        "for k, v in pairs(t) do\n    \nend",
        "Iterate over key-value pairs in a table",
    ),
    (
        "ipairs",
        "for i, v in ipairs(t) do\n    \nend",
        "Iterate over array-style table entries",
    ),
];
