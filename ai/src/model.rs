use gettextrs::gettext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    deepseek_models: Option<String>,
    deepseek_api_key: Option<String>,
    openrouter_models: Option<String>,
    openrouter_api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
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
const OPENROUTER_API_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

impl Config {
    pub fn from_env() -> Self {
        Config {
            deepseek_models: std::env::var("DEEPSEEK_MODELS").ok(),
            deepseek_api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
            openrouter_models: std::env::var("OPENROUTER_MODELS").ok(),
            openrouter_api_key: std::env::var("OPENROUTER_API_KEY").ok(),
        }
    }
}

impl ModelList {
    pub fn new(cfg: &Config) -> Result<Self, String> {
        let mut models = Vec::new();
        if let (Some(models_var), Some(api_key_var)) = (&cfg.deepseek_models, &cfg.deepseek_api_key)
        {
            models.extend(parse_model_config(
                models_var,
                api_key_var,
                DEEPSEEK_API_ENDPOINT,
            ));
        }
        if let (Some(models_var), Some(api_key_var)) =
            (&cfg.openrouter_models, &cfg.openrouter_api_key)
        {
            models.extend(parse_model_config(
                models_var,
                api_key_var,
                OPENROUTER_API_ENDPOINT,
            ));
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
            if let Some(model) = self.models.iter().find(|m| m.name == *f) {
                selected = Some(model);
            }
        }
        if let Some(model) = selected {
            return Ok(model);
        }
        Ok(self.default_model())
    }

    pub fn list_models(&self) -> Vec<&str> {
        self.models
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<&str>>()
    }
}

fn parse_model_config(models_list: &str, api_key: &str, endpoint: &str) -> Vec<Model> {
    let models = models_list
        .split(' ')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    models
        .into_iter()
        .map(|name| Model {
            name,
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
                api_key: "key1".to_string(),
                endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
            },
            Model {
                name: "openrouter-2".to_string(),
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
            deepseek_models: None,
            deepseek_api_key: None,
            openrouter_models: None,
            openrouter_api_key: None,
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
            deepseek_models: Some("deepseek-1 deepseek-2".to_string()),
            deepseek_api_key: Some("key1".to_string()),
            openrouter_models: None,
            openrouter_api_key: None,
        };
        let result = ModelList::new(&cfg);
        assert!(result.is_ok());
        let model_list = result.unwrap();
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].name, "deepseek-1");
        assert_eq!(model_list.models[0].api_key, "key1");
        assert_eq!(model_list.models[0].endpoint, DEEPSEEK_API_ENDPOINT);
        assert_eq!(model_list.models[1].name, "deepseek-2");
        assert_eq!(model_list.models[1].api_key, "key1");
        assert_eq!(model_list.models[1].endpoint, DEEPSEEK_API_ENDPOINT);
    }

    #[test]
    fn test_new_parses_openrouter_env_vars() {
        let cfg = Config {
            deepseek_models: None,
            deepseek_api_key: None,
            openrouter_models: Some("openrouter-1 openrouter-2".to_string()),
            openrouter_api_key: Some("key2".to_string()),
        };
        let result = ModelList::new(&cfg);
        assert!(result.is_ok());
        let model_list = result.unwrap();
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].name, "openrouter-1");
        assert_eq!(model_list.models[0].api_key, "key2");
        assert_eq!(model_list.models[0].endpoint, OPENROUTER_API_ENDPOINT);
        assert_eq!(model_list.models[1].name, "openrouter-2");
        assert_eq!(model_list.models[1].api_key, "key2");
        assert_eq!(model_list.models[1].endpoint, OPENROUTER_API_ENDPOINT);
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
    }

    #[test]
    fn test_select_model_with_model_prefix() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["openrouter-2".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
    }

    #[test]
    fn test_select_model_with_unknown_model() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["unknownmodel".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "deepseek-1");
    }

    #[test]
    fn test_select_model_with_only_model_prefix() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["openrouter-2".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
    }

    #[test]
    fn test_select_model_with_leading_and_trailing_spaces() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["openrouter-2".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
    }

    #[test]
    fn test_select_model_with_no_query_after_model() {
        let model_list = setup_model_list();
        let result = model_list.select_model(&vec!["deepseek-1".to_string()]);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "deepseek-1");
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
        let flags = vec![
            "".to_string(),
            "deepseek-1".to_string(),
            "openrouter-2".to_string(),
        ];
        let result = model_list.select_model(&flags);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "openrouter-2");
    }

    #[test]
    fn test_list_models_returns_all_model_names() {
        let model_list = setup_model_list();
        assert_eq!(model_list.list_models(), vec!["deepseek-1", "openrouter-2"]);
    }

    #[test]
    fn test_list_models_with_single_model() {
        let models = vec![Model {
            name: "only-model".to_string(),
            api_key: "key".to_string(),
            endpoint: DEEPSEEK_API_ENDPOINT.to_string(),
        }];
        let model_list = ModelList {
            models,
            default_model_index: 0,
        };
        assert_eq!(model_list.list_models(), vec!["only-model"]);
    }

    #[test]
    fn test_list_models_empty() {
        let model_list = ModelList {
            models: vec![],
            default_model_index: 0,
        };
        let models = model_list.list_models();
        assert_eq!(models, Vec::<&str>::new());
    }
}
