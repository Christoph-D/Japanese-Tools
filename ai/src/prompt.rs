use crate::{
    constants::{DEFAULT_SYSTEM_PROMPT, DEFAULT_SYSTEM_PROMPT_DE, MAX_LINE_LENGTH},
    memory::Memory,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const CHANNEL_PROMPTS_DIR: &str = "channel_prompts";

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    role: String,
    content: String,
}

fn default_prompt() -> &'static str {
    if std::env::var("LANG").unwrap_or_default().starts_with("de") {
        DEFAULT_SYSTEM_PROMPT_DE
    } else {
        DEFAULT_SYSTEM_PROMPT
    }
}

fn format_prompt(prompt: &str) -> String {
    prompt.replace("{MAX_LINE_LENGTH}", &MAX_LINE_LENGTH.to_string())
}

// Builds the system prompt from an optional per-channel prompt and the history of all users in the same union-set.
pub fn build_prompt(
    query: &str,
    sender: &str,
    receiver: &str,
    memory: &Memory,
    config_path: &Path,
) -> Vec<Message> {
    let mut v = vec![Message {
        role: "system".to_string(),
        content: format_prompt(&per_channel_prompt(receiver, config_path)),
    }];

    let joined_users = memory.get_joined_users(sender);
    let mut all_messages = Vec::new();
    for user in joined_users {
        let user_history = memory.user_history(&user, receiver);
        all_messages.extend(user_history);
    }

    all_messages.sort_by_key(|(_, _, timestamp)| *timestamp);

    for (role, content, _) in all_messages {
        v.push(Message {
            role: role.to_string(),
            content,
        });
    }

    v.push(Message {
        role: "user".to_string(),
        content: query.to_string(),
    });
    v
}

fn load_prompt_file(config_path: &Path, receiver: &str) -> Option<String> {
    let prompt_path = config_path.join(CHANNEL_PROMPTS_DIR).join(receiver);
    std::fs::read_to_string(prompt_path).ok()
}

// Loads a per-channel system prompt if one exists.
// If receiver matches a channel name, tries to load a prompt from {prompt_path}/<channel>.
fn per_channel_prompt(receiver: &str, config_path: &Path) -> String {
    // Only allow channel names starting with '#' and without '.' or '/'
    if !receiver.starts_with('#') || receiver.contains('.') || receiver.contains('/') {
        return default_prompt().to_string();
    }
    load_prompt_file(config_path, receiver).unwrap_or_else(|| default_prompt().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Sender;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_per_channel_prompt_with_valid_channel() {
        let dir = tempdir().unwrap();
        let prompt_path = dir.path();
        let channel = "#test_channel";
        let prompt_content = "This is a test prompt.";

        fs::create_dir_all(prompt_path.join(CHANNEL_PROMPTS_DIR)).unwrap();
        let mut file =
            fs::File::create(prompt_path.join(CHANNEL_PROMPTS_DIR).join(channel)).unwrap();
        file.write_all(prompt_content.as_bytes()).unwrap();

        let result = per_channel_prompt(channel, prompt_path);
        assert_eq!(result, prompt_content);
    }

    #[test]
    fn test_per_channel_prompt_with_invalid_channel() {
        let dir = tempdir().unwrap();
        let result = per_channel_prompt("not_a_channel", dir.path());
        assert_eq!(result, default_prompt());
    }
    #[test]
    fn test_build_prompt() {
        let p = format_prompt("Limit your response to {MAX_LINE_LENGTH} characters.");
        assert_eq!(
            p,
            format!("Limit your response to {} characters.", MAX_LINE_LENGTH)
        );
    }

    #[test]
    fn test_build_prompt_with_memory() {
        let dir = tempdir().unwrap();
        let config_path = dir.path();
        let query = "Test query";
        let sender = "user1";
        let receiver = "#test_channel";

        let mut memory = Memory::new_from_path(config_path).unwrap();
        memory.add_to_history(sender, Sender::User, receiver, "Hello!");
        memory.add_to_history(sender, Sender::Assistant, receiver, "Hi there!");

        let result = build_prompt(query, sender, receiver, &memory, config_path);

        assert_eq!(result.len(), 4);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, format_prompt(&default_prompt()));
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Hello!");
        assert_eq!(result[2].role, "assistant");
        assert_eq!(result[2].content, "Hi there!");
        assert_eq!(result[3].role, "user");
        assert_eq!(result[3].content, query);
    }

    #[test]
    fn test_build_prompt_with_custom_prompt() {
        let dir = tempdir().unwrap();
        let config_path = dir.path();
        let channel = "#test_channel";
        let prompt_content = "This is a test prompt.";
        let query = "Test query";
        let memory = Memory::new_from_path(&config_path).unwrap();

        fs::create_dir_all(config_path.join(CHANNEL_PROMPTS_DIR)).unwrap();
        let mut file =
            fs::File::create(config_path.join(CHANNEL_PROMPTS_DIR).join(channel)).unwrap();
        file.write_all(prompt_content.as_bytes()).unwrap();

        let result = build_prompt(query, "user1", channel, &memory, config_path);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, prompt_content);
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, query);
    }

    #[test]
    fn test_build_prompt_with_joined_history() {
        let dir = tempdir().unwrap();
        let config_path = dir.path();
        let query = "Test query";
        let receiver = "#test_channel";

        let mut memory = Memory::new_from_path(config_path).unwrap();

        memory.add_to_history("user1", Sender::User, receiver, "Hello from user1!");
        memory.add_to_history("user1", Sender::Assistant, receiver, "Hi user1!");

        memory.add_to_history("user2", Sender::User, receiver, "Hello from user2!");
        memory.add_to_history("user2", Sender::Assistant, receiver, "Hi user2!");

        memory.add_to_history("user3", Sender::User, receiver, "Hello from user3!");
        memory.add_to_history("user3", Sender::Assistant, receiver, "Hi user3!");

        memory.join_users("user1", "user2");

        let result = build_prompt(query, "user1", receiver, &memory, config_path);

        assert_eq!(result.len(), 6);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Hello from user1!");
        assert_eq!(result[2].role, "assistant");
        assert_eq!(result[2].content, "Hi user1!");
        assert_eq!(result[3].role, "user");
        assert_eq!(result[3].content, "Hello from user2!");
        assert_eq!(result[4].role, "assistant");
        assert_eq!(result[4].content, "Hi user2!");
        assert_eq!(result[5].role, "user");
        assert_eq!(result[5].content, query);
    }
}
