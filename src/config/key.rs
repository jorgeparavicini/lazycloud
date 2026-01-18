use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Key {
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn with_ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub fn with_shift(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        // For character keys, compare case-insensitively when shift is involved
        match (self.code, event.code) {
            (KeyCode::Char(a), KeyCode::Char(b)) => {
                // Check if the characters match (considering case)
                let chars_match = a == b
                    || (a.is_ascii_alphabetic()
                        && b.is_ascii_alphabetic()
                        && a.to_ascii_lowercase() == b.to_ascii_lowercase());

                // Handle shift modifier for uppercase characters
                let expected_mods = if a.is_ascii_uppercase() {
                    self.modifiers | KeyModifiers::SHIFT
                } else {
                    self.modifiers
                };

                let actual_mods = if b.is_ascii_uppercase() {
                    event.modifiers | KeyModifiers::SHIFT
                } else {
                    event.modifiers
                };

                chars_match && (expected_mods & !KeyModifiers::SHIFT) == (actual_mods & !KeyModifiers::SHIFT)
            }
            _ => self.code == event.code && self.modifiers == event.modifiers,
        }
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("ctrl".to_string());
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("alt".to_string());
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("shift".to_string());
        }

        let key_str = match self.code {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Insert => "Insert".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::F(n) => format!("F{}", n),
            _ => "?".to_string(),
        };

        parts.push(key_str);
        parts.join("+")
    }
}

impl FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('+').collect();

        let mut modifiers = KeyModifiers::NONE;
        let mut key_part = s;

        if parts.len() > 1 {
            for part in &parts[..parts.len() - 1] {
                match part.to_lowercase().as_str() {
                    "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                    "alt" => modifiers |= KeyModifiers::ALT,
                    "shift" => modifiers |= KeyModifiers::SHIFT,
                    _ => return Err(format!("Unknown modifier: {}", part)),
                }
            }
            key_part = parts[parts.len() - 1];
        }

        let code = match key_part.to_lowercase().as_str() {
            "enter" | "return" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "tab" => KeyCode::Tab,
            "backspace" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "space" => KeyCode::Char(' '),
            s if s.starts_with('f') && s.len() > 1 => {
                let num: u8 = s[1..]
                    .parse()
                    .map_err(|_| format!("Invalid function key: {}", key_part))?;
                KeyCode::F(num)
            }
            s if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                // Preserve case from original input for single chars
                let original_char = key_part.chars().next().unwrap();
                KeyCode::Char(original_char)
            }
            _ => return Err(format!("Unknown key: {}", key_part)),
        };

        Ok(Key { code, modifiers })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.display())
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Key::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyBinding {
    Single(Key),
    Multiple(Vec<Key>),
}

impl KeyBinding {
    pub fn single(key: Key) -> Self {
        KeyBinding::Single(key)
    }

    pub fn multiple(keys: Vec<Key>) -> Self {
        KeyBinding::Multiple(keys)
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        match self {
            KeyBinding::Single(key) => key.matches(event),
            KeyBinding::Multiple(keys) => keys.iter().any(|k| k.matches(event)),
        }
    }

    pub fn display(&self) -> String {
        match self {
            KeyBinding::Single(key) => key.display(),
            KeyBinding::Multiple(keys) => {
                keys.iter()
                    .map(|k| k.display())
                    .collect::<Vec<_>>()
                    .join("/")
            }
        }
    }

    pub fn first_key(&self) -> &Key {
        match self {
            KeyBinding::Single(key) => key,
            KeyBinding::Multiple(keys) => keys.first().expect("Multiple must have at least one key"),
        }
    }
}

impl Default for KeyBinding {
    fn default() -> Self {
        KeyBinding::Single(Key::new(KeyCode::Null))
    }
}

impl From<Key> for KeyBinding {
    fn from(key: Key) -> Self {
        KeyBinding::Single(key)
    }
}

impl From<Vec<Key>> for KeyBinding {
    fn from(keys: Vec<Key>) -> Self {
        KeyBinding::Multiple(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_parsing() {
        assert_eq!(Key::from_str("q").unwrap(), Key::new(KeyCode::Char('q')));
        assert_eq!(Key::from_str("Enter").unwrap(), Key::new(KeyCode::Enter));
        assert_eq!(Key::from_str("Esc").unwrap(), Key::new(KeyCode::Esc));
        assert_eq!(
            Key::from_str("ctrl+c").unwrap(),
            Key::with_ctrl(KeyCode::Char('c'))
        );
        assert_eq!(Key::from_str("F1").unwrap(), Key::new(KeyCode::F(1)));
    }

    #[test]
    fn test_key_display() {
        assert_eq!(Key::new(KeyCode::Char('q')).display(), "q");
        assert_eq!(Key::new(KeyCode::Enter).display(), "Enter");
        assert_eq!(Key::with_ctrl(KeyCode::Char('c')).display(), "ctrl+c");
    }

    #[test]
    fn test_key_matches() {
        let key = Key::new(KeyCode::Char('q'));
        let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(key.matches(&event));
    }

    #[test]
    fn test_uppercase_key() {
        let key = Key::new(KeyCode::Char('G'));
        let event = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert!(key.matches(&event));
    }
}
