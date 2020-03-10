#![deny(clippy::all)]

use std::convert::TryFrom;

use thiserror::Error as ThisError;

use super::{
    facts::Facts,
    jobs::{self, Main},
};

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Job {
        #[from]
        source: jobs::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn render<S>(input: S, _facts: &Facts) -> Result<String>
where
    S: AsRef<str>,
{
    Main::try_from(input.as_ref())?; // check that we have valid TOML first
    Ok(input.as_ref().to_string())
}

#[cfg(test)]
mod tests {

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
    fn render_toml_without_expressions() {
        let input = r#"
            [[jobs]]
            name = "run something"
            type = "command"
            command = "something"
            argv = [ "foo" ]
            "#;
        let facts = Facts::default();
        let want = String::from(input);
        let result = render(input, &facts);
        assert!(result.is_ok());
        if let Ok(got) = result {
            assert_eq!(got, want);
        }
    }
}
