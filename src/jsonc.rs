//! Reading JSON with comments and trailing commas.

/// Parses JSONC text into a JSON value.
pub fn parse(text: &str) -> Result<serde_json::Value, String> {
    let mut text = text.strip_prefix('\u{feff}').unwrap_or(text).to_string();
    json_strip_comments::strip(&mut text).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_should_accept_comments_and_trailing_commas() {
        let value = parse("{\n // note\n \"a\": [1, 2,], /* x */\n}").unwrap();
        assert_eq!(value, serde_json::json!({ "a": [1, 2] }));
    }
}
