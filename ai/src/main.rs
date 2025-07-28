mod constants;
mod gettext;
mod memory;
mod model;
mod prompt;

use crate::constants::{CLEAR_MEMORY_FLAG, MAX_LINE_LENGTH, TEMPERATURE_FLAG};
use crate::memory::{Memory, Sender};
use crate::model::{Config, Model, ModelList};
use crate::prompt::{Message, build_prompt};

use formatx::formatx;
use gettextrs::{TextDomain, gettext, ngettext};
use std::io::Read;
use std::path::Path;
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
        model: &model.name,
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

fn sanitize_output(s: &str, api_key: &str) -> String {
    // An HTTPS request error might expose the API key by accident, so we redact it to be safe.
    // This is irrelevant for successful requests because the LLM doesn't know the API key.
    let redacted = s.replace(api_key, "[REDACTED]");
    let s_no_newlines: String = redacted.chars().filter(|&c| c != '\n').collect();
    if s_no_newlines.len() > MAX_LINE_LENGTH {
        format!("{}...", &s_no_newlines[..MAX_LINE_LENGTH])
    } else {
        s_no_newlines
    }
}

fn usage(models: &ModelList) -> String {
    formatget!(
        "Usage: !ai [-model] [-{}|-c] [-{}=1.0|-t=1.0] <query>.  Model options: {}.  Default model: {}",
        CLEAR_MEMORY_FLAG,
        TEMPERATURE_FLAG,
        models.list_model_flags_human_readable().join(" "),
        models.default_model_name()
    )
}

fn load_env(path: &Option<&Path>) {
    if let Some(p) = path {
        let _ = dotenv::from_path(p.join("api-keys"));
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
// Example: ["foo", "bar", "t"], "-foo -bar -t=foo   rest -of    the   query" -> ["foo", "bar", "t=foo"], "rest -of    the   query"
fn extract_flags(known_flags: &[String], query: &str) -> Result<(Vec<String>, String), String> {
    // Extract all -flags from query from the beginning until query no longer starts with - or we hit the end of string. Use string splitting or something, don't iterate over individual characters.
    // Collect all extracted flags which are not in known_flags. If non-empty, return all of them in the error.
    // Otherwise, return the extracted flags and the remaining query.
    let mut flags = Vec::new();
    let mut unknown_flags = Vec::new();
    let mut rest = query.trim_start();
    while let Some(stripped) = rest.strip_prefix('-') {
        let mut split = stripped.splitn(2, ' ');
        let flag_with_value = split.next().unwrap_or("").to_string();
        rest = split.next().unwrap_or("").trim_start();
        if flag_with_value.is_empty() {
            break;
        }
        let flag_name = flag_with_value.split('=').next().unwrap_or("");
        if known_flags.iter().any(|f| f == flag_name) {
            flags.push(flag_with_value);
        } else {
            unknown_flags.push(flag_with_value);
        }
    }
    if !unknown_flags.is_empty() {
        let unknown_flags_str = unknown_flags.join(", ");
        let unknown_flags_str = if unknown_flags_str.len() > 60 {
            unknown_flags_str[0..60].to_string() + "..."
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

fn main() {
    if let Some(dir) = textdomain_dir() {
        // Ignore errors and use untranslated strings if it fails.
        let _ = TextDomain::new("japanese_tools")
            .skip_system_data_paths()
            .push(&dir)
            .init();
    }

    let exe_path = std::env::current_exe().ok();
    let exe_parent_dir = exe_path.as_ref().and_then(|p| p.parent());
    load_env(&exe_parent_dir);
    load_env(&std::env::current_dir().ok().as_deref());

    let sender = std::env::var("DMB_SENDER").unwrap_or_default();
    let receiver = std::env::var("DMB_RECEIVER").unwrap_or_default();
    // Prevent usage in private messages
    if std::env::var("IRC_PLUGIN").ok().as_deref() == Some("1") && !receiver.starts_with('#') {
        println!("{}", gettext("!ai is only available in channels."));
        std::process::exit(1);
    }

    let cfg = Config::from_env();
    let models = ModelList::new(&cfg).unwrap_or_else(|err| {
        println!("{}", err);
        std::process::exit(1);
    });

    let query = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let known_flags = {
        let mut known_flags = models.list_model_flags();
        known_flags.push(CLEAR_MEMORY_FLAG.to_string());
        known_flags.push("c".to_string()); // Short for CLEAR_MEMORY_FLAG
        known_flags.push("temperature".to_string());
        known_flags.push("t".to_string()); // Short for temperature
        known_flags
    };
    if known_flags.len()
        != known_flags
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
    {
        println!(
            "{}",
            gettext("Internal error: duplicate configured flags detected, check your model config")
        );
        std::process::exit(1);
    }

    let (flags, query) = match extract_flags(&known_flags, &query) {
        Ok(res) => res,
        Err(err) => {
            println!("{}.  {}", err, usage(&models));
            std::process::exit(1);
        }
    };

    let model = match models.select_model(&flags) {
        Ok(m) => m,
        Err(err) => {
            println!("{}", err);
            std::process::exit(1);
        }
    };

    let mut memory = Memory::new_from_disk().unwrap_or_else(|err| {
        println!("{}", err);
        std::process::exit(1);
    });

    let history_cleared = if flags.iter().any(|f| f == CLEAR_MEMORY_FLAG || f == "c") {
        memory.clear_history(&sender, &receiver);
        true
    } else {
        false
    };

    if query.trim().is_empty() {
        if history_cleared {
            println!("[📜→🔥]");
        } else {
            println!("{}", usage(&models));
        }
        std::process::exit(0);
    }

    let prompt = build_prompt(&query, &sender, &receiver, &memory);
    memory.add_to_history(&sender, Sender::User, &receiver, &query);

    let _ = memory.save();

    let temperature = flags
        .iter()
        .find(|f| f.starts_with(&(TEMPERATURE_FLAG.to_string() + "=")) || f.starts_with("t="))
        .and_then(|f| f.split('=').nth(1))
        .and_then(|s| s.parse::<f64>().ok().map(|t| t.clamp(0.0, 2.0)));

    let result = match call_api(model, &prompt, &temperature) {
        Ok(res) => sanitize_output(&res, &model.api_key),
        Err(err) => {
            println!("{}", err);
            std::process::exit(1);
        }
    };
    memory.add_to_history(&sender, Sender::Assistant, &receiver, &result);
    let _ = memory.save();

    let flag_state = {
        let mut flag_state: Vec<String> = Vec::new();
        if model.name != models.default_model_name() {
            flag_state.push(model.short_name.as_ref().unwrap_or(&model.name).to_string());
        }
        if let Some(t) = temperature {
            flag_state.push(format!("t={}", t));
        }
        flag_state.join(" ")
    };
    let result = if flag_state.is_empty() {
        result
    } else {
        format!("[{}] {}", flag_state, result)
    };

    let result = if history_cleared {
        "[📜→🔥] ".to_string() + &result
    } else {
        result
    };

    // Prevent triggering other bots that might be present in the same channel.
    if let Some(first_char) = result.chars().next() {
        if first_char == '!' {
            print!(" ");
        }
    }
    println!("{}", result);
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
