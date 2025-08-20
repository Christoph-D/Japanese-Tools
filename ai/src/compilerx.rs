use crate::constants::{
    COMPILER_CACHE_DURATION_SECS, COMPILER_CACHE_FILE_NAME, COMPILER_EXPLORER_MAX_RESPONSE_BYTES,
    COMPILER_EXPLORER_COMPILE_TIMEOUT,
};
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    #[serde(rename = "_internalid")]
    pub internal_id: u32,
    pub filters: serde_json::Value,
    pub libs: Vec<serde_json::Value>,
    pub overrides: Vec<serde_json::Value>,
    pub specialoutputs: Vec<serde_json::Value>,
    pub tools: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    pub id: u32,
    pub language: String,
    pub source: String,
    pub compilers: Vec<Compiler>,
    pub conformanceview: bool,
    pub executors: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShortlinkInfo {
    pub sessions: Vec<Session>,
    pub trees: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompilerInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CompilerCache {
    pub compilers: HashMap<String, CompilerInfo>,
    pub last_updated: u64,
}

#[derive(Debug, Serialize)]
struct CompilerOptions {
    #[serde(rename = "userArguments")]
    user_arguments: String,
    #[serde(rename = "compilerOptions")]
    compiler_options: CompilerOptionsInner,
    filters: CompilerFilters,
}

#[derive(Debug, Serialize)]
struct CompilerOptionsInner {
    #[serde(rename = "skipAsm")]
    skip_asm: bool,
    #[serde(rename = "executorRequest")]
    executor_request: bool,
    overrides: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CompilerFilters {
    binary: bool,
    #[serde(rename = "binaryObject")]
    binary_object: bool,
    #[serde(rename = "commentOnly")]
    comment_only: bool,
    demangle: bool,
    directives: bool,
    execute: bool,
    intel: bool,
    labels: bool,
    #[serde(rename = "libraryCode")]
    library_code: bool,
    trim: bool,
    #[serde(rename = "debugCalls")]
    debug_calls: bool,
}

#[derive(Debug, Serialize)]
struct CompilationRequest {
    source: String,
    options: CompilerOptions,
    lang: Option<String>,
    #[serde(rename = "allowStoreCodeDebug")]
    allow_store_code_debug: bool,
}

#[derive(Debug, Deserialize)]
struct CompilationResponse {
    code: i32,
    stdout: Vec<CompilerMessage>,
    stderr: Vec<CompilerMessage>,
    #[serde(default)]
    asm: Option<Vec<AssemblyLine>>,
}

#[derive(Debug, Deserialize)]
struct CompilerMessage {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AssemblyLine {
    text: String,
}

pub fn process_shortlinks(query: &str, config_path: &Path) -> Result<String, CompilerError> {
    let shortlink_ids = detect_shortlinks(query)?;
    if shortlink_ids.is_empty() {
        return Ok(query.to_string());
    }
    if shortlink_ids.len() > 1 {
        return Err(CompilerError::MultipleShortlinks(shortlink_ids.len()));
    }

    let info = fetch_shortlink_info(&shortlink_ids[0])?;
    validate_shortlink_info(&info)?;

    let cache_path = config_path.join(COMPILER_CACHE_FILE_NAME);
    let mut cache = load_compiler_cache(&cache_path)?;

    if is_cache_expired(&cache) {
        cache = refresh_compiler_cache(&cache_path).unwrap_or(cache);
    }

    let compilation_result = compile_shortlink_code(&info).ok();

    Ok(transform_query(query, &info, &cache, &compilation_result))
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

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn is_cache_expired(cache: &CompilerCache) -> bool {
    let current_time = get_current_timestamp();
    current_time.saturating_sub(cache.last_updated) > COMPILER_CACHE_DURATION_SECS
}

fn load_compiler_cache(cache_path: &Path) -> Result<CompilerCache, CompilerError> {
    if !cache_path.exists() {
        return Ok(CompilerCache {
            compilers: HashMap::new(),
            last_updated: 0,
        });
    }
    let content = fs::read_to_string(cache_path)
        .map_err(|e| CompilerError::InvalidResponse(format!("Failed to read cache file: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| CompilerError::InvalidResponse(format!("Failed to parse cache file: {}", e)))
}

fn save_compiler_cache(cache: &CompilerCache, cache_path: &Path) -> Result<(), CompilerError> {
    let content = serde_json::to_string_pretty(cache)
        .map_err(|e| CompilerError::InvalidResponse(format!("Failed to serialize cache: {}", e)))?;
    fs::write(cache_path, content)
        .map_err(|e| CompilerError::InvalidResponse(format!("Failed to write cache file: {}", e)))
}

fn fetch_compilers_from_api() -> Result<HashMap<String, CompilerInfo>, CompilerError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| CompilerError::NetworkError(format!("Client creation error: {}", e)))?;

    let response = client
        .get("https://godbolt.org/api/compilers")
        .header("Accept", "application/json")
        .send()
        .map_err(|e| CompilerError::NetworkError(format!("Request error: {}", e)))?;

    if !response.status().is_success() {
        return Err(CompilerError::ApiError(format!(
            "HTTP {} from Godbolt compilers API",
            response.status()
        )));
    }

    let text = response
        .text()
        .map_err(|e| CompilerError::NetworkError(format!("Read error: {}", e)))?;

    let compilers_list: Vec<CompilerInfo> = serde_json::from_str(&text)
        .map_err(|e| CompilerError::InvalidResponse(format!("JSON parsing error: {}", e)))?;

    let mut compilers_map = HashMap::new();
    for compiler in compilers_list {
        compilers_map.insert(compiler.id.clone(), compiler);
    }

    Ok(compilers_map)
}

fn refresh_compiler_cache(cache_path: &Path) -> Result<CompilerCache, CompilerError> {
    let compilers = fetch_compilers_from_api()?;
    let cache = CompilerCache {
        compilers,
        last_updated: get_current_timestamp(),
    };
    save_compiler_cache(&cache, cache_path)?;
    Ok(cache)
}

fn compile_shortlink_code(info: &ShortlinkInfo) -> Result<String, CompilerError> {
    let session = &info.sessions[0];
    let compiler = &session.compilers[0];

    let client = Client::builder()
        .timeout(COMPILER_EXPLORER_COMPILE_TIMEOUT)
        .build()
        .map_err(|e| CompilerError::NetworkError(format!("Client creation error: {}", e)))?;

    let request = CompilationRequest {
        source: session.source.clone(),
        lang: Some(session.language.clone()),
        allow_store_code_debug: true,
        options: CompilerOptions {
            user_arguments: compiler.options.clone(),
            compiler_options: CompilerOptionsInner {
                skip_asm: false,
                executor_request: false,
                overrides: vec![],
            },
            filters: CompilerFilters {
                binary: false,
                binary_object: false,
                comment_only: true,
                demangle: true,
                directives: true,
                execute: false,
                intel: true,
                labels: true,
                library_code: false,
                trim: false,
                debug_calls: false,
            },
        },
    };

    let url = format!("https://godbolt.org/api/compiler/{}/compile", compiler.id);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| CompilerError::NetworkError(format!("Request error: {}", e)))?;

    if !response.status().is_success() {
        return Err(CompilerError::ApiError(format!(
            "HTTP {} from Godbolt compilation API",
            response.status()
        )));
    }
    response.text().map_err(|e| CompilerError::NetworkError(format!("Response error: {}", e)))
}

fn transform_query(query: &str, info: &ShortlinkInfo, compiler_cache: &CompilerCache, compilation_result: &Option<String>) -> String {
    let session = &info.sessions[0];
    let compiler = &session.compilers[0];

    let compiler_name = compiler_cache
        .compilers
        .get(&compiler.id)
        .map(|info| &info.name)
        .unwrap_or(&compiler.id);

    let mut replacement = format!(
        "INCLUDED_SOURCE\n\n<INCLUDED_SOURCE (do not mention this name, this section is invisible to the user)>\nCompiler: \"{}\" {}\nSource:\n```{}\n{}\n```",
        compiler_name, compiler.options, session.language, session.source
    );

    // Add compilation output if available
    if let Some(compilation) = compilation_result {
        replacement.push_str(&format!("\n\nCompilation output:\n{}", compilation));
    }

    replacement.push_str("\n</INCLUDED_SOURCE>");

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

        let cache = CompilerCache {
            compilers: HashMap::new(),
            last_updated: 0,
        };

        let result = transform_query(query, &info, &cache, &None);
        
        // Check essential parts rather than exact string match
        assert!(result.starts_with("What's wrong with INCLUDED_SOURCE"));
        assert!(result.contains("<INCLUDED_SOURCE"));
        assert!(result.contains("Compiler: \"clang2010\" -O3"));
        assert!(result.contains("Source:"));
        assert!(result.contains("```c++"));
        assert!(result.contains("struct foo { int x; union { int y; char z[]; }};"));
        assert!(result.contains("</INCLUDED_SOURCE>"));
        assert!(result.ends_with("?"));
    }

    #[test]
    fn test_transform_query_with_compiler_name() {
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

        let mut compilers = HashMap::new();
        compilers.insert(
            "clang2010".to_string(),
            CompilerInfo {
                id: "clang2010".to_string(),
                name: "Clang 20.1.0".to_string(),
            },
        );

        let cache = CompilerCache {
            compilers,
            last_updated: get_current_timestamp(),
        };

        let result = transform_query(query, &info, &cache, &None);
        
        // Check essential parts rather than exact string match
        assert!(result.starts_with("What's wrong with INCLUDED_SOURCE"));
        assert!(result.contains("<INCLUDED_SOURCE"));
        assert!(result.contains("Compiler: \"Clang 20.1.0\" -O3"));
        assert!(result.contains("Source:"));
        assert!(result.contains("```c++"));
        assert!(result.contains("struct foo { int x; union { int y; char z[]; }};"));
        assert!(result.contains("</INCLUDED_SOURCE>"));
        assert!(result.ends_with("?"));
    }

    #[test]
    fn test_is_cache_expired() {
        let current_time = get_current_timestamp();

        let fresh_cache = CompilerCache {
            compilers: HashMap::new(),
            last_updated: current_time,
        };
        assert!(!is_cache_expired(&fresh_cache));

        let old_cache = CompilerCache {
            compilers: HashMap::new(),
            last_updated: current_time.saturating_sub(COMPILER_CACHE_DURATION_SECS + 1),
        };
        assert!(is_cache_expired(&old_cache));
    }

    #[test]
    fn test_load_compiler_cache_nonexistent() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("nonexistent.json");

        let cache = load_compiler_cache(&cache_path).unwrap();
        assert!(cache.compilers.is_empty());
        assert_eq!(cache.last_updated, 0);
    }

    #[test]
    fn test_transform_query_with_compilation_output() {
        let query = "What's wrong with https://godbolt.org/z/9E9M3GK5c?";
        let info = ShortlinkInfo {
            sessions: vec![Session {
                id: 1,
                language: "c++".to_string(),
                source: "int main() { return 0; }".to_string(),
                compilers: vec![Compiler {
                    id: "clang2010".to_string(),
                    options: "-O3".to_string(),
                }],
            }],
        };

        let cache = CompilerCache {
            compilers: HashMap::new(),
            last_updated: 0,
        };

        let compilation_output = "Compilation successful".to_string();
        let result = transform_query(query, &info, &cache, &Some(compilation_output));
        
        assert!(result.starts_with("What's wrong with INCLUDED_SOURCE"));
        assert!(result.contains("<INCLUDED_SOURCE"));
        assert!(result.contains("Compiler: \"clang2010\" -O3"));
        assert!(result.contains("Source:"));
        assert!(result.contains("```c++"));
        assert!(result.contains("int main() { return 0; }"));
        assert!(result.contains("Compilation output:"));
        assert!(result.contains("Compilation successful"));
        assert!(result.contains("</INCLUDED_SOURCE>"));
        assert!(result.ends_with("?"));
    }

    #[test]
    fn test_save_and_load_compiler_cache() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("test_cache.json");

        let mut compilers = HashMap::new();
        compilers.insert(
            "test_id".to_string(),
            CompilerInfo {
                id: "test_id".to_string(),
                name: "Test Compiler".to_string(),
            },
        );

        let original_cache = CompilerCache {
            compilers,
            last_updated: 12345,
        };

        save_compiler_cache(&original_cache, &cache_path).unwrap();
        let loaded_cache = load_compiler_cache(&cache_path).unwrap();

        assert_eq!(loaded_cache.last_updated, 12345);
        assert_eq!(loaded_cache.compilers.len(), 1);
        assert_eq!(
            loaded_cache.compilers.get("test_id").unwrap().name,
            "Test Compiler"
        );
    }
}
