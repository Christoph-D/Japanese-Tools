use std::{collections::HashMap, sync::LazyLock};
use tokenizers::tokenizer::Tokenizer;

struct TokenizerRegistry {
    tokenizers: HashMap<String, String>,
    default_tokenizer: String,
}

impl TokenizerRegistry {
    fn new(default_tokenizer: String) -> Self {
        Self {
            tokenizers: HashMap::new(),
            default_tokenizer,
        }
    }

    fn add_tokenizer(mut self, key: &str, value: &str) -> Self {
        self.tokenizers.insert(key.to_string(), value.to_string());
        self
    }

    fn get_options(&self) -> Vec<&str> {
        self.tokenizers.keys().map(|k| k.as_str()).collect()
    }

    // Returns (tokenizer, rest)
    fn parse_input(&self, input: &str) -> (String, String) {
        let (p, rest) = input.split_once(' ').unwrap_or((input, ""));
        if let Some(tokenizer) = self.tokenizers.get(p) {
            return (tokenizer.clone(), rest.to_string());
        }
        (self.default_tokenizer.clone(), input.to_string())
    }
}

fn main() {
    let tokenizer_registry = TokenizerRegistry::new("deepseek-ai/DeepSeek-V3-0324".to_string())
        .add_tokenizer("-llama3", "Xenova/llama-3-tokenizer");

    let input = std::env::args()
        .nth(1)
        .unwrap_or_default()
        .trim()
        .to_string();
    if input.is_empty() || input == "help" {
        let options = tokenizer_registry.get_options();
        let usage = if options.is_empty() {
            "Usage: !tok <text>".to_string()
        } else {
            format!("Usage: !tok [{}] <text>", options.join("|"))
        };
        println!("{}", usage);
        return;
    }

    let (name, input) = tokenizer_registry.parse_input(&input);
    let tokenizer = match Tokenizer::from_pretrained(name, None) {
        Ok(t) => t,
        Err(e) => {
            println!("Failed to load tokenizer: {}", e);
            std::process::exit(1);
        }
    };

    let encoded = match tokenizer.encode(input.to_string(), false) {
        Ok(s) => s,
        Err(e) => {
            println!("Encode error: {}", e);
            return;
        }
    };
    let result: Vec<String> = encoded
        .get_ids()
        .iter()
        .map(|id| format!("\"{}\"", decode_token(&tokenizer, *id)))
        .collect();
    println!("{}", result.join(", "));
}

/// Decodes a token using the tokenizer, falling back to hex string representation if decoding fails.
fn decode_token(tokenizer: &Tokenizer, token_id: u32) -> String {
    tokenizer
        // Try to use tokenizer.decode() first.
        .decode(&[token_id], false)
        .ok()
        .filter(|s| s != "\u{FFFD}") // "Missing character" symbol
        // Fall back to our own decoding if tokenizer.decode() fails to return something useful.
        // We blindly assume that the token uses ByteLevel
        .unwrap_or_else(|| token_to_hex_string(tokenizer, token_id))
}

/// Converts unicode characters to UTF-8 bytes.
/// The inverse of https://github.com/openai/gpt-2/blob/master/src/encoder.py#L9
fn char_bytes() -> HashMap<char, u8> {
    let mut cs: Vec<(u8, u32)> = (b'!'..=b'~')
        .chain(b'\xA1'..=b'\xAC')
        .chain(b'\xAE'..=b'\xFF')
        .map(|i| (i, i as u32))
        .collect();
    let mut n = 0;
    for b in 0..=255u8 {
        if !cs.iter().any(|(x, _)| *x == b) {
            cs.push((b, 256 + n));
            n += 1;
        }
    }
    cs.into_iter()
        .map(|(byte, codepoint)| (std::char::from_u32(codepoint).unwrap(), byte))
        .collect()
}

static CHAR_MAP: LazyLock<HashMap<char, u8>> = LazyLock::new(char_bytes);

fn token_to_hex_string(tokenizer: &Tokenizer, token_id: u32) -> String {
    if let Some(t) = tokenizer.id_to_token(token_id) {
        t.chars()
            .filter_map(|c| CHAR_MAP.get(&c).copied())
            .map(|b| format!("\\x{:02x}", b))
            .collect()
    } else {
        format!("<{}>", token_id)
    }
}
