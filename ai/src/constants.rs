pub const CONFIG_FILE_NAME: &str = "api-keys";

// Hardcoded limit on line length for IRC
pub const MAX_LINE_LENGTH: usize = 300;

// Default system prompt. See prompt::system_prompt() for how to set per-channel prompts.
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful AI in an IRC chatroom. You communicate with experienced software developers.
    Write in English unless the user asks for something else. Keep your response under {MAX_LINE_LENGTH} characters.
    Write only a single line without markdown. Your answers are suitable for all age groups."#;

// The default system prompt if LANG is set to de_DE.UTF-8 or similar.
pub const DEFAULT_SYSTEM_PROMPT_DE: &str = r#"Du bist eine hilfreiche KI in einem IRC-Chatraum. Du redest mit erfahrenen Software-Entwicklern.
    Schreib auf Deutsch, außer wenn der User dich um etwas anderes bittet. Antworte mit maximal {MAX_LINE_LENGTH} Zeichen.
    Schreib nur eine einzige Zeile ohne Markdown. Deine Antworten sind für alle Altersstufen geeignet."#;

// MEMORY_MAX_MESSAGES divided by half is the number of remembered user queries.
// Each invocation creates two messages, a user query and a response from the assistant.
pub const MEMORY_MAX_MESSAGES: usize = 20;
// Memories older than MEMORY_RETENTION will be dropped.
pub const MEMORY_RETENTION: time::Duration = time::Duration::minutes(10);

pub const CLEAR_MEMORY_FLAG: &str = "clear_history";

pub const TEMPERATURE_FLAG: &str = "temp";
