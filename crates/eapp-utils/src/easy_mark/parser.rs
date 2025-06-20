//! A parser for `EasyMark`: a very simple markup language.
//!
//! WARNING: `EasyMark` is subject to change.
//
//! # `EasyMark` design goals:
//! 1. easy to parse
//! 2. easy to learn
//! 3. similar to markdown

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item {
    /// `\n`
    // TODO(emilk): add Style here so empty heading still uses up the right amount of space.
    Newline,

    /// text
    Text(Style, String),

    /// title, url
    Hyperlink(Style, String, String),

    /// leading space before e.g. a [`Self::BulletPoint`].
    Indentation(usize),

    /// >
    QuoteIndent,

    /// - a point well made.
    BulletPoint,

    /// 1. numbered list. The string is the number(s).
    NumberedPoint(String),

    /// ---
    Separator,

    /// language, code
    CodeBlock(String, String),
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    /// # heading (large text)
    pub heading: bool,

    /// > quoted (slightly dimmer color or other font style)
    pub quoted: bool,

    /// `code` (monospace, some other color)
    pub code: bool,

    /// self.strong* (emphasized, e.g. bold)
    pub strong: bool,

    /// _underline_
    pub underline: bool,

    /// ~strikethrough~
    pub strikethrough: bool,

    /// /italics/
    pub italics: bool,

    /// $small$
    pub small: bool,

    /// ^raised^
    pub raised: bool,
}

/// Parser for the `EasyMark` markup language.
///
/// See the module-level documentation for details.
///
/// # Example:
/// ```
/// # use egui_demo_lib::easy_mark::parser::Parser;
/// for item in Parser::new("Hello *world*!") {
/// }
///
/// ```
pub struct Parser<'a> {
    /// The remainder of the input text
    s: &'a str,

    /// Are we at the start of a line?
    start_of_line: bool,

    /// Current self.style. Reset after a newline.
    style: Style,
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            s,
            start_of_line: true,
            style: Style::default(),
        }
    }

    /// `1. `, `42. ` etc.
    fn numbered_list(&mut self) -> Option<Item> {
        let n_digits = self.s.chars().take_while(|c| c.is_ascii_digit()).count();
        if n_digits > 0 && self.s.chars().skip(n_digits).take(2).eq(". ".chars()) {
            let number = &self.s[..n_digits];
            self.s = &self.s[(n_digits + 2)..];
            self.start_of_line = false;
            return Some(Item::NumberedPoint(number.to_owned()));
        }
        None
    }

    // ```{language}\n{code}```
    fn code_block(&mut self) -> Option<Item> {
        if let Some(language_start) = self.s.strip_prefix("```") {
            if let Some(newline) = language_start.find('\n') {
                let language = &language_start[..newline];
                let code_start = &language_start[newline + 1..];
                if let Some(end) = code_start.find("\n```") {
                    let code = code_start[..end].trim();
                    self.s = &code_start[end + 4..];
                    self.start_of_line = false;
                    return Some(Item::CodeBlock(language.to_owned(), code.to_owned()));
                }

                self.s = "";
                return Some(Item::CodeBlock(language.to_owned(), code_start.to_owned()));
            }
        }
        None
    }

    // `code`
    fn inline_code(&mut self) -> Option<Item> {
        if let Some(rest) = self.s.strip_prefix('`') {
            self.s = rest;
            self.start_of_line = false;
            self.style.code = true;
            let rest_of_line = &self.s[..self.s.find('\n').unwrap_or(self.s.len())];
            if let Some(end) = rest_of_line.find('`') {
                let item = Item::Text(self.style, self.s[..end].to_owned());
                self.s = &self.s[end + 1..];
                self.style.code = false;
                return Some(item);
            }
            let end = rest_of_line.len();
            let item = Item::Text(self.style, rest_of_line.to_owned());
            self.s = &self.s[end..];
            self.style.code = false;
            return Some(item);
        }
        None
    }

    /// `<url>` or `[link](url)`
    fn url(&mut self) -> Option<Item> {
        if self.s.starts_with('<') {
            let this_line = &self.s[..self.s.find('\n').unwrap_or(self.s.len())];
            if let Some(url_end) = this_line.find('>') {
                let url = &self.s[1..url_end];
                self.s = &self.s[url_end + 1..];
                self.start_of_line = false;
                return Some(Item::Hyperlink(self.style, url.to_owned(), url.to_owned()));
            }
        }

        // [text](url)
        if self.s.starts_with('[') {
            let this_line = &self.s[..self.s.find('\n').unwrap_or(self.s.len())];
            if let Some(bracket_end) = this_line.find(']') {
                let text = &this_line[1..bracket_end];
                if this_line[bracket_end + 1..].starts_with('(') {
                    if let Some(parens_end) = this_line[bracket_end + 2..].find(')') {
                        let parens_end = bracket_end + 2 + parens_end;
                        let url = &self.s[bracket_end + 2..parens_end];
                        self.s = &self.s[parens_end + 1..];
                        self.start_of_line = false;
                        return Some(Item::Hyperlink(self.style, text.to_owned(), url.to_owned()));
                    }
                }
            }
        }
        None
    }
}

impl Iterator for Parser<'_> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.s.is_empty() {
                return None;
            }

            // \n
            if self.s.starts_with('\n') {
                self.s = &self.s[1..];
                self.start_of_line = true;
                self.style = Style::default();
                return Some(Item::Newline);
            }

            // Ignore line break (continue on the same line)
            if self.s.starts_with("\\\n") && self.s.len() >= 2 {
                self.s = &self.s[2..];
                self.start_of_line = false;
                continue;
            }

            // \ escape (to show e.g. a backtick)
            if self.s.starts_with('\\') && self.s.len() >= 2 {
                let text = &self.s[1..2];
                self.s = &self.s[2..];
                self.start_of_line = false;
                return Some(Item::Text(self.style, text.to_owned()));
            }

            if self.start_of_line {
                // leading space (indentation)
                if self.s.starts_with(' ') {
                    let length = self.s.find(|c| c != ' ').unwrap_or(self.s.len());
                    self.s = &self.s[length..];
                    self.start_of_line = true; // indentation doesn't count
                    return Some(Item::Indentation(length));
                }

                // # Heading
                if let Some(after) = self.s.strip_prefix("# ") {
                    self.s = after;
                    self.start_of_line = false;
                    self.style.heading = true;
                    continue;
                }

                // > quote
                if let Some(after) = self.s.strip_prefix("> ") {
                    self.s = after;
                    self.start_of_line = true; // quote indentation doesn't count
                    self.style.quoted = true;
                    return Some(Item::QuoteIndent);
                }

                // - bullet point
                if self.s.starts_with("- ") {
                    self.s = &self.s[2..];
                    self.start_of_line = false;
                    return Some(Item::BulletPoint);
                }

                // `1. `, `42. ` etc.
                if let Some(item) = self.numbered_list() {
                    return Some(item);
                }

                // --- separator
                if let Some(after) = self.s.strip_prefix("---") {
                    self.s = after.trim_start_matches('-'); // remove extra dashes
                    self.s = self.s.strip_prefix('\n').unwrap_or(self.s); // remove trailing newline
                    self.start_of_line = false;
                    return Some(Item::Separator);
                }

                // ```{language}\n{code}```
                if let Some(item) = self.code_block() {
                    return Some(item);
                }
            }

            // `code`
            if let Some(item) = self.inline_code() {
                return Some(item);
            }

            if let Some(rest) = self.s.strip_prefix("**") {
                self.s = rest;
                self.start_of_line = false;
                self.style.strong = !self.style.strong;
                continue;
            }
            if let Some(rest) = self.s.strip_prefix("~~") {
                self.s = rest;
                self.start_of_line = false;
                self.style.strikethrough = !self.style.strikethrough;
                continue;
            }
            if let Some(rest) = self.s.strip_prefix('*') {
                self.s = rest;
                self.start_of_line = false;
                self.style.italics = !self.style.italics;
                continue;
            }

            // `<url>` or `[link](url)`
            if let Some(item) = self.url() {
                return Some(item);
            }

            let specials = ['*', '`', '~', '_', '/', '$', '^', '\\', '<', '[', '\n'];
            let end = match self.s.find(specials) {
                Some(pos) => pos.max(1),
                None => self.s.len(),
            };

            let item = Item::Text(self.style, self.s[..end].to_owned());

            self.s = &self.s[end..];
            self.start_of_line = false;

            return Some(item);
        }
    }
}

#[test]
fn test_easy_mark_parser() {
    let items: Vec<_> = Parser::new("~strikethrough `code`~").collect();
    assert_eq!(
        items,
        vec![
            Item::Text(
                Style {
                    strikethrough: true,
                    ..Default::default()
                },
                "strikethrough ".to_owned()
            ),
            Item::Text(
                Style {
                    code: true,
                    strikethrough: true,
                    ..Default::default()
                },
                "code".to_owned()
            ),
        ]
    );
}
