mod constants;
mod gettext;
mod memory;
mod model;
mod prompt;

use crate::constants::{CLEAR_MEMORY_FLAG, CONFIG_FILE_NAME, MAX_LINE_LENGTH, TEMPERATURE_FLAG};
use crate::memory::{Memory, Sender};
use crate::model::{Config, Model, ModelList};
use crate::prompt::{Message, build_prompt};

use formatx::formatx;
use gettextrs::{TextDomain, gettext, ngettext};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
    if s_no_newlines.len() > MAX_LINE_LENGTH {
        format!("{}...", &s_no_newlines[..MAX_LINE_LENGTH])
    } else {
        s_no_newlines
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

fn load_env(path: &Path) {
    let _ = dotenvy::from_path(path.join(CONFIG_FILE_NAME));
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
// Example: ["foo", "bar", "t"], "-foo -bar -t=foo   rest -of    the   query" -> ["foo", "bar", "t=foo"], "rest -of    the   query"
fn extract_flags(known_flags: &[String], query: &str) -> Result<(Vec<String>, String), String> {
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
        if known_flags.contains(&flag_name.to_string()) {
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
        let mut known_flags = models.list_model_flags();
        known_flags.push(CLEAR_MEMORY_FLAG.to_string());
        known_flags.push("c".to_string()); // Short for CLEAR_MEMORY_FLAG
        known_flags.push(TEMPERATURE_FLAG.to_string());
        known_flags.push("t".to_string()); // Short for TEMPERATURE_FLAG
        known_flags
    };
    if known_flags.len()
        != known_flags
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
    load_env(&config_path);

    let sender = std::env::var("DMB_SENDER").unwrap_or_default();
    let receiver = std::env::var("DMB_RECEIVER").unwrap_or_default();

    let models = ModelList::new(&Config::from_env())?;

    let (flags, query) = parse_command_line(
        &std::env::args().skip(1).collect::<Vec<_>>().join(" "),
        &models,
    )?;

    let model = models.select_model(&flags)?.clone();

    Ok(Input {
        query,
        sender,
        receiver,
        flags,
        models,
        model,
        config_path,
        irc_plugin: std::env::var("IRC_PLUGIN").ok().as_deref() == Some("1"),
    })
}

fn process_command(
    command: &str,
    args: &str,
    sender: &str,
    memory: &mut Memory,
) -> Result<Option<String>, String> {
    match command {
        "join" => {
            let target_user = args.trim();
            if target_user.is_empty() || target_user == sender {
                return Ok(Some(gettext("Usage: join <username>").to_string()));
            }

            memory.join_users(sender, target_user);
            memory
                .save()
                .map_err(|e| format!("Failed to save memory: {}", e))?;

            Ok(Some(formatget!(
                "{} joined memory with the group: {}",
                sender,
                memory.get_joined_users_excluding_self(sender).join(", ")
            )))
        }
        "solo" => {
            let arg = args.trim();
            let target = if arg.is_empty() {
                sender
            } else {
                arg
            };

            memory.make_user_solo(target);
            memory
                .save()
                .map_err(|e| format!("Failed to save memory: {}", e))?;

            Ok(Some(formatget!("{} is now solo.", target)))
        }
        "joined" => {
            let other_users = memory.get_joined_users_excluding_self(sender);
            if other_users.is_empty() {
                return Ok(Some(formatget!(
                    "{} is not sharing memory with anyone.",
                    sender
                )));
            }
            Ok(Some(formatget!(
                "{} is sharing memory with: {}",
                sender,
                other_users.join(", ")
            )))
        }
        _ => Ok(None), // Not a command
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
        memory.clear_history(&input.sender, &input.receiver);
    }

    let query = input.query.trim();
    if query.is_empty() {
        if history_cleared {
            return Ok("[📜→🔥]".to_string());
        } else {
            return Ok(usage(&input.models));
        }
    }

    let (command, args) = query.split_once(' ').unwrap_or((query, ""));
    if let Some(result) = process_command(command, args, &input.sender, &mut memory)? {
        return Ok(result);
    }

    let prompt = build_prompt(
        &input.query,
        &input.sender,
        &input.receiver,
        &memory,
        &input.config_path,
    );
    memory.add_to_history(&input.sender, Sender::User, &input.receiver, &input.query);

    let _ = memory.save();

    let temperature = input
        .flags
        .iter()
        .find(|f| f.starts_with(&(TEMPERATURE_FLAG.to_string() + "=")) || f.starts_with("t="))
        .and_then(|f| f.split('=').nth(1))
        .and_then(|s| s.parse::<f64>().ok().map(|t| t.clamp(0.0, 2.0)));

    let result = &call_api(&input.model, &prompt, &temperature)?;

    memory.add_to_history(&input.sender, Sender::Assistant, &input.receiver, result);
    let _ = memory.save();

    let flag_state = {
        let mut flag_state: Vec<String> = Vec::new();
        if input.model.name != input.models.default_model_name() {
            flag_state.push(input.model.short_name.clone());
        }
        if let Some(t) = temperature {
            flag_state.push(format!("t={}", t));
        }
        flag_state.join(" ")
    };
    let result = if flag_state.is_empty() {
        result.to_string()
    } else {
        format!("[{}] {}", flag_state, result)
    };

    let result = if history_cleared {
        "[📜→🔥] ".to_string() + &result
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
        let (flags, query) = extract_flags(
            &vec!["foo".to_string(), "bar".to_string(), "t".to_string()],
            "-foo -t=1.3 -bar   rest -of    the   query",
        )
        .unwrap();
        assert_eq!(flags, vec!["foo", "t=1.3", "bar"]);
        assert_eq!(query, "rest -of    the   query");
    }

    #[test]
    fn test_process_command_join() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("join", "alice", "bob", &mut memory).unwrap();
        assert_eq!(
            result,
            Some("bob joined memory with the group: alice".to_string())
        );

        // Verify the users are actually joined
        let joined_users = memory.get_joined_users("bob");
        assert!(joined_users.contains(&"alice".to_string()));
        assert!(joined_users.contains(&"bob".to_string()));
        assert_eq!(joined_users.len(), 2);
    }

    #[test]
    fn test_process_command_join_empty_user() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("join", "", "bob", &mut memory);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("Usage: join <username>".to_string()));
    }

    #[test]
    fn test_process_command_join_self() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("join", "bob", "bob", &mut memory);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("Usage: join <username>".to_string()));
    }

    #[test]
    fn test_process_command_unknown() {
        let dir = tempdir().unwrap();
        let mut memory = Memory::new_from_path(dir.path()).unwrap();

        let result = process_command("unknown", "args", "bob", &mut memory).unwrap();
        assert_eq!(result, None);
    }
}
