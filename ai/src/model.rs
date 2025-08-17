use gettextrs::gettext;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    name: String,
    api_key: String,
    endpoint: String,
    models: Vec<TomlModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    providers: Vec<Provider>,
    default_model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub id: String,
    pub short_name: String,
    pub name: String,
    pub api_key: String,
    pub endpoint: String,
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

#[derive(Debug, serde::Deserialize)]
struct TomlConfig {
    general: TomlGeneral,
    providers: HashMap<String, TomlProvider>,
}

#[derive(Debug, serde::Deserialize)]
struct TomlGeneral {
    default_model: String,
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
}

impl Config {
    pub fn new(config_path: &std::path::Path) -> Result<Self, String> {
        let toml_path = config_path.join("config.toml");
        let toml_content = std::fs::read_to_string(toml_path)
            .map_err(|e| format!("Failed to read config.toml: {}", e))?;
        let toml_config: TomlConfig = toml::from_str(&toml_content)
            .map_err(|e| format!("Failed to parse config.toml: {}", e))?;

        let mut providers = Vec::new();
        for (provider_name, toml_provider) in toml_config.providers {
            let env_prefix = provider_name.to_uppercase();
            match provider_name.as_str() {
                "litellm" => {
                    if toml_provider
                        .endpoint
                        .as_deref()
                        .unwrap_or_default()
                        .is_empty()
                    {
                        return Err("LiteLLM provider requires an endpoint.".to_string());
                    }
                }
                "anthropic" | "deepseek" | "mistral" | "openrouter" => {
                    if toml_provider.endpoint.is_some() {
                        return Err(format!(
                            "Provider '{}' endpoint is not configurable.",
                            provider_name
                        ));
                    }
                }
                _ => return Err(format!("Unknown provider: {}", provider_name)),
            }

            // API key comes from an environment variable
            if let Ok(api_key) = std::env::var(format!("{}_API_KEY", env_prefix)) {
                if !api_key.is_empty() {
                    let endpoint = match provider_name.as_str() {
                        "anthropic" => ANTHROPIC_API_ENDPOINT.to_string(),
                        "deepseek" => DEEPSEEK_API_ENDPOINT.to_string(),
                        "mistral" => MISTRAL_API_ENDPOINT.to_string(),
                        "openrouter" => OPENROUTER_API_ENDPOINT.to_string(),
                        "litellm" => toml_provider.endpoint.unwrap(), // we validated above
                        _ => unreachable!(),                          // validated above
                    };
                    providers.push(Provider {
                        name: Self::provider_display_name(&provider_name),
                        api_key,
                        endpoint,
                        models: toml_provider.models,
                    });
                }
            }
        }

        Ok(Config {
            providers,
            default_model_id: toml_config.general.default_model,
        })
    }

    fn provider_display_name(provider_name: &str) -> String {
        match provider_name {
            "anthropic" => "Anthropic".to_string(),
            "deepseek" => "Deepseek".to_string(),
            "litellm" => "LiteLLM".to_string(),
            "mistral" => "Mistral".to_string(),
            "openrouter" => "OpenRouter".to_string(),
            _ => provider_name.to_string(),
        }
    }
}

impl ModelList {
    pub fn new(cfg: &Config) -> Result<Self, String> {
        let mut models = Vec::new();
        for provider in &cfg.providers {
            for toml_model in &provider.models {
                models.push(Model {
                    id: toml_model.id.clone(),
                    short_name: toml_model.short_name.clone(),
                    name: toml_model.name.clone(),
                    api_key: provider.api_key.clone(),
                    endpoint: provider.endpoint.clone(),
                });
            }
        }
        if models.is_empty() {
            return Err(gettext("Missing API keys or model configuration"));
        }
        let default_model_index = models
            .iter()
            .position(|m| m.id == cfg.default_model_id)
            .ok_or_else(|| gettext("DEFAULT_MODEL not found"))?;
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

    // Selects a model based on the query.
    // If the flags contains a model name, it selects the last specified model.
    // Otherwise, it returns the default model.
    pub fn select_model(&self, flags: &[String]) -> Result<&Model, String> {
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
        Ok(self.default_model())
    }

    pub fn list_model_flags_human_readable(&self) -> Vec<String> {
        let d = &self.default_model().id;
        self.models
            .iter()
            .filter(|m| &m.id != d)
            .map(|m| format!("[{}]{}", m.short_name, m.name))
            .collect::<Vec<String>>()
    }

    pub fn list_model_flags_without_default(&self) -> Vec<String> {
        let d = &self.default_model().id;
        self.models
            .iter()
            .filter(|m| &m.id != d)
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
            },
            Model {
                id: "openrouter-2".to_string(),
                short_name: "o".to_string(),
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
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
        let cfg = Config {
            providers: vec![Provider {
                name: "Deepseek".to_string(),
                api_key: "key1".to_string(),
                endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
                models: vec![
                    TomlModel {
                        id: "deepseek-1".to_string(),
                        short_name: "short1".to_string(),
                        name: "Deepseek 1".to_string(),
                    },
                    TomlModel {
                        id: "deepseek-2".to_string(),
                        short_name: "short2".to_string(),
                        name: "Deepseek 2".to_string(),
                    },
                ],
            }],
            default_model_id: "deepseek-1".to_string(),
        };
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
                },
                Model {
                    id: "deepseek-2".to_string(),
                    short_name: "short2".to_string(),
                    name: "Deepseek 2".to_string(),
                    api_key: "key1".to_string(),
                    endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
                }
            ]
        );
        assert_eq!(model_list.default_model_name(), "Deepseek 1");
    }

    #[test]
    fn test_new_parses_openrouter() {
        let cfg = Config {
            providers: vec![Provider {
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                models: vec![
                    TomlModel {
                        id: "openrouter-1".to_string(),
                        short_name: "o".to_string(),
                        name: "OpenRouter 1".to_string(),
                    },
                    TomlModel {
                        id: "openrouter-2".to_string(),
                        short_name: "p".to_string(),
                        name: "OpenRouter 2".to_string(),
                    },
                ],
            }],
            default_model_id: "openrouter-1".to_string(),
        };
        let model_list = ModelList::new(&cfg).expect("new()");
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].id, "openrouter-1");
        assert_eq!(model_list.models[0].short_name, "o".to_string());
    }

    #[test]
    fn test_new_unknown_default_model_fails() {
        let cfg = Config {
            providers: vec![Provider {
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                models: vec![
                    TomlModel {
                        id: "openrouter-1".to_string(),
                        short_name: "o".to_string(),
                        name: "OpenRouter-1".to_string(),
                    },
                    TomlModel {
                        id: "openrouter-2".to_string(),
                        short_name: "p".to_string(),
                        name: "OpenRouter-2".to_string(),
                    },
                ],
            }],
            default_model_id: "unknown_model".to_string(),
        };
        let err = ModelList::new(&cfg).unwrap_err();
        assert!(err.contains("DEFAULT_MODEL"), "{}", err);
    }

    #[test]
    fn test_select_model_with_empty_query() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec![]);
        let model = result.expect("select_model()");
        assert_eq!(model.id, "deepseek-1");
    }

    #[test]
    fn test_select_model_default() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["d".to_string()]);
        let model = result.expect("select_model()");
        assert_eq!(model.id, "deepseek-1");
        assert_eq!(model.short_name, "d");
    }

    #[test]
    fn test_select_model_with_unknown_model() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["unknownmodel".to_string()]);
        let model = result.expect("select_model()");
        assert_eq!(model.id, "deepseek-1");
        assert_eq!(model.short_name, "d");
    }

    #[test]
    fn test_select_model() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["o".to_string()]);
        let model = result.expect("select_model()");
        assert_eq!(model.id, "openrouter-2");
        assert_eq!(model.short_name, "o");
    }

    #[test]
    fn test_select_model_with_empty_flag() {
        let model_list = setup_model_list();
        let flags = vec!["".to_string(), "o".to_string()];
        let result = model_list.select_model(&flags);
        let model = result.expect("select_model()");
        assert_eq!(model.id, "openrouter-2");
    }

    #[test]
    fn test_select_model_with_flags_containing_multiple_model_names() {
        let model_list = setup_model_list();
        let flags = vec!["d".to_string(), "o".to_string()];
        let result = model_list.select_model(&flags);
        let model = result.expect("select_model()");
        assert_eq!(model.id, "openrouter-2");
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
            model_list.list_model_flags_human_readable(),
            vec!["[o]OpenRouter"]
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
        }];
        let model_list = ModelList {
            models,
            default_model_index: 0,
        };
        assert_eq!(model_list.list_model_flags(), vec!["o"]);
    }
}
