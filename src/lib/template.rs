use std::{collections::HashMap, convert::TryFrom};

use lazy_static::lazy_static;
use regex::Regex;
use tera::{self, from_value, to_value, Context, Tera, Value};
use thiserror::Error as ThisError;
use which::which;

use super::{
    facts::Facts,
    jobs::{self, Main},
};

lazy_static! {
    static ref DIR_EXPRESSION_RE: Regex = Regex::new(r"_dir\s*\}\}").unwrap();
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Job {
        #[from]
        source: jobs::Error,
    },
    #[error("template error: {}", source)]
    Tera {
        #[from]
        source: tera::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn render<S>(input: S, facts: &Facts) -> Result<String>
where
    S: AsRef<str>,
{
    let context = Context::from_serialize(facts)?;

    let mut t = Tera::new("template/**/*").expect("unable to prepare template system");
    t.add_raw_template(
        "main.toml",
        &DIR_EXPRESSION_RE.replace_all(input.as_ref(), "_dir | addslashes }}"),
    )?;
    t.register_function("has_executable", template_function_has_executable);

    let output = t.render("main.toml", &context)?;

    Main::try_from(output.as_str())?; // check that we have valid TOML first

    Ok(output)
}

fn template_function_has_executable(args: &HashMap<String, Value>) -> tera::Result<Value> {
    match args.get("exe") {
        Some(val) => match from_value::<String>(val.clone()) {
            Ok(v) => Ok(to_value(which(v).is_ok()).unwrap()),
            Err(_) => Err(tera::Error::from(r#""exe" must be a string"#)),
        },
        None => Err(tera::Error::from(r#"missing "exe" argument"#)),
    }
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
            when = {{ is_os_linux or is_os_macos }}
            "#;
        let facts = Facts {
            cache_dir: PathBuf::from("c:\\my_cache_dir"), // like Windows
            config_dir: PathBuf::from("my_config_dir"),
            home_dir: PathBuf::from("my_home_dir"),
            is_os_linux: false,
            is_os_macos: false,
            ..Default::default()
        };
        let want = r#"
            [[jobs]]
            name = "c:\\my_cache_dir my_home_dir"
            type = "command"
            command = "my_config_dir"
            when = false
            "#;
        let result = dbg!(render(input, &facts));
        assert!(result.is_ok());
        if let Ok(got) = result {
            assert_eq!(got, want);
        }
    }

    #[test]
    fn render_toml_with_function_expressions() {
        let input = r#"
            [[jobs]]
            name = "{{ has_executable(exe="missing_command") }}"
            type = "command"
            command = "foo"
            "#;
        let facts = Facts::default();
        let want = r#"
            [[jobs]]
            name = "false"
            type = "command"
            command = "foo"
            "#;
        let result = dbg!(render(input, &facts));
        assert!(result.is_ok());
        if let Ok(got) = result {
            assert_eq!(got, want);
        }
    }
}
