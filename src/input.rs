//! Checks on the captured text before any request starts.

#![forbid(unsafe_code)]

/// The preview is the safety gate before Replace; past about 2,000 characters a rewrite no longer
/// fits a compact popup and tends to be accepted unread. Fixed in the MVP.
pub const INPUT_LIMIT: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    NoText,
    TooLong { chars: usize },
    SecureField,
}

impl Refusal {
    pub fn title(&self) -> &'static str {
        match self {
            Refusal::NoText => "No text selected",
            Refusal::TooLong { .. } => "Selection too long",
            Refusal::SecureField => "Password field",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Refusal::NoText => "Select some text, then press your shortcut again.".into(),
            Refusal::TooLong { chars } => format!(
                "{} characters. Retone rewrites up to {} at a time. Select a shorter part and try again.",
                group_thousands(*chars),
                group_thousands(INPUT_LIMIT)
            ),
            Refusal::SecureField => "Retone does not read or change password fields.".into(),
        }
    }
}

/// Trims the selection and applies the limit. Counts characters, not bytes, so accented and CJK
/// text gets the same limit as ASCII.
pub fn check(text: &str) -> Result<String, Refusal> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(Refusal::NoText);
    }
    let chars = trimmed.chars().count();
    if chars > INPUT_LIMIT {
        return Err(Refusal::TooLong { chars });
    }
    Ok(trimmed.to_string())
}

fn group_thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_counts_characters_after_trimming() {
        let at_limit = format!("  {}\n", "é".repeat(INPUT_LIMIT));
        assert_eq!(check(&at_limit).map(|t| t.chars().count()), Ok(INPUT_LIMIT));
        let over = "é".repeat(INPUT_LIMIT + 1);
        assert_eq!(check(&over), Err(Refusal::TooLong { chars: INPUT_LIMIT + 1 }));
    }

    #[test]
    fn white_space_only_is_no_text() {
        assert_eq!(check(" \n\t "), Err(Refusal::NoText));
    }

    #[test]
    fn the_too_long_message_matches_the_plan() {
        assert_eq!(
            Refusal::TooLong { chars: 2431 }.message(),
            "2,431 characters. Retone rewrites up to 2,000 at a time. Select a shorter part and try again."
        );
        assert_eq!(group_thousands(1_234_567), "1,234,567");
    }
}
