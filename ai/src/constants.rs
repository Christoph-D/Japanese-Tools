use std::time::Duration;

pub const ENV_FILE_NAME: &str = ".env";
pub const CONFIG_FILE_NAME: &str = "config.toml";

// Hardcoded limit on line length for IRC in bytes.
pub const MAX_LINE_LENGTH: usize = 300;

// Default system prompt. See prompt::system_prompt() for how to set per-channel prompts.
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful AI in an IRC chatroom. You communicate with experienced software developers.
    Write in English unless the user asks for something else. Important: Limit your response to {MAX_LINE_LENGTH} characters.
    Write only a single line without markdown. Your answers are suitable for all age groups."#;

// The default system prompt if LANG is set to de_DE.UTF-8 or similar.
pub const DEFAULT_SYSTEM_PROMPT_DE: &str = r#"Du bist eine hilfreiche KI in einem IRC-Chatraum. Du redest mit erfahrenen Software-Entwicklern.
    Schreib auf Deutsch, außer wenn der User dich um etwas anderes bittet. Wichtig: Beschränk deine Antwort auf {MAX_LINE_LENGTH} Zeichen.
    Schreib nur eine einzige Zeile ohne Markdown. Deine Antworten sind für alle Altersstufen geeignet."#;

pub const DEFAULT_WEATHER_PROMPT: &str =
    "Describe the weather in your own words and comment on it.";
pub const DEFAULT_WEATHER_PROMPT_DE: &str =
    "Beschreib das Wetter in eigenen Worten und kommentiere es.";

// MEMORY_MAX_MESSAGES divided by half is the number of remembered user queries.
// Each invocation creates two messages, a user query and a response from the assistant.
pub const MEMORY_MAX_MESSAGES: usize = 20;
// Memories older than MEMORY_RETENTION will be dropped.
pub const MEMORY_RETENTION: time::Duration = time::Duration::minutes(10);
// User groups older than USER_GROUP_RETENTION will be dropped,
// isolating the contained users' memories from each other.
pub const USER_GROUP_RETENTION: time::Duration = time::Duration::hours(16);

pub const CLEAR_MEMORY_FLAG: &str = "clear_history";

// Message prefix to indicate to the user that memory was cleared.
pub const CLEAR_MEMORY_MESSAGE: &str = "📜🔥";

pub const TEMPERATURE_FLAG: &str = "temp";

pub const WEATHER_API_TIMEOUT: Duration = Duration::from_secs(3);

pub const MAX_TOKENS: u32 = 500;
