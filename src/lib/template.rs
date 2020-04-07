#![deny(clippy::all)]

use std::convert::TryFrom;

use lazy_static::lazy_static;
use regex::Regex;
use tera::{self, Context, Tera};
use thiserror::Error as ThisError;

use super::{
    facts::Facts,
    jobs::{self, Main},
};

lazy_static! {
    static ref DIR_EXPRESSION_RE: Regex = Regex::new(r"_dir\s*\}\}").unwrap();
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("unable to prepare template context: {}", source)]
    Context { source: tera::Error },
    #[error(transparent)]
    Job {
        #[from]
        source: jobs::Error,
    },
    #[error("unable to render template: {}", source)]
    Render { source: tera::Error },
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn render<S>(input: S, facts: &Facts) -> Result<String>
where
    S: AsRef<str>,
{
    Main::try_from(input.as_ref())?; // check that we have valid TOML first

    let context = Context::from_serialize(facts).map_err(|e| Error::Context { source: e })?;
    Tera::one_off(
        &DIR_EXPRESSION_RE.replace_all(input.as_ref(), "_dir | addslashes }}"),
        &context,
        false,
    )
    .map_err(|e| Error::Render { source: e })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::facts::Facts;

    use super::*;

    #[test]
    fn render_errs_if_not_toml() {
        let input = r#"{"hello": "world"}"#;
        let facts = Facts::default();
        let got = render(input, &facts);
        assert!(got.is_err());
        // TODO: assert on error contents
    }

    #[test]
    fn render_errs_if_bad_toml() {
        let input = r#"unexpected_key = "value""#;
        let facts = Facts::default();
        let got = render(input, &facts);
        assert!(got.is_err());
        // TODO: assert on error contents
    }

    #[test]
    fn render_toml_with_missing_value() {
        let input = r#"
            [[jobs]]
            type = "command"
            command = "{{ missing_value }}"
            "#;
        let facts = Facts::default();
        let got = render(input, &facts);
        assert!(got.is_err());
        // TODO: assert on error contents
    }

    #[test]
    fn render_toml_without_expressions() {
        let input = r#"
            [[jobs]]
            type = "command"
            command = "something"
            "#;
        let facts = Facts::default();
        let want = String::from(input);
        let result = render(input, &facts);
        assert!(result.is_ok());
        if let Ok(got) = result {
            assert_eq!(got, want);
        }
    }

    #[test]
    fn render_toml_with_expressions() {
        let input = r#"
            [[jobs]]
            name = "{{ cache_dir }} {{ home_dir }}"
            type = "command"
            command = "{{ config_dir }}"
            "#;
        let facts = Facts {
            cache_dir: PathBuf::from("c:\\my_cache_dir"), // like Windows
            config_dir: PathBuf::from("my_config_dir"),
            home_dir: PathBuf::from("my_home_dir"),
        };
        let want = r#"
            [[jobs]]
            name = "c:\\my_cache_dir my_home_dir"
            type = "command"
            command = "my_config_dir"
            "#;
        let result = render(input, &facts);
        assert!(result.is_ok());
        if let Ok(got) = result {
            assert_eq!(got, want);
        }
    }
}
