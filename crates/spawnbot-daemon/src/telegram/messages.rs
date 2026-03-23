/// Split a message into chunks that fit within the Telegram message size limit.
///
/// Tries to split at newlines first, then at spaces. Falls back to hard split
/// at max_len if no natural break point is found.
pub fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }
    let mut parts = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            parts.push(remaining.to_string());
            break;
        }
        // Try to split at newline
        let split_at = remaining[..max_len]
            .rfind('\n')
            .unwrap_or_else(|| remaining[..max_len].rfind(' ').unwrap_or(max_len));
        parts.push(remaining[..split_at].to_string());
        remaining = remaining[split_at..].trim_start();
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_message_no_split() {
        let parts = split_message("hello world", 100);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], "hello world");
    }

    #[test]
    fn test_split_at_newline() {
        let text = "line one\nline two\nline three";
        let parts = split_message(text, 15);
        assert_eq!(parts[0], "line one");
        assert_eq!(parts[1], "line two");
        assert_eq!(parts[2], "line three");
    }

    #[test]
    fn test_split_at_space() {
        let text = "word1 word2 word3 word4";
        let parts = split_message(text, 12);
        // "word1 word2 " is 12 chars, rfind(' ') at index 11
        assert_eq!(parts[0], "word1 word2");
        assert_eq!(parts[1], "word3 word4");
    }

    #[test]
    fn test_hard_split_no_break() {
        let text = "abcdefghijklmnop";
        let parts = split_message(text, 5);
        assert_eq!(parts[0], "abcde");
        assert_eq!(parts[1], "fghij");
        assert_eq!(parts[2], "klmno");
        assert_eq!(parts[3], "p");
    }

    #[test]
    fn test_empty_string() {
        let parts = split_message("", 100);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], "");
    }
}
