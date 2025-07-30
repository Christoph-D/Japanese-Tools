use gettextrs::gettext;
use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_while1},
    character::complete::multispace1,
    combinator::{all_consuming, map},
    multi::separated_list0,
    sequence::delimited,
};

use crate::formatget;

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
        Config {
            providers,
            default_model_id: std::env::var("DEFAULT_MODEL").unwrap_or_default(),
        }
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
                )?);
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

fn parse_model_id(i: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c != '[' && !c.is_whitespace())(i)
}
fn is_short_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}
fn is_human_name_char(c: char) -> bool {
    c != ')'
}
// Parses a single model from this syntax: <model ID>[<short name>](<human-readable name>)
fn parse_model<'a>(i: &'a str, api_key: &str, endpoint: &str) -> IResult<&'a str, Model> {
    map(
        (
            parse_model_id,
            delimited(tag("["), take_while1(is_short_name_char), tag("]")),
            delimited(tag("("), take_while1(is_human_name_char), tag(")")),
        ),
        |(id, short_name, name)| Model {
            id: id.to_string(),
            short_name: short_name.to_string(),
            name: name.to_string(),
            api_key: api_key.to_string(),
            endpoint: endpoint.to_string(),
        },
    )
    .parse(i)
}

// Parses a white-space separated list of models.
fn parse_models<'a>(i: &'a str, api_key: &str, endpoint: &str) -> IResult<&'a str, Vec<Model>> {
    all_consuming(separated_list0(multispace1, |i| {
        parse_model(i, api_key, endpoint)
    }))
    .parse(i)
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_parse_model_nom() {
        let input = "deepseek-1[short1](Deepseek 1)";
        let (rest, parsed) = parse_model(input, "key", "endpoint").unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            parsed,
            Model {
                id: "deepseek-1".to_string(),
                short_name: "short1".to_string(),
                name: "Deepseek 1".to_string(),
                api_key: "key".to_string(),
                endpoint: "endpoint".to_string()
            }
        );

        let input2 = "openrouter-2[o](OpenRouter 2)";
        let (_, parsed2) = parse_model(input2, "", "").unwrap();
        assert_eq!(parsed2.id, "openrouter-2");
        assert_eq!(parsed2.short_name, "o");
        assert_eq!(parsed2.name, "OpenRouter 2");
    }

    #[test]
    fn test_parse_model_nom_invalid() {
        assert!(parse_model("invalidmodel", "", "").is_err());
        assert!(parse_model("id[short](name", "", "").is_err());
        assert!(parse_model("idshort](name)", "", "").is_err());
    }

    #[test]
    fn test_parse_models() {
        let input = "deepseek-1[short1](Deepseek 1) openrouter-2[o](OpenRouter 2)";
        let (rest, parsed) = parse_models(input, "key", "endpoint").unwrap();
        assert_eq!(
            parsed,
            vec![
                Model {
                    id: "deepseek-1".to_string(),
                    short_name: "short1".to_string(),
                    name: "Deepseek 1".to_string(),
                    api_key: "key".to_string(),
                    endpoint: "endpoint".to_string()
                },
                Model {
                    id: "openrouter-2".to_string(),
                    short_name: "o".to_string(),
                    name: "OpenRouter 2".to_string(),
                    api_key: "key".to_string(),
                    endpoint: "endpoint".to_string()
                }
            ]
        );
        assert_eq!(rest, "");
    }
}

fn parse_model_config(
    models_list: &str,
    api_key: &str,
    endpoint: &str,
) -> Result<Vec<Model>, String> {
    match parse_models(models_list, api_key, endpoint) {
        Ok((_, models)) => Ok(models),
        Err(e) => Err(formatget!("Invalid model syntax: {}", e)),
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
                model_string: "deepseek-1[short1](Deepseek 1) deepseek-2[short2](Deepseek 2)"
                    .to_string(),
            }],
            default_model_id: "deepseek-1".to_string(),
        };
        let model_list = ModelList::new(&cfg).expect("new()");
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].id, "deepseek-1");
        assert_eq!(model_list.models[0].short_name, "short1");
        assert_eq!(model_list.models[0].name, "Deepseek 1");
        assert_eq!(model_list.models[0].api_key, "key1");
        assert_eq!(model_list.models[0].endpoint, DEEPSEEK_API_ENDPOINT);
        assert_eq!(model_list.models[1].id, "deepseek-2");
        assert_eq!(model_list.models[1].short_name, "short2");
        assert_eq!(model_list.models[1].name, "Deepseek 2");
        assert_eq!(model_list.models[1].api_key, "key1");
        assert_eq!(model_list.models[1].endpoint, DEEPSEEK_API_ENDPOINT);
        assert_eq!(model_list.default_model_name(), "Deepseek 1");
    }

    #[test]
    fn test_new_parses_openrouter_env_vars() {
        let cfg = Config {
            providers: vec![Provider {
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                model_string: "openrouter-1[o](OpenRouter 1) openrouter-2[p](OpenRouter 2)"
                    .to_string(),
            }],
            default_model_id: "openrouter-1".to_string(),
        };
        let model_list = ModelList::new(&cfg).expect("new()");
        assert_eq!(model_list.models.len(), 2);
        assert_eq!(model_list.models[0].id, "openrouter-1");
        assert_eq!(model_list.models[0].short_name, "o".to_string());
    }

    #[test]
    fn test_new_parses_missing_short_name() {
        let cfg = Config {
            providers: vec![Provider {
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                model_string: "openrouter-1[o](OpenRouter-1) openrouter-2".to_string(),
            }],
            default_model_id: "openrouter-1".to_string(),
        };
        let err = ModelList::new(&cfg).unwrap_err();
        assert!(err.contains("openrouter-2"), "{}", err);
    }

    #[test]
    fn test_new_unknown_default_model_fails() {
        let cfg = Config {
            providers: vec![Provider {
                name: "OpenRouter".to_string(),
                api_key: "key2".to_string(),
                endpoint: OPENROUTER_API_ENDPOINT.to_string(),
                model_string: "openrouter-1[o](OpenRouter-1) openrouter-2[p](OpenRouter-2)"
                    .to_string(),
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
    fn test_select_model_with_flags_containing_empty_and_valid_model_names() {
        let model_list = setup_model_list();
        let flags = vec!["".to_string(), "clear_history".to_string(), "o".to_string()];
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
            vec!["OpenRouter: -o"]
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
