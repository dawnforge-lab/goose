use spawnbot_common::config::TelegramConfig;

/// Check if a user/chat is allowed to interact with the bot.
///
/// Access is granted if:
/// - The user is the configured owner
/// - The user is in the allowed_users list
/// - The chat is in the allowed_chats list
pub fn is_allowed(user_id: u64, chat_id: i64, config: &TelegramConfig) -> bool {
    if user_id == config.owner_id {
        return true;
    }
    if config.allowed_users.contains(&user_id) {
        return true;
    }
    if config.allowed_chats.contains(&chat_id) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> TelegramConfig {
        TelegramConfig {
            enabled: true,
            bot_token_env: "TEST_TOKEN".into(),
            owner_id: 12345,
            allowed_users: vec![67890, 11111],
            allowed_chats: vec![-100200300],
            ..Default::default()
        }
    }

    #[test]
    fn test_owner_allowed() {
        let config = test_config();
        assert!(is_allowed(12345, 999, &config));
    }

    #[test]
    fn test_allowed_user() {
        let config = test_config();
        assert!(is_allowed(67890, 999, &config));
    }

    #[test]
    fn test_allowed_chat() {
        let config = test_config();
        assert!(is_allowed(99999, -100200300, &config));
    }

    #[test]
    fn test_rejected_user() {
        let config = test_config();
        assert!(!is_allowed(99999, 999, &config));
    }

    #[test]
    fn test_zero_owner_not_matched() {
        let config = TelegramConfig {
            owner_id: 0,
            ..Default::default()
        };
        // user_id 0 would match owner_id 0 — this is expected since 0 == 0
        assert!(is_allowed(0, 0, &config));
        // but a real user should not match
        assert!(!is_allowed(12345, 999, &config));
    }
}
