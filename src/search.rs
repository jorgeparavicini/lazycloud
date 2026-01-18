//! Search functionality for filtering items.
//!
//! This module encapsulates the search/matching logic, allowing the underlying
//! implementation to be changed without affecting the rest of the codebase.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

/// A matcher for fuzzy searching text.
///
/// This wraps the underlying fuzzy matching implementation, providing a simple
/// interface that can be used throughout the application.
pub struct Matcher {
    inner: SkimMatcherV2,
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Matcher {
    /// Create a new matcher instance.
    pub fn new() -> Self {
        Self {
            inner: SkimMatcherV2::default(),
        }
    }

    /// Check if the text matches the pattern using fuzzy matching.
    ///
    /// Returns `true` if the pattern fuzzy-matches the text.
    /// The matching is case-insensitive and allows non-consecutive characters.
    ///
    /// # Examples
    ///
    /// ```
    /// let matcher = Matcher::new();
    /// assert!(matcher.matches("api-key", "apk"));
    /// assert!(matcher.matches("database-password", "dbpw"));
    /// assert!(!matcher.matches("hello", "xyz"));
    /// ```
    pub fn matches(&self, text: &str, pattern: &str) -> bool {
        // Convert pattern to lowercase for case-insensitive matching
        let pattern_lower = pattern.to_lowercase();
        self.inner.fuzzy_match(text, &pattern_lower).is_some()
    }

    /// Get the match score for ranking results.
    ///
    /// Returns `Some(score)` if the pattern matches, where higher scores
    /// indicate better matches. Returns `None` if there's no match.
    pub fn score(&self, text: &str, pattern: &str) -> Option<i64> {
        // Convert pattern to lowercase for case-insensitive matching
        let pattern_lower = pattern.to_lowercase();
        self.inner.fuzzy_match(text, &pattern_lower)
    }

    /// Check if any of the provided texts match the pattern.
    pub fn matches_any<'a>(&self, texts: impl IntoIterator<Item = &'a str>, pattern: &str) -> bool {
        texts.into_iter().any(|text| self.matches(text, pattern))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        let matcher = Matcher::new();

        // Basic fuzzy matching
        assert!(matcher.matches("api-key", "apk"));
        assert!(matcher.matches("database-password", "dbpw"));
        assert!(matcher.matches("production", "prd"));

        // Exact match
        assert!(matcher.matches("hello", "hello"));

        // Case-insensitive
        assert!(matcher.matches("API-KEY", "apk"));
        assert!(matcher.matches("api-key", "APK"));

        // No match
        assert!(!matcher.matches("hello", "xyz"));
    }

    #[test]
    fn test_matches_any() {
        let matcher = Matcher::new();

        let texts = vec!["apple", "banana", "cherry"];
        assert!(matcher.matches_any(texts.iter().map(|s| *s), "ban"));
        assert!(matcher.matches_any(texts.iter().map(|s| *s), "cher"));
        assert!(!matcher.matches_any(texts.iter().map(|s| *s), "xyz"));
    }

    #[test]
    fn test_score() {
        let matcher = Matcher::new();

        // Exact match should score higher than fuzzy
        let exact_score = matcher.score("api", "api").unwrap();
        let fuzzy_score = matcher.score("api-key", "api").unwrap();
        assert!(exact_score >= fuzzy_score);

        // No match returns None
        assert!(matcher.score("hello", "xyz").is_none());
    }
}
