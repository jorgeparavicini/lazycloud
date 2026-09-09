use chrono::{DateTime, Utc};

#[must_use]
pub fn format_timestamp(seconds: i64) -> String {
    DateTime::<Utc>::from_timestamp(seconds, 0).map_or_else(
        || "Unknown".to_string(),
        |dt| dt.format("%Y-%m-%d %H:%M").to_string(),
    )
}

/// Drop the trailing newline an external editor appends when it writes a file.
///
/// Editors terminate the last line on save, so a payload opened and saved
/// unchanged comes back one byte longer and would be stored as a new secret
/// version ending in a stray `\n`. Content that already ended in a newline
/// keeps it.
#[must_use]
pub fn strip_editor_trailing_newline(original: &str, mut edited: String) -> String {
    if original.ends_with('\n') {
        return edited;
    }

    if edited.ends_with('\n') {
        edited.pop();
        if edited.ends_with('\r') {
            edited.pop();
        }
    }

    edited
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_newline_the_editor_added() {
        let stripped = strip_editor_trailing_newline("secret", "secret\n".to_string());
        assert_eq!(stripped, "secret");
    }

    #[test]
    fn strips_a_crlf_terminator() {
        let stripped = strip_editor_trailing_newline("secret", "secret\r\n".to_string());
        assert_eq!(stripped, "secret");
    }

    #[test]
    fn strips_only_the_final_newline() {
        let stripped = strip_editor_trailing_newline("a\nb", "a\nb\n\n".to_string());
        assert_eq!(stripped, "a\nb\n");
    }

    #[test]
    fn keeps_the_newline_when_the_original_had_one() {
        let stripped = strip_editor_trailing_newline("secret\n", "changed\n".to_string());
        assert_eq!(stripped, "changed\n");
    }

    #[test]
    fn leaves_content_without_a_terminator_alone() {
        let stripped = strip_editor_trailing_newline("secret", "changed".to_string());
        assert_eq!(stripped, "changed");
    }
}
