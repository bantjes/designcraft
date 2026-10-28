//! Community and project links. Marian DesignCraft has none: it never opens the internet.

use serde_json::{Value, json};

/// The app's slug (kept for the names of files and folders that use it).
pub const APP: &str = "designcraft";

/// All links as JSON (the `app.links` command): none.
pub fn all() -> Value {
    json!({})
}

/// A link by key: none.
pub fn get(_key: &str) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_no_links() {
        assert_eq!(all(), json!({}));
        for k in ["discord", "website", "appPage", "github", "issues"] {
            assert!(get(k).is_none(), "{k}");
        }
    }
}
