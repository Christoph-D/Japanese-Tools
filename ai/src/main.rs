mod constants;
mod gettext;
mod memory;
mod model;
mod prompt;
mod unicodebytelimit;
mod weather;

use crate::constants::{
    CLEAR_MEMORY_FLAG, CLEAR_MEMORY_MESSAGE, CONFIG_FILE_NAME, ENV_FILE_NAME, MAX_LINE_LENGTH,
    TEMPERATURE_FLAG,
};
use crate::memory::{Memory, Sender};
use crate::model::{Config, Model, ModelList};
use crate::prompt::{Message, build_prompt};
use crate::unicodebytelimit::UnicodeByteLimit;

use formatx::formatx;
use gettextrs::{TextDomain, gettext, ngettext};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Flag {
    name: String,
    requires_value: bool,
}

impl Flag {
    fn new(name: String, requires_value: bool) -> Self {
        Flag {
            name,
            requires_value,
        }
    }
}

fn call_api(
    model: &Model,
    prompt: &Vec<Message>,
    temperature: &Option<f64>,
) -> Result<String, String> {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(formatget!("HTTP client error: {}", e)),
    };

    #[derive(Debug, serde::Serialize)]
    struct Payload<'a> {
        model: &'a str,
        messages: &'a Vec<Message>,
        max_tokens: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        temperature: &'a Option<f64>,
    }

    let payload = serde_json::json!(Payload {
        model: &model.id,
        messages: prompt,
        max_tokens: 300,
        temperature,
    });

    let response = client
        .post(model.endpoint.clone())
        .header("Authorization", format!("Bearer {}", model.api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send();

    let mut resp = match response {
        Ok(r) => r,
        Err(e) => return Err(formatget!("API error: {}", e)),
    };

    let mut body = String::new();
    if let Err(e) = resp.read_to_string(&mut body) {
        return Err(formatget!("Failed to read response: {}", e));
    }

    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(j) => j,
        Err(e) => return Err(formatget!("Invalid response: {}", e)),
    };

    let content = json["choices"]
        .get(0)
        .and_then(|c| c["message"]["content"].as_str())
        .map(|s| s.to_string());

    match content {
        Some(text) => Ok(text),
        None => Err(formatget!("Invalid response: {}", body)),
    }
}

fn sanitize_output(s: &str, api_key: &Option<&str>) -> String {
    // An HTTPS request error might expose the API key by accident, so we redact it to be safe.
    // This is irrelevant for successful requests because the LLM doesn't know the API key.
    let redacted = match api_key {
        Some(k) => s.replace(k, "[REDACTED]"),
        None => s.to_string(),
    };
    let s_no_newlines: String = redacted.chars().filter(|&c| c != '\n').collect();
    let truncated = s_no_newlines.unicode_byte_limit(MAX_LINE_LENGTH);
    if truncated != s_no_newlines {
        format!("{}...", truncated)
    } else {
        truncated.to_string()
    }
}

fn usage(models: &ModelList) -> String {
    formatget!(
        "Usage: !ai [{}] [-{}|-c] [-{}=1.0|-t=1.0] <query>.  Models: {}.  Default: {}",
        models
            .list_model_flags_without_default()
            .into_iter()
            .map(|f| format!("-{}", f))
            .collect::<Vec<_>>()
            .join("|"),
        CLEAR_MEMORY_FLAG,
        TEMPERATURE_FLAG,
        models.list_model_flags_human_readable().join(" "),
        models.default_model_name()
    )
}

fn locate_config_path() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let config_file = current_dir.join(CONFIG_FILE_NAME);
    if config_file.exists() && config_file.is_file() {
        return Some(current_dir);
    }
    None
}

#[derive(Debug)]
struct EnvVars {
    vars: HashMap<String, String>,
}

impl EnvVars {
    fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let env_file_path = path.join(ENV_FILE_NAME);
        let mut vars = HashMap::new();

        for item in dotenvy::from_path_iter(&env_file_path)? {
            let (key, value) = item?;
            vars.insert(key, value);
        }

        Ok(EnvVars { vars })
    }

    fn get(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }
}

fn textdomain_dir() -> Option<String> {
    // start in the executable directory, walk up to find the "gettext" directory
    let mut dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    loop {
        let gettext_dir = dir.join("gettext");
        if gettext_dir.is_dir() {
            return Some(gettext_dir.to_string_lossy().into_owned());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

// Extracts known flags from the query. Returns the flags and the remaining query.
// Example:
// Input: [Flag{name:"foo", requires_value:false}, Flag{name:"t", requires_value:true}],
//        "-foo -t=1.3   rest -of    the   query"
// Result: ["foo", "t=1.3"], "rest -of    the   query"
fn extract_flags(known_flags: &[Flag], query: &str) -> Result<(Vec<String>, String), String> {
    // Extract all -flags from query from the beginning until query no longer starts with - or we hit the end of string. Use string splitting or something, don't iterate over individual characters.
    // Collect all extracted flags which are not in known_flags. If non-empty, return all of them in the error.
    // Otherwise, return the extracted flags and the remaining query.
    let mut flags = Vec::new();
    let mut unknown_flags = Vec::new();
    let mut rest = query.trim_start();
    while let Some(stripped) = rest.strip_prefix('-') {
        let (flag_with_value, remaining) = stripped.split_once(' ').unwrap_or((stripped, ""));
        rest = remaining.trim_start();
        let flag_name = flag_with_value.split('=').next().unwrap_or(flag_with_value);

        if let Some(known_flag) = known_flags.iter().find(|f| f.name == flag_name) {
            if known_flag.requires_value && !flag_with_value.contains('=') {
                return Err(formatget!(
                    "Flag -{} requires a value (e.g., -{}=1.0)",
                    flag_name,
                    flag_name
                ));
            }
            flags.push(flag_with_value.to_string());
        } else {
            unknown_flags.push(flag_with_value.to_string());
        }
    }
    if !unknown_flags.is_empty() {
        let unknown_flags_str = unknown_flags.join(", ");
        let unknown_flags_str = if unknown_flags_str.len() > 60 {
            unknown_flags_str[..60].to_string() + "..."
        } else {
            unknown_flags_str
        };
        return Err(formatx!(
            ngettext(
                "Unknown flag: {}",
                "Unknown flags: {}",
                unknown_flags.len() as u32
            ),
            &unknown_flags_str
        )
        .unwrap_or_else(|_| format!("Unknown flag(s): {}", &unknown_flags_str)));
    }
    Ok((flags, rest.to_string()))
}

fn parse_command_line(query: &str, models: &ModelList) -> Result<(Vec<String>, String), String> {
    let known_flags = {
        let mut known_flags = Vec::new();
        for model_flag in models.list_model_flags() {
            known_flags.push(Flag::new(model_flag, false));
        }
        known_flags.push(Flag::new(CLEAR_MEMORY_FLAG.to_string(), false));
        known_flags.push(Flag::new("c".to_string(), false)); // Short for CLEAR_MEMORY_FLAG
        known_flags.push(Flag::new(TEMPERATURE_FLAG.to_string(), true));
        known_flags.push(Flag::new("t".to_string(), true)); // Short for TEMPERATURE_FLAG
        known_flags
    };
    let flag_names: Vec<&String> = known_flags.iter().map(|f| &f.name).collect();
    if flag_names.len()
        != flag_names
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
    {
        return Err(gettext(
            "Internal error: duplicate configured flags detected, check your model config",
        ));
    }
    extract_flags(&known_flags, query).map_err(|err| format!("{}.  {}", err, usage(models)))
}

struct Input {
    config_path: PathBuf,
    config: Config,
    flags: Vec<String>,
    models: ModelList,
    model: Model,
    sender: String,
    receiver: String,
    query: String,
    irc_plugin: bool,
}

fn main() {
    let input = setup().unwrap_or_else(|err| {
        println!("{}", sanitize_output(&err.to_string(), &None));
        std::process::exit(1);
    });
    match run(&input) {
        Ok(msg) => println!("{}", sanitize_output(&msg, &Some(&input.model.api_key))),
        Err(err) => {
            println!(
                "{}",
                sanitize_output(&err.to_string(), &Some(&input.model.api_key))
            );
            std::process::exit(1);
        }
    }
}

fn setup() -> Result<Input, String> {
    if let Some(dir) = textdomain_dir() {
        // Ignore errors and use untranslated strings if it fails.
        let _ = TextDomain::new("japanese_tools")
            .skip_system_data_paths()
            .push(&dir)
            .init();
    }
    let config_path = match locate_config_path() {
        Some(path) => path,
        None => return Err(gettext("Config file not found.")),
    };
    let env_vars = EnvVars::from_file(&config_path).unwrap_or_else(|_| EnvVars {
        vars: HashMap::new(),
    });

    let sender = std::env::var("DMB_SENDER").unwrap_or_default();
    let receiver = std::env::var("DMB_RECEIVER").unwrap_or_default();

    let config = Config::new(&config_path, &env_vars)?;
    let models = ModelList::new(&config)?;

    let (flags, query) = parse_command_line(
        &std::env::args().skip(1).collect::<Vec<_>>().join(" "),
        &models,
    )?;

    let channel_default_model = config.get_channel_default_model(&receiver);
    let model = models
        .select_model_for_channel(&flags, channel_default_model)?
        .clone();

    Ok(Input {
        query,
        sender,
        receiver,
        flags,
        models,
        model,
        config,
        config_path,
        irc_plugin: std::env::var("IRC_PLUGIN").as_deref() == Ok("1"),
    })
}

#[derive(Debug, PartialEq)]
enum CommandResult {
    NotACommand,
    Message(String),
    AskAgent {
        extra_history: String,
        query: String,
    },
}

fn process_command(
    command: &str,
    args: &str,
    sender: &str,
    receiver: &str,
    memory: &mut Memory,
) -> Result<CommandResult, String> {
    let command = command.to_lowercase();
    match command.as_ref() {
        "join" => {
            let target_user = args.trim();
            if target_user.is_empty() || target_user == sender {
                return Ok(CommandResult::Message(
                    gettext("Usage: join <username>").to_string(),
                ));
            }

            memory
                .join_users(sender, target_user, receiver)
                .map_err(|e| format!("Failed to join users: {}", e))?;

            Ok(CommandResult::Message(formatget!(
                "{} joined memory with the group: {}",
                sender,
                memory
                    .get_joined_users_excluding_self(sender, receiver)
                    .join(", ")
            )))
        }
        "solo" => {
            let arg = args.trim();
            let target = if arg.is_empty() { sender } else { arg };

            memory
                .make_user_solo(target, receiver)
                .map_err(|e| format!("Failed to make user solo: {}", e))?;

            Ok(CommandResult::Message(formatget!(
                "{} is now solo.",
                target
            )))
        }
        "joined" => {
            let other_users = memory.get_joined_users_excluding_self(sender, receiver);
            if other_users.is_empty() {
                return Ok(CommandResult::Message(formatget!(
                    "{} is not sharing memory with anyone.",
                    sender
                )));
            }
            Ok(CommandResult::Message(formatget!(
                "{} is sharing memory with: {}",
                sender,
                other_users.join(", ")
            )))
        }
        _ => {
            if command == gettext("weather").to_lowercase() {
                let city = args.trim();
                if city.is_empty() {
                    return Ok(CommandResult::Message(format!(
                        "{}  ({})",
                        gettext("Usage: weather <city>"),
                        gettext("Weather data by https://open-meteo.com")
                    )));
                }
                match weather::get_weather(city) {
                    Ok(w) => Ok(CommandResult::AskAgent {
                        query: weather::weather_prompt().to_string(),
                        extra_history: formatget!("The weather in {} is: {}.", w.city, w.weather),
                    }),
                    Err(err) => Ok(CommandResult::Message(err)),
                }
            } else {
                Ok(CommandResult::NotACommand)
            }
        }
    }
}

fn run(input: &Input) -> Result<String, String> {
    // Prevent usage in private messages
    if input.irc_plugin && !input.receiver.starts_with('#') {
        return Ok(gettext("!ai is only available in channels."));
    }

    let mut memory = Memory::new_from_path(&input.config_path)?;

    let history_cleared = input.flags.contains(&CLEAR_MEMORY_FLAG.to_string())
        || input.flags.contains(&"c".to_string());
    if history_cleared {
        memory
            .clear_history(&input.sender, &input.receiver)
            .map_err(|e| format!("Failed to clear history: {}", e))?;
    }

    let query = input.query.trim();
    if query.is_empty() {
        if history_cleared {
            return Ok(format!("[{}]", CLEAR_MEMORY_MESSAGE));
        } else {
            return Ok(usage(&input.models));
        }
    }

    let (command, args) = query.split_once(' ').unwrap_or((query, ""));
    let query = match process_command(command, args, &input.sender, &input.receiver, &mut memory)? {
        CommandResult::Message(result) => return Ok(result),
        CommandResult::NotACommand => query.to_string(),
        CommandResult::AskAgent {
            query,
            extra_history,
        } => {
            memory
                .add_to_history(&input.sender, Sender::User, &input.receiver, &extra_history)
                .map_err(|e| format!("Failed to add extra history: {}", e))?;
            query
        }
    };

    let prompt = build_prompt(
        &query,
        &input.sender,
        &input.receiver,
        &memory,
        &input.config,
    );
    memory
        .add_to_history(&input.sender, Sender::User, &input.receiver, &query)
        .map_err(|e| format!("Failed to add user query to history: {}", e))?;

    let temperature = input
        .flags
        .iter()
        .find(|f| f.starts_with(&(TEMPERATURE_FLAG.to_string() + "=")) || f.starts_with("t="))
        .and_then(|f| f.split('=').nth(1))
        .and_then(|s| s.parse::<f64>().ok().map(|t| t.clamp(0.0, 2.0)))
        .or_else(|| input.config.get_channel_temperature(&input.receiver));

    let result = &call_api(&input.model, &prompt, &temperature)?;

    memory
        .add_to_history(&input.sender, Sender::Assistant, &input.receiver, result)
        .map_err(|e| format!("Failed to add assistant response to history: {}", e))?;

    let flag_state = {
        let mut flag_state: Vec<String> = Vec::new();

        // Only show model prefix if user explicitly used a model flag
        let user_model_flag = input
            .flags
            .iter()
            .any(|flag| input.models.list_model_flags().contains(&flag.to_string()));
        if user_model_flag && input.model.name != input.models.default_model_name() {
            flag_state.push(input.model.short_name.clone());
        }

        // Only show temperature prefix if user explicitly used -t or -temperature flag
        let user_temperature_flag = input.flags.iter().any(|flag| {
            flag.starts_with(&(TEMPERATURE_FLAG.to_string() + "=")) || flag.starts_with("t=")
        });
        if user_temperature_flag {
            if let Some(t) = temperature {
                flag_state.push(format!("t={}", t));
            }
        }

        flag_state.join(" ")
    };
    let result = if flag_state.is_empty() {
        result.to_string()
    } else {
        format!("[{}] {}", flag_state, result)
    };

    let result = if history_cleared {
        format!("[{}] {}", CLEAR_MEMORY_MESSAGE, &result)
    } else {
        result
    };

    // Prevent triggering other bots that might be present in the same channel.
    let result = match result.chars().next() {
        Some('!') => " ".to_string() + &result,
        _ => result,
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_flags() {
        let known_flags = vec![
            Flag::new("foo".to_string(), false),
            Flag::new("bar".to_string(), false),
            Flag::new("t".to_string(), true),
        ];
        let (flags, query) =
            extract_flags(&known_flags, "-foo -t=1.3 -bar   rest -of    the   query").unwrap();
        assert_eq!(flags, vec!["foo", "t=1.3", "bar"]);
        assert_eq!(query, "rest -of    the   query");
    }

    #[test]
    fn test_extract_flags_temperature_requires_value() {
        let known_flags = vec![
            Flag::new("foo".to_string(), false),
            Flag::new("t".to_string(), true),
        ];
        let result = extract_flags(&known_flags, "-t foo");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Flag -t requires a value"));
        assert!(error.contains("-t=1.0"));
    }

    #[test]
    fn test_process_command_join() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("join", "alice", "bob", "receiver1", &mut memory).unwrap();
        assert_eq!(
            result,
            CommandResult::Message("bob joined memory with the group: alice".to_string())
        );

        // Verify the users are actually joined
        let joined_users = memory.get_joined_users("bob", "receiver1");
        assert!(joined_users.contains(&"alice".to_string()));
        assert!(joined_users.contains(&"bob".to_string()));
        assert_eq!(joined_users.len(), 2);
    }

    #[test]
    fn test_process_command_join_empty_user() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("join", "", "bob", "receiver1", &mut memory);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            CommandResult::Message("Usage: join <username>".to_string())
        );
    }

    #[test]
    fn test_process_command_join_self() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("join", "bob", "bob", "receiver1", &mut memory);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            CommandResult::Message("Usage: join <username>".to_string())
        );
    }

    #[test]
    fn test_process_command_unknown() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("unknown", "args", "bob", "receiver1", &mut memory).unwrap();
        assert_eq!(result, CommandResult::NotACommand);
    }

    #[test]
    fn test_channel_configuration_integration() {
        use crate::constants::CONFIG_FILE_NAME;
        use crate::model::{Config, ModelList};

        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "LITELLM_API_KEY=test-key\n").unwrap();
        let env_vars = EnvVars::from_file(&config_dir).unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(&config_path, r##"
[general]
default_model = "default-model"

[providers.litellm]
endpoint = "http://test.example.com"
models = [
  { id = "test-model", short_name = "t", name = "Test Model" },
  { id = "default-model", short_name = "d", name = "Default Model" }
]

[channels]
"#test-channel" = { default_model = "test-model", temperature = 0.8, system_prompt = "Test channel prompt" }
"##).unwrap();

        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");
        let models = ModelList::new(&config).unwrap();

        // Test channel-specific model selection
        let channel_default = config.get_channel_default_model("#test-channel");
        assert_eq!(channel_default, "test-model");

        let selected_model = models
            .select_model_for_channel(&vec![], channel_default)
            .unwrap();
        assert_eq!(selected_model.id, "test-model");

        // Test channel-specific temperature
        let channel_temp = config.get_channel_temperature("#test-channel");
        assert_eq!(channel_temp, Some(0.8));

        // Test channel-specific system prompt
        let channel_prompt = config.get_channel_system_prompt("#test-channel");
        assert_eq!(channel_prompt, Some("Test channel prompt"));

        // Test fallback for unconfigured channel
        let unconfigured_default = config.get_channel_default_model("#unknown");
        assert_eq!(unconfigured_default, "default-model");

        let unconfigured_temp = config.get_channel_temperature("#unknown");
        assert_eq!(unconfigured_temp, None);

        let unconfigured_prompt = config.get_channel_system_prompt("#unknown");
        assert_eq!(unconfigured_prompt, None);
    }

    #[test]
    fn test_temperature_extraction_with_channel_config() {
        use crate::constants::CONFIG_FILE_NAME;
        use crate::model::Config;

        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "LITELLM_API_KEY=test-key\n").unwrap();
        let env_vars = EnvVars::from_file(&config_dir).unwrap();

        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r##"
[general]
default_model = "default-model"

[providers.deepseek]
models = [
  { id = "default-model", short_name = "d", name = "Default Model" }
]

[channels]
"#test" = { default_model = "test-model", temperature = 0.5, system_prompt = "Test prompt" }
"##,
        )
        .unwrap();

        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");

        // Test channel-specific temperature fallback (simulating the logic from run())
        let flags: Vec<String> = vec![];
        let temperature = flags
            .iter()
            .find(|f| f.starts_with("temperature=") || f.starts_with("t="))
            .and_then(|f| f.split('=').nth(1))
            .and_then(|s| s.parse::<f64>().ok().map(|t| t.clamp(0.0, 2.0)))
            .or_else(|| config.get_channel_temperature("#test"));

        assert_eq!(temperature, Some(0.5));

        // Test for unconfigured channel
        let temperature_unconfigured = flags
            .iter()
            .find(|f| f.starts_with("temperature=") || f.starts_with("t="))
            .and_then(|f| f.split('=').nth(1))
            .and_then(|s| s.parse::<f64>().ok().map(|t| t.clamp(0.0, 2.0)))
            .or_else(|| config.get_channel_temperature("#unknown"));

        assert_eq!(temperature_unconfigured, None);
    }

    #[test]
    fn test_temperature_flag_overrides_channel() {
        use crate::constants::CONFIG_FILE_NAME;
        use crate::model::Config;

        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "DEEPSEEK_API_KEY=test-key\n").unwrap();
        let env_vars = EnvVars::from_file(&config_dir).unwrap();

        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r##"
[general]
default_model = "default-model"

[providers.deepseek]
models = [
  { id = "default-model", short_name = "d", name = "Default Model" }
]

[channels]
"#test" = { temperature = 0.5 }
"##,
        )
        .unwrap();

        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");

        // Temperature flag should override channel temperature (simulating the logic from run())
        let flags = vec!["t=0.9".to_string()];
        let temperature = flags
            .iter()
            .find(|f| f.starts_with("temperature=") || f.starts_with("t="))
            .and_then(|f| f.split('=').nth(1))
            .and_then(|s| s.parse::<f64>().ok().map(|t| t.clamp(0.0, 2.0)))
            .or_else(|| config.get_channel_temperature("#test"));

        assert_eq!(temperature, Some(0.9));
    }
}
