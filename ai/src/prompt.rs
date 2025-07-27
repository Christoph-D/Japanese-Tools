use crate::{
    constants::{DEFAULT_SYSTEM_PROMPT, DEFAULT_SYSTEM_PROMPT_DE, MAX_LINE_LENGTH},
    memory::Memory,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

fn load_prompt_file(path: &Path, receiver: &str) -> Option<String> {
    let prompt_path = path.join("channel_prompts").join(receiver);
    std::fs::read_to_string(prompt_path).ok()
}

// Builds the system prompt from an optional per-channel prompt and the user's history.
pub fn build_prompt(query: &str, sender: &str, receiver: &str, memory: &Memory) -> Vec<Message> {
    let mut v = vec![Message {
        role: "system".to_string(),
        content: format_prompt(&per_channel_prompt(receiver)),
    }];
    memory
        .user_history(sender, receiver)
        .into_iter()
        .map(|(role, content)| Message {
            role: role.to_string(),
            content,
        })
        .for_each(|message| v.push(message));
    v.push(Message {
        role: "user".to_string(),
        content: query.to_string(),
    });
    v
}

// Loads a per-channel system prompt if one exists.
// If receiver matches a channel name, tries to load a prompt from channel_prompts/<channel>.
fn per_channel_prompt(receiver: &str) -> String {
    // Only allow channel names starting with '#' and without '.' or '/'
    if !receiver.starts_with('#') || receiver.contains('.') || receiver.contains('/') {
        return default_prompt().to_string();
    }
    let paths = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf())),
        std::env::current_dir().ok(),
    ];
    for path in paths.iter().flatten() {
        if let Some(prompt) = load_prompt_file(path, receiver) {
            return prompt;
        }
    }
    default_prompt().to_string()
}
