use mockito::Server;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_ai_binary_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let mut server = Server::new();
    let mock_url = server.url();

    let _mock = server
        .mock("POST", "/v1/chat/completions")
        .match_header("authorization", "Bearer test-api-key")
        .match_header("content-type", "application/json")
        .match_body(mockito::Matcher::PartialJsonString(
            serde_json::json!({
                "model": "test-model",
                "messages": [
                    {
                        "role": "system"
                    },
                    {
                        "role": "user",
                        "content": "Hello, this is a test query"
                    }
                ],
                "max_tokens": 500
            })
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "choices": [{
                    "message": {
                        "content": "Test AI response from mock server"
                    }
                }]
            })
            .to_string(),
        )
        .create();

    let env_content = "LITELLM_API_KEY=test-api-key\n";
    fs::write(temp_path.join(".env"), env_content).expect("Failed to write .env file");

    let config_content = format!(
        r#"[general]
default_model = "test-model"

[providers.litellm]
endpoint = "{}/v1/chat/completions"
models = [
    {{ id = "test-model", short_name = "tm", name = "Test Model" }}
]
"#,
        mock_url
    );
    fs::write(temp_path.join("config.toml"), config_content)
        .expect("Failed to write config.toml file");

    let binary_path = env!("CARGO_BIN_EXE_ai");
    let output = Command::new(binary_path)
        .current_dir(temp_path)
        .arg("Hello, this is a test query")
        .output()
        .expect("Failed to execute ai binary");

    if !output.status.success() {
        eprintln!("Binary stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("Binary stdout: {}", String::from_utf8_lossy(&output.stdout));
        panic!("Binary execution failed with status: {}", output.status);
    }

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8 in stdout");
    assert!(
        stdout.eq("Test AI response from mock server\n"),
        "Expected response not found in output: {}",
        stdout
    );

    _mock.assert();
}

#[test]
fn test_ai_binary_with_temperature_flag() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let mut server = Server::new();
    let mock_url = server.url();

    let _mock = server
        .mock("POST", "/v1/chat/completions")
        .match_header("authorization", "Bearer test-api-key")
        .match_header("content-type", "application/json")
        .match_body(mockito::Matcher::PartialJsonString(
            serde_json::json!({"temperature": 0.8}).to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "choices": [{
                    "message": {
                        "content": "Response with custom temperature"
                    }
                }]
            })
            .to_string(),
        )
        .create();

    let env_content = "LITELLM_API_KEY=test-api-key\n";
    fs::write(temp_path.join(".env"), env_content).expect("Failed to write .env file");

    let config_content = format!(
        r#"[general]
default_model = "test-model"

[providers.litellm]
endpoint = "{}/v1/chat/completions"
models = [
    {{ id = "test-model", short_name = "tm", name = "Test Model" }}
]
"#,
        mock_url
    );
    fs::write(temp_path.join("config.toml"), config_content)
        .expect("Failed to write config.toml file");

    let binary_path = env!("CARGO_BIN_EXE_ai");
    let output = Command::new(binary_path)
        .current_dir(temp_path)
        .arg("-t=0.8")
        .arg("Test query with temperature")
        .output()
        .expect("Failed to execute ai binary");

    if !output.status.success() {
        eprintln!("Binary stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("Binary stdout: {}", String::from_utf8_lossy(&output.stdout));
        panic!("Binary execution failed with status: {}", output.status);
    }

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8 in stdout");
    assert!(
        stdout.eq("[t=0.8] Response with custom temperature\n"),
        "Expected response not found in output: {}",
        stdout
    );

    _mock.assert();
}

#[test]
fn test_ai_binary_server_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let env_content = "LITELLM_API_KEY=test-api-key\n";
    fs::write(temp_path.join(".env"), env_content).expect("Failed to write .env file");

    let config_content = r#"[general]
default_model = "test-model"

[providers.litellm]
endpoint = "http://localhost:99999/v1/chat/completions"
models = [
    { id = "test-model", short_name = "tm", name = "Test Model" }
]
"#;
    fs::write(temp_path.join("config.toml"), config_content)
        .expect("Failed to write config.toml file");

    let binary_path = env!("CARGO_BIN_EXE_ai");
    let output = Command::new(binary_path)
        .current_dir(temp_path)
        .arg("Test query")
        .output()
        .expect("Failed to execute ai binary");

    assert!(
        !output.status.success(),
        "Binary should have failed due to server error"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8 in stdout");
    assert!(
        stdout.contains("API error") || stdout.contains("Failed to read response"),
        "Expected error message not found in output: {}",
        stdout
    );
}
