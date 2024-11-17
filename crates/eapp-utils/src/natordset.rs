use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct NatOrdSet(pub Vec<String>);

impl NatOrdSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn push(&mut self, value: String) {
        self.0.push(value);
    }

    pub fn search(&self, value: &str) -> Result<usize, usize> {
        self.0
            .binary_search_by(|item| natord::compare(item.as_str(), value))
    }

    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.0.iter()
    }

    pub fn sort(&mut self) {
        self.0.sort_by(|a, b| natord::compare(a, b));
    }
}
