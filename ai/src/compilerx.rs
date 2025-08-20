use crate::constants::COMPILER_EXPLORER_MAX_RESPONSE_BYTES;
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerError {
    MultipleShortlinks(usize),
    NetworkError(String),
    ApiError(String),
    InvalidResponse(String),
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::MultipleShortlinks(count) => {
                write!(f, "Multiple shortlinks found ({}), only one allowed", count)
            }
            CompilerError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            CompilerError::ApiError(msg) => write!(f, "API error: {}", msg),
            CompilerError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl std::error::Error for CompilerError {}

#[derive(Debug, Deserialize, Serialize)]
pub struct Compiler {
    pub id: String,
    pub options: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    pub id: u32,
    pub language: String,
    pub source: String,
    pub compilers: Vec<Compiler>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShortlinkInfo {
    pub sessions: Vec<Session>,
}

pub fn process_shortlinks(query: &str) -> Result<String, CompilerError> {
    let shortlink_ids = detect_shortlinks(query)?;
    if shortlink_ids.is_empty() {
        return Ok(query.to_string());
    }
    if shortlink_ids.len() > 1 {
        return Err(CompilerError::MultipleShortlinks(shortlink_ids.len()));
    }

    let info = fetch_shortlink_info(&shortlink_ids[0])?;
    validate_shortlink_info(&info)?;
    Ok(transform_query(query, &info))
}

fn detect_shortlinks(query: &str) -> Result<Vec<String>, CompilerError> {
    let shortlink_regex = Regex::new(r"https://godbolt\.org/z/([A-Za-z0-9]{6,12})\b").unwrap();
    let mut shortlink_ids = Vec::new();
    for capture in shortlink_regex.captures_iter(query) {
        if let Some(id_match) = capture.get(1) {
            let id = id_match.as_str().to_string();
            shortlink_ids.push(id);
        }
    }
    Ok(shortlink_ids)
}

fn fetch_shortlink_info(id: &str) -> Result<ShortlinkInfo, CompilerError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| CompilerError::NetworkError(format!("Client creation error: {}", e)))?;
    let url = format!("https://godbolt.org/api/shortlinkinfo/{}", id);
    let response = client
        .get(&url)
        .send()
        .map_err(|e| CompilerError::NetworkError(format!("Request error: {}", e)))?;

    if !response.status().is_success() {
        return Err(CompilerError::ApiError(format!(
            "HTTP {} from Godbolt API",
            response.status()
        )));
    }

    let mut buffer = Vec::new();

    use std::io::Read;
    // Limit the response size for safety
    let mut limited_reader = response.take(COMPILER_EXPLORER_MAX_RESPONSE_BYTES);
    limited_reader
        .read_to_end(&mut buffer)
        .map_err(|e| CompilerError::NetworkError(format!("Read error: {}", e)))?;
    if buffer.len() as u64 == COMPILER_EXPLORER_MAX_RESPONSE_BYTES {
        return Err(CompilerError::InvalidResponse(format!(
            "Response too large (exceeded {} bytes)",
            COMPILER_EXPLORER_MAX_RESPONSE_BYTES
        )));
    }

    let text = String::from_utf8(buffer)
        .map_err(|e| CompilerError::InvalidResponse(format!("UTF-8 decode error: {}", e)))?;
    serde_json::from_str(&text)
        .map_err(|e| CompilerError::InvalidResponse(format!("JSON parsing error: {}", e)))
}

fn validate_shortlink_info(info: &ShortlinkInfo) -> Result<(), CompilerError> {
    if info.sessions.len() != 1 {
        return Err(CompilerError::InvalidResponse(format!(
            "Expected exactly 1 session, got {}",
            info.sessions.len()
        )));
    }

    let session = &info.sessions[0];
    if session.compilers.len() != 1 {
        return Err(CompilerError::InvalidResponse(format!(
            "Expected exactly 1 compiler, got {}",
            session.compilers.len()
        )));
    }

    Ok(())
}

fn transform_query(query: &str, info: &ShortlinkInfo) -> String {
    let session = &info.sessions[0];
    let compiler = &session.compilers[0];
    let replacement = format!(
        "INCLUDED_SOURCE\n\n<INCLUDED_SOURCE (do not mention the name)>\nCompiler: {} {}\nSource:\n```{}\n{}\n```\n</INCLUDED_SOURCE>",
        compiler.id, compiler.options, session.language, session.source
    );
    let shortlink_regex = Regex::new(r"https://godbolt\.org/z/[A-Za-z0-9]{6,12}\b")
        .expect("Shortlink regex should be valid");
    shortlink_regex
        .replace_all(query, replacement.as_str())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shortlinks_valid() {
        let query = "What's wrong with https://godbolt.org/z/9E9M3GK5c?";
        let result = detect_shortlinks(query).unwrap();
        assert_eq!(result, vec!["9E9M3GK5c"]);
    }

    #[test]
    fn test_detect_shortlinks_invalid_too_short() {
        let query = "https://godbolt.org/z/12345";
        let result = detect_shortlinks(query).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_detect_shortlinks_invalid_too_long() {
        let query = "https://godbolt.org/z/1234567890123";
        let result = detect_shortlinks(query).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_detect_shortlinks_invalid_characters() {
        let query = "https://godbolt.org/z/12345@abc";
        let result = detect_shortlinks(query).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_detect_shortlinks_multiple() {
        let query = "Compare https://godbolt.org/z/9E9M3GK5c with https://godbolt.org/z/abcd1234";
        let result = detect_shortlinks(query).unwrap();
        assert_eq!(result, vec!["9E9M3GK5c", "abcd1234"]);
    }

    #[test]
    fn test_detect_shortlinks_none() {
        let query = "Just a normal query without shortlinks";
        let result = detect_shortlinks(query).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_validate_shortlink_info_valid() {
        let info = ShortlinkInfo {
            sessions: vec![Session {
                id: 1,
                language: "c++".to_string(),
                source: "int main() {}".to_string(),
                compilers: vec![Compiler {
                    id: "clang2010".to_string(),
                    options: "-O3".to_string(),
                }],
            }],
        };

        assert!(validate_shortlink_info(&info).is_ok());
    }

    #[test]
    fn test_validate_shortlink_info_multiple_sessions() {
        let info = ShortlinkInfo {
            sessions: vec![
                Session {
                    id: 1,
                    language: "c++".to_string(),
                    source: "int main() {}".to_string(),
                    compilers: vec![Compiler {
                        id: "clang2010".to_string(),
                        options: "-O3".to_string(),
                    }],
                },
                Session {
                    id: 2,
                    language: "c".to_string(),
                    source: "int main() {}".to_string(),
                    compilers: vec![Compiler {
                        id: "gcc".to_string(),
                        options: "-O2".to_string(),
                    }],
                },
            ],
        };

        let result = validate_shortlink_info(&info);
        assert!(result.is_err());
        assert!(matches!(result, Err(CompilerError::InvalidResponse(_))));
    }

    #[test]
    fn test_validate_shortlink_info_multiple_compilers() {
        let info = ShortlinkInfo {
            sessions: vec![Session {
                id: 1,
                language: "c++".to_string(),
                source: "int main() {}".to_string(),
                compilers: vec![
                    Compiler {
                        id: "clang2010".to_string(),
                        options: "-O3".to_string(),
                    },
                    Compiler {
                        id: "gcc".to_string(),
                        options: "-O2".to_string(),
                    },
                ],
            }],
        };

        let result = validate_shortlink_info(&info);
        assert!(result.is_err());
        assert!(matches!(result, Err(CompilerError::InvalidResponse(_))));
    }

    #[test]
    fn test_transform_query() {
        let query = "What's wrong with https://godbolt.org/z/9E9M3GK5c?";
        let info = ShortlinkInfo {
            sessions: vec![Session {
                id: 1,
                language: "c++".to_string(),
                source: "struct foo { int x; union { int y; char z[]; }};".to_string(),
                compilers: vec![Compiler {
                    id: "clang2010".to_string(),
                    options: "-O3".to_string(),
                }],
            }],
        };

        let result = transform_query(query, &info);
        let expected = "What's wrong with INCLUDED_SOURCE\n\n<INCLUDED_SOURCE (do not mention the name)>\nCompiler: clang2010 -O3\nSource:\n```c++\nstruct foo { int x; union { int y; char z[]; }};\n```\n</INCLUDED_SOURCE>?";

        assert_eq!(result, expected);
    }
}
