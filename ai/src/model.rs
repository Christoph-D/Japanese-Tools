use gettextrs::gettext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    name: String,
    api_key: String,
    endpoint: String,
    model_string: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    providers: Vec<Provider>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub short_name: Option<String>,
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

impl Config {
    pub fn from_env() -> Self {
        let mut providers = Vec::new();
        let provider_configs = [
            ("DEEPSEEK", "Deepseek", DEEPSEEK_API_ENDPOINT),
            ("MISTRAL", "Mistral", MISTRAL_API_ENDPOINT),
            ("OPENROUTER", "OpenRouter", OPENROUTER_API_ENDPOINT),
        ];
        for (env_prefix, name, endpoint) in provider_configs.iter() {
            if let Ok(api_key) = std::env::var(format!("{}_API_KEY", env_prefix)) {
                let model_string =
                    std::env::var(format!("{}_MODELS", env_prefix)).unwrap_or_default();
                providers.push(Provider {
                    name: name.to_string(),
                    api_key,
                    endpoint: endpoint.to_string(),
                    model_string,
                });
            }
        }
        Config { providers }
    }
}

impl ModelList {
    pub fn new(cfg: &Config) -> Result<Self, String> {
        let mut models = Vec::new();
        for provider in &cfg.providers {
            if !provider.model_string.is_empty() {
                models.extend(parse_model_config(
                    &provider.model_string,
                    &provider.api_key,
                    &provider.endpoint,
                ));
            }
        }
        if models.is_empty() {
            return Err(gettext("Missing API keys or model configuration"));
        }
        Ok(ModelList {
            models,
            default_model_index: 0,
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
            if let Some(model) = self
                .models
                .iter()
                .find(|m| m.name == *f || m.short_name.as_deref() == Some(f))
            {
                selected = Some(model);
            }
        }
        if let Some(model) = selected {
            return Ok(model);
        }
        Ok(self.default_model())
    }

    pub fn list_model_flags_human_readable(&self) -> Vec<String> {
        let default_name = &self.default_model().name;
        self.models
            .iter()
            .filter(|m| &m.name != default_name)
            .map(|m| {
                if let Some(short_name) = &m.short_name {
                    format!("-{}|-{}", m.name, short_name)
                } else {
                    format!("-{}", m.name)
                }
            })
            .collect::<Vec<String>>()
    }

    pub fn list_model_flags(&self) -> Vec<String> {
        self.models
            .iter()
            .flat_map(|m| [Some(m.name.clone()), m.short_name.clone()])
            .flatten()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
    }
}

fn parse_model_config(models_list: &str, api_key: &str, endpoint: &str) -> Vec<Model> {
    let model_re = regex::Regex::new(r"([^(]*)\(([^)]*)\)").unwrap();
    let models = models_list
        .split(' ')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| match model_re.captures(s) {
            Some(cap) => {
                if cap.len() == 2 {
                    (cap[1].to_string(), "".to_string())
                } else if cap.len() == 3 {
                    (cap[1].to_string(), cap[2].to_string())
                } else {
                    (s.to_string(), "".to_string())
                }
            }
            None => (s.to_string(), "".to_string()),
        })
        .map(|(name, short)| (name, if short.is_empty() { None } else { Some(short) }))
        .collect::<Vec<_>>();
    models
        .into_iter()
        .map(|(name, short_name)| Model {
            name,
            short_name,
            api_key: api_key.to_string(),
            endpoint: endpoint.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_model_list() -> ModelList {
        let models = vec![
            Model {
                name: "deepseek-1".to_string(),
                short_name: Some("d".to_string()),
                api_key: "key1".to_string(),
                endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
            },
            Model {
                name: "openrouter-2".to_string(),
                short_name: Some("o".to_string()),
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
        let cfg = Config { providers: vec![] };
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
                model_string: "deepseek-1(d) deepseek-2(e)".to_string(),
            }],
        };
        let result = ModelList::new(&cfg);
        assert!(result.is_ok());
        let model_list = result.unwrap();
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].name, "deepseek-1");
        assert_eq!(model_list.models[0].short_name, Some("d".to_string()));
        assert_eq!(model_list.models[0].api_key, "key1");
        assert_eq!(model_list.models[0].endpoint, DEEPSEEK_API_ENDPOINT);
        assert_eq!(model_list.models[1].name, "deepseek-2");
        assert_eq!(model_list.models[1].short_name, Some("e".to_string()));
        assert_eq!(model_list.models[1].api_key, "key1");
        assert_eq!(model_list.models[1].endpoint, DEEPSEEK_API_ENDPOINT);
    }

    #[test]
    fn test_new_parses_openrouter_env_vars() {
        let cfg = Config {
            providers: vec![Provider {
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                model_string: "openrouter-1(o) openrouter-2(p)".to_string(),
            }],
        };
        let result = ModelList::new(&cfg);
        assert!(result.is_ok());
        let model_list = result.unwrap();
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].name, "openrouter-1");
        assert_eq!(model_list.models[0].short_name, Some("o".to_string()));
    }

    #[test]
    fn test_new_parses_missing_short_name() {
        let cfg = Config {
            providers: vec![Provider {
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                model_string: "openrouter-1(o) openrouter-2".to_string(),
            }],
        };
        let result = ModelList::new(&cfg);
        assert!(result.is_ok());
        let model_list = result.unwrap();
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].name, "openrouter-1");
        assert_eq!(model_list.models[0].short_name, Some("o".to_string()));
        assert_eq!(model_list.models[1].name, "openrouter-2");
        assert_eq!(model_list.models[1].short_name, None);
    }

    #[test]
    fn test_select_model_with_empty_query() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec![]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "deepseek-1");
    }

    #[test]
    fn test_select_model_default() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["deepseek-1".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "deepseek-1");
        assert_eq!(model.short_name, Some("d".to_string()));
    }

    #[test]
    fn test_select_model_with_unknown_model() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["unknownmodel".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "deepseek-1");
        assert_eq!(model.short_name, Some("d".to_string()));
    }

    #[test]
    fn test_select_model() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["openrouter-2".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
        assert_eq!(model.short_name, Some("o".to_string()));
    }

    #[test]
    fn test_select_model_short_name() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["o".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
        assert_eq!(model.short_name, Some("o".to_string()));
    }

    #[test]
    fn test_select_model_with_flags_containing_empty_and_valid_model_names() {
        let model_list = setup_model_list();
        let flags = vec![
            "".to_string(),
            "clear_history".to_string(),
            "openrouter-2".to_string(),
        ];
        let result = model_list.select_model(&flags);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
    }

    #[test]
    fn test_select_model_with_flags_containing_multiple_model_names() {
        let model_list = setup_model_list();
        let flags = vec!["deepseek-1".to_string(), "openrouter-2".to_string()];
        let result = model_list.select_model(&flags);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
    }

    #[test]
    fn test_list_models_returns_all_model_names() {
        let model_list = setup_model_list();
        assert_eq!(
            model_list.list_model_flags(),
            vec!["deepseek-1", "d", "openrouter-2", "o"]
        );
    }

    #[test]
    fn test_list_models_human_readable_excludes_default() {
        let model_list = setup_model_list();
        assert_eq!(
            model_list.list_model_flags_human_readable(),
            vec!["-openrouter-2|-o"]
        );
    }

    #[test]
    fn test_list_models_with_single_model() {
        let models = vec![Model {
            name: "only-model".to_string(),
            short_name: Some("o".to_string()),
            api_key: "key".to_string(),
            endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
        }];
        let model_list = ModelList {
            models,
            default_model_index: 0,
        };
        assert_eq!(model_list.list_model_flags(), vec!["only-model", "o"]);
    }

    #[test]
    fn test_list_models_empty() {
        let model_list = ModelList {
            models: vec![],
            default_model_index: 0,
        };
        let models = model_list.list_model_flags();
        assert_eq!(models, Vec::<&str>::new());
    }
}
