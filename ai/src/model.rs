use gettextrs::gettext;
use std::collections::HashMap;

use crate::EnvVars;
use crate::constants::{CONFIG_FILE_NAME, DEFAULT_TIMEOUT, DEFAULT_TIMEOUT_REASONING};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Provider {
    name: String,
    api_key: String,
    endpoint: String,
    models: Vec<TomlModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    providers: Vec<Provider>,
    default_model_id: String,
    channels: HashMap<String, ChannelConfig>,
    enable_compiler_explorer: bool,
    timeout: u64,
    timeout_reasoning: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelConfig {
    pub default_model: Option<String>,
    pub temperature: Option<f64>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub id: String,
    pub short_name: String,
    pub name: String,
    pub api_key: String,
    pub endpoint: String,
    pub reasoning: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelList {
    models: Vec<Model>,
    default_model_index: usize,
}

const DEEPSEEK_API_ENDPOINT: &str = "https://api.deepseek.com/v1/chat/completions";
const MISTRAL_API_ENDPOINT: &str = "https://api.mistral.ai/v1/chat/completions";
const OPENROUTER_API_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";
const ANTHROPIC_API_ENDPOINT: &str = "https://api.anthropic.com/v1/chat/completions";
const Z_AI_API_ENDPOINT: &str = "https://api.z.ai/api/paas/v4/chat/completions";
const Z_AI_CODE_API_ENDPOINT: &str = "https://api.z.ai/api/coding/paas/v4/chat/completions";

#[derive(Debug, serde::Deserialize)]
struct TomlConfig {
    general: TomlGeneral,
    providers: HashMap<String, TomlProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channels: Option<HashMap<String, TomlChannel>>,
}

#[derive(Debug, serde::Deserialize)]
struct TomlGeneral {
    default_model: String,
    #[serde(default)]
    enable_compiler_explorer: bool,
    #[serde(default = "default_timeout")]
    timeout: u64,
    #[serde(default = "default_timeout_reasoning")]
    timeout_reasoning: u64,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT
}

fn default_timeout_reasoning() -> u64 {
    DEFAULT_TIMEOUT_REASONING
}

#[derive(Debug, serde::Deserialize)]
struct TomlProvider {
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<String>,
    models: Vec<TomlModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct TomlModel {
    id: String,
    short_name: String,
    name: String,
    #[serde(default)]
    reasoning: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TomlChannel {
    #[serde(skip_serializing_if = "Option::is_none")]
    default_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
}

impl Config {
    pub fn new(config_path: &std::path::Path, env_vars: &EnvVars) -> Result<Self, String> {
        let toml_path = config_path.join(CONFIG_FILE_NAME);
        let toml_content = std::fs::read_to_string(toml_path)
            .map_err(|e| format!("Failed to read {}: {}", CONFIG_FILE_NAME, e))?;
        let toml_config: TomlConfig = toml::from_str(&toml_content)
            .map_err(|e| format!("Failed to parse {}: {}", CONFIG_FILE_NAME, e))?;

        let mut providers = Vec::new();
        for (provider_name, toml_provider) in toml_config.providers {
            let env_prefix = provider_name.to_uppercase().replace("-", "_");
            match provider_name.as_str() {
                "litellm" => {
                    if toml_provider
                        .endpoint
                        .as_deref()
                        .unwrap_or_default()
                        .is_empty()
                    {
                        return Err(format!(
                            "{}: LiteLLM provider requires an endpoint.",
                            CONFIG_FILE_NAME
                        ));
                    }
                }
                "anthropic" | "deepseek" | "mistral" | "openrouter" | "z-ai" | "z-ai-code" => {
                    if toml_provider.endpoint.is_some() {
                        return Err(format!(
                            "{}: Provider '{}' endpoint is not configurable.",
                            CONFIG_FILE_NAME, provider_name
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "{}: Unknown provider: {}",
                        CONFIG_FILE_NAME, provider_name
                    ));
                }
            }
            if let Some(api_key) = env_vars.get(&format!("{}_API_KEY", env_prefix))
                && !api_key.is_empty()
            {
                let endpoint = match provider_name.as_str() {
                    "anthropic" => ANTHROPIC_API_ENDPOINT.to_string(),
                    "deepseek" => DEEPSEEK_API_ENDPOINT.to_string(),
                    "mistral" => MISTRAL_API_ENDPOINT.to_string(),
                    "openrouter" => OPENROUTER_API_ENDPOINT.to_string(),
                    "z-ai" => Z_AI_API_ENDPOINT.to_string(),
                    "z-ai-code" => Z_AI_CODE_API_ENDPOINT.to_string(),
                    "litellm" => toml_provider.endpoint.unwrap(), // we validated above
                    _ => unreachable!(),                          // validated above
                };
                providers.push(Provider {
                    name: Self::provider_display_name(&provider_name),
                    api_key: api_key.clone(),
                    endpoint,
                    models: toml_provider.models,
                });
            }
        }
        let channels = toml_config
            .channels
            .unwrap_or_default()
            .into_iter()
            .map(|(name, channel)| {
                (
                    name,
                    ChannelConfig {
                        default_model: channel.default_model,
                        temperature: channel.temperature,
                        system_prompt: channel.system_prompt,
                    },
                )
            })
            .collect();

        Ok(Config {
            providers,
            default_model_id: toml_config.general.default_model,
            channels,
            enable_compiler_explorer: toml_config.general.enable_compiler_explorer,
            timeout: toml_config.general.timeout,
            timeout_reasoning: toml_config.general.timeout_reasoning,
        })
    }

    fn provider_display_name(provider_name: &str) -> String {
        match provider_name {
            "anthropic" => "Anthropic".to_string(),
            "deepseek" => "Deepseek".to_string(),
            "litellm" => "LiteLLM".to_string(),
            "mistral" => "Mistral".to_string(),
            "openrouter" => "OpenRouter".to_string(),
            "z-ai" => "Z.AI".to_string(),
            "z-ai-code" => "Z.AI Code".to_string(),
            _ => provider_name.to_string(),
        }
    }

    pub fn get_channel_system_prompt(&self, channel_name: &str) -> Option<&str> {
        self.channels
            .get(channel_name)
            .and_then(|c| c.system_prompt.as_deref())
    }

    pub fn get_channel_default_model(&self, channel_name: &str) -> &str {
        self.channels
            .get(channel_name)
            .and_then(|c| c.default_model.as_deref())
            .unwrap_or(&self.default_model_id)
    }

    pub fn get_channel_temperature(&self, channel_name: &str) -> Option<f64> {
        self.channels.get(channel_name).and_then(|c| c.temperature)
    }

    pub fn is_compiler_explorer_enabled(&self) -> bool {
        self.enable_compiler_explorer
    }

    pub fn get_timeout(&self, reasoning: bool) -> u64 {
        if reasoning {
            self.timeout_reasoning
        } else {
            self.timeout
        }
    }
}

impl ModelList {
    pub fn new(cfg: &Config) -> Result<Self, String> {
        let mut models = Vec::new();
        for provider in &cfg.providers {
            for toml_model in &provider.models {
                models.push(Model {
                    id: toml_model.id.to_string(),
                    short_name: toml_model.short_name.to_string(),
                    name: toml_model.name.to_string(),
                    api_key: provider.api_key.clone(),
                    endpoint: provider.endpoint.clone(),
                    reasoning: toml_model.reasoning,
                });
            }
        }
        if models.is_empty() {
            return Err(gettext("Missing API keys or model configuration"));
        }
        let default_model_index = models
            .iter()
            .position(|m| m.id == cfg.default_model_id)
            .ok_or_else(|| gettext("Default model not found"))?;
        Ok(ModelList {
            models,
            default_model_index,
        })
    }

    fn default_model(&self) -> &Model {
        self.models.get(self.default_model_index).unwrap()
    }

    pub fn default_model_name(&self) -> &str {
        &self.default_model().name
    }

    pub fn select_model_for_channel(
        &self,
        flags: &[String],
        channel_default_model_id: &str,
    ) -> Result<&Model, String> {
        // Find the last flag that matches a model name
        let mut selected: Option<&Model> = None;
        for f in flags.iter() {
            if let Some(model) = self.models.iter().find(|m| m.short_name == *f) {
                selected = Some(model);
            }
        }
        if let Some(model) = selected {
            return Ok(model);
        }

        // Use channel default model if available
        if let Some(model) = self
            .models
            .iter()
            .find(|m| m.id == channel_default_model_id)
        {
            return Ok(model);
        }

        Ok(self.default_model())
    }

    pub fn list_model_flags_human_readable(&self, channel_default_id: &str) -> Vec<String> {
        self.models
            .iter()
            .filter(|m| m.id != channel_default_id)
            .map(|m| format!("[{}]{}", m.short_name, m.name))
            .collect::<Vec<String>>()
    }

    pub fn list_model_flags_without_default(&self, channel_default_id: &str) -> Vec<String> {
        self.models
            .iter()
            .filter(|m| m.id != channel_default_id)
            .map(|m| m.short_name.clone())
            .collect::<Vec<String>>()
    }

    pub fn list_model_flags(&self) -> Vec<String> {
        self.models
            .iter()
            .map(|m| m.short_name.clone())
            .collect::<Vec<String>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_model_list() -> ModelList {
        let models = vec![
            Model {
                id: "deepseek-1".to_string(),
                short_name: "d".to_string(),
                name: "Deepseek".to_string(),
                api_key: "key1".to_string(),
                endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
                reasoning: false,
            },
            Model {
                id: "openrouter-2".to_string(),
                short_name: "o".to_string(),
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                reasoning: false,
            },
        ];
        ModelList {
            models,
            default_model_index: 0,
        }
    }

    #[test]
    fn test_new_returns_error_when_no_env_vars() {
        let cfg = Config {
            providers: vec![],
            default_model_id: "".to_string(),
            channels: HashMap::new(),
            enable_compiler_explorer: false,
        };
        let result = ModelList::new(&cfg);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Missing API keys or model configuration"
        );
    }
    #[test]
    fn test_new_parses_deepseek_env_vars() {
        // Clear any existing DEEPSEEK_API_KEY to avoid pollution
        unsafe {
            std::env::remove_var("DEEPSEEK_API_KEY");
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "DEEPSEEK_API_KEY=key1\n").unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r#"
[general]
default_model = "deepseek-1"

[providers.deepseek]
models = [
  { id = "deepseek-1", short_name = "short1", name = "Deepseek 1" },
  { id = "deepseek-2", short_name = "short2", name = "Deepseek 2" }
]
"#,
        )
        .unwrap();
        let env_vars = EnvVars::from_file(&config_dir).unwrap();
        let cfg = Config::new(&config_dir, &env_vars).expect("Config::new()");
        let model_list = ModelList::new(&cfg).expect("new()");
        assert_eq!(
            model_list.models,
            vec![
                Model {
                    id: "deepseek-1".to_string(),
                    short_name: "short1".to_string(),
                    name: "Deepseek 1".to_string(),
                    api_key: "key1".to_string(),
                    endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
                    reasoning: false,
                },
                Model {
                    id: "deepseek-2".to_string(),
                    short_name: "short2".to_string(),
                    name: "Deepseek 2".to_string(),
                    api_key: "key1".to_string(),
                    endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
                    reasoning: false,
                }
            ]
        );
        assert_eq!(model_list.default_model_name(), "Deepseek 1");
    }

    #[test]
    fn test_new_parses_openrouter() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "OPENROUTER_API_KEY=key2\n").unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r#"
[general]
default_model = "openrouter-1"

[providers.openrouter]
models = [
  { id = "openrouter-1", short_name = "o", name = "OpenRouter 1" },
  { id = "openrouter-2", short_name = "p", name = "OpenRouter 2" }
]
"#,
        )
        .unwrap();
        let env_vars = EnvVars::from_file(&config_dir).unwrap();
        let cfg = Config::new(&config_dir, &env_vars).expect("Config::new()");
        let model_list = ModelList::new(&cfg).expect("new()");
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].id, "openrouter-1");
        assert_eq!(model_list.models[0].short_name, "o".to_string());
    }

    #[test]
    fn test_new_unknown_default_model_fails() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "OPENROUTER_API_KEY=key2\n").unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r#"
[general]
default_model = "unknown_model"

[providers.openrouter]
models = [
  { id = "openrouter-1", short_name = "o", name = "OpenRouter-1" },
  { id = "openrouter-2", short_name = "p", name = "OpenRouter-2" }
]
"#,
        )
        .unwrap();
        let env_vars = EnvVars::from_file(&config_dir).unwrap();
        let cfg = Config::new(&config_dir, &env_vars).expect("Config::new()");
        let err = ModelList::new(&cfg).unwrap_err();
        assert!(err.contains("Default model"), "{}", err);
    }

    #[test]
    fn test_list_models_returns_all_model_names() {
        let model_list = setup_model_list();
        assert_eq!(model_list.list_model_flags(), vec!["d", "o"]);
    }

    #[test]
    fn test_list_models_human_readable_excludes_default() {
        let model_list = setup_model_list();
        assert_eq!(
            model_list.list_model_flags_human_readable("deepseek-1"),
            vec!["[o]OpenRouter"]
        );
    }

    #[test]
    fn test_list_models_human_readable_for_channel_different_default() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "DEEPSEEK_API_KEY=test-key\n").unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r##"
[general]
default_model = "deepseek-1"

[providers.deepseek]
models = [
  { id = "deepseek-1", short_name = "d", name = "Deepseek" },
  { id = "deepseek-2", short_name = "d2", name = "Deepseek 2" }
]

[channels]
"#test" = { default_model = "deepseek-2" }
"##,
        )
        .unwrap();

        let env_vars = EnvVars::from_file(&config_dir).unwrap();
        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");
        let model_list = ModelList::new(&config).expect("ModelList::new()");

        let channel_default = config.get_channel_default_model("#test");
        assert_eq!(channel_default, "deepseek-2");

        assert_eq!(
            model_list.list_model_flags_human_readable(channel_default),
            vec!["[d]Deepseek"]
        );
    }

    #[test]
    fn test_list_models_without_default_for_channel() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(
            &env_file,
            "DEEPSEEK_API_KEY=test-key\nOPENROUTER_API_KEY=test-key2\n",
        )
        .unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r##"
[general]
default_model = "deepseek-1"

[providers.deepseek]
models = [
  { id = "deepseek-1", short_name = "d", name = "Deepseek" }
]

[providers.openrouter]
models = [
  { id = "openrouter-1", short_name = "o", name = "OpenRouter" }
]

[channels]
"#test" = { default_model = "openrouter-1" }
"##,
        )
        .unwrap();

        let env_vars = EnvVars::from_file(&config_dir).unwrap();
        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");
        let model_list = ModelList::new(&config).expect("ModelList::new()");

        let channel_default = config.get_channel_default_model("#test");
        assert_eq!(channel_default, "openrouter-1");

        assert_eq!(
            model_list.list_model_flags_without_default(channel_default),
            vec!["d"]
        );
    }

    #[test]
    fn test_list_models_with_single_model() {
        let models = vec![Model {
            id: "only-model".to_string(),
            short_name: "o".to_string(),
            name: "Only Model".to_string(),
            api_key: "key".to_string(),
            endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
            reasoning: false,
        }];
        let model_list = ModelList {
            models,
            default_model_index: 0,
        };
        assert_eq!(model_list.list_model_flags(), vec!["o"]);
    }

    #[test]
    fn test_select_model_for_channel_with_channel_default() {
        let model_list = setup_model_list();
        let result = model_list.select_model_for_channel(&vec![], "openrouter-2");
        let model = result.expect("select_model_for_channel()");
        assert_eq!(model.id, "openrouter-2");
        assert_eq!(model.short_name, "o");
    }

    #[test]
    fn test_select_model_for_channel_flags_override_channel_default() {
        let model_list = setup_model_list();
        let result = model_list.select_model_for_channel(&vec!["d".to_string()], "openrouter-2");
        let model = result.expect("select_model_for_channel()");
        assert_eq!(model.id, "deepseek-1");
        assert_eq!(model.short_name, "d");
    }

    #[test]
    fn test_select_model_for_channel_fallback_to_global_default() {
        let model_list = setup_model_list();
        let result = model_list.select_model_for_channel(&vec![], "unknown-model");
        let model = result.expect("select_model_for_channel()");
        assert_eq!(model.id, "deepseek-1");
        assert_eq!(model.short_name, "d");
    }

    #[test]
    fn test_get_channel_default_model() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "DEEPSEEK_API_KEY=test-key\n").unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r##"
[general]
default_model = "global-default"

[providers.deepseek]
models = [
  { id = "global-default", short_name = "g", name = "Global Default" },
  { id = "test-model", short_name = "t", name = "Test Model" }
]

[channels]
"#test" = { default_model = "test-model" }
"##,
        )
        .unwrap();

        let env_vars = EnvVars {
            vars: HashMap::new(),
        };
        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");

        assert_eq!(config.get_channel_default_model("#test"), "test-model");
        assert_eq!(
            config.get_channel_default_model("#unknown"),
            "global-default"
        );
    }

    #[test]
    fn test_get_channel_temperature() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "DEEPSEEK_API_KEY=test-key\n").unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r##"
[general]
default_model = "default"

[providers.deepseek]
models = [
  { id = "default", short_name = "d", name = "Default" }
]

[channels]
"#test" = { temperature = 0.7 }
"#no-temp" = { default_model = "default" }
"##,
        )
        .unwrap();

        let env_vars = EnvVars {
            vars: HashMap::new(),
        };
        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");

        assert_eq!(config.get_channel_temperature("#test"), Some(0.7));
        assert_eq!(config.get_channel_temperature("#no-temp"), None);
        assert_eq!(config.get_channel_temperature("#unknown"), None);
    }

    #[test]
    fn test_get_channel_system_prompt() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path();
        let env_file = config_dir.join(".env");
        std::fs::write(&env_file, "DEEPSEEK_API_KEY=test-key\n").unwrap();

        let config_path = config_dir.join(CONFIG_FILE_NAME);
        std::fs::write(
            &config_path,
            r##"
[general]
default_model = "default"

[providers.deepseek]
models = [
  { id = "default", short_name = "d", name = "Default" }
]

[channels]
"#test" = { system_prompt = "Test prompt" }
"#no-prompt" = { default_model = "default" }
"##,
        )
        .unwrap();

        let env_vars = EnvVars {
            vars: HashMap::new(),
        };
        let config = Config::new(&config_dir, &env_vars).expect("Config::new()");

        assert_eq!(
            config.get_channel_system_prompt("#test"),
            Some("Test prompt")
        );
        assert_eq!(config.get_channel_system_prompt("#no-prompt"), None);
        assert_eq!(config.get_channel_system_prompt("#unknown"), None);
    }
}
