mod constants;
mod gettext;
mod memory;
mod model;
mod prompt;

use crate::constants::{CLEAR_MEMORY_FLAG, MAX_LINE_LENGTH};
use crate::memory::{Memory, Sender};
use crate::model::{Config, Model, ModelList};
use crate::prompt::{Message, build_prompt};

use gettextrs::{TextDomain, gettext};
use std::io::Read;
use std::path::Path;
use std::time::Duration;

fn call_api(model: &Model, prompt: &Vec<Message>) -> Result<String, String> {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(formatget!("HTTP client error: {}", e)),
    };

    let payload = serde_json::json!({
        "model": model.name,
        "messages": prompt,
        "max_tokens": 300
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
    let redacted = s.replace(api_key, "[REDACTED]");
    let s_no_newlines: String = redacted.chars().filter(|&c| c != '\n').collect();
    if s_no_newlines.len() > MAX_LINE_LENGTH {
        format!("{}...", &s_no_newlines[..MAX_LINE_LENGTH])
    } else {
        s_no_newlines
    }
}

fn usage(models: &ModelList) {
    println!(
        "{}",
        formatget!(
            "Usage: !ai [-model] [-{}] <query>. Known models: {}. Default: {}",
            CLEAR_MEMORY_FLAG,
            models.list_models().join(" "),
            models.default_model_name()
        )
    );
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

// Extracts command line flags from the query. Returns the flags and the remaining query.
// Example: "-foo -bar   rest -of    the   query" -> (["foo", "bar"], "rest -of    the   query")
fn extract_flags(query: &str) -> (Vec<String>, String) {
    let mut flags = Vec::new();
    let mut rest = query;
    loop {
        rest = rest.trim_start();
        let stripped = match rest.strip_prefix('-') {
            Some(s) => s,
            None => break,
        };
        let flag_end = stripped.find(char::is_whitespace).unwrap_or(stripped.len());
        let flag = &stripped[..flag_end];
        flags.push(flag.to_string());
        rest = &stripped[flag_end..];
    }
    (flags, rest.to_string())
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

    let (flags, query) = extract_flags(&query);

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

    let history_cleared = if flags.iter().any(|f| f == CLEAR_MEMORY_FLAG) {
        memory.clear_history(&sender, &receiver);
        true
    } else {
        false
    };

    if query.trim().is_empty() {
        if history_cleared {
            println!("[📜→🔥]");
        } else {
            usage(&models);
        }
        std::process::exit(0);
    }

    let prompt = build_prompt(&query, &sender, &receiver, &memory);
    memory.add_to_history(&sender, Sender::User, &receiver, &query);

    let _ = memory.save();

    let result = match call_api(model, &prompt) {
        Ok(res) => sanitize_output(&res, &model.api_key),
        Err(err) => {
            println!("{}", err);
            std::process::exit(1);
        }
    };
    memory.add_to_history(&sender, Sender::Assistant, &receiver, &result);
    let _ = memory.save();

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
        let (flags, query) = extract_flags("-foo -bar   rest -of    the   query");
        assert_eq!(flags, vec!["foo", "bar"]);
        assert_eq!(query, "rest -of    the   query");
    }
}
