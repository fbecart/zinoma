use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct EnvVarsState(HashMap<String, Option<String>>);

impl EnvVarsState {
    pub fn current(var_names: &[String]) -> Result<Self> {
        let state = var_names
            .iter()
            .map(|var_name| match env::var(var_name) {
                Ok(value) => Ok((var_name.to_owned(), Some(value))),
                Err(env::VarError::NotPresent) => Ok((var_name.to_owned(), None)),
                Err(e) => {
                    Err(Error::new(e).context(format!("Failed to look up env var {}", var_name,)))
                }
            })
            .collect::<Result<_>>()?;

        Ok(Self(state))
    }

    pub fn eq_current_state(&self, var_names: &[String]) -> Result<bool> {
        Ok(&Self::current(var_names)? == self)
    }
}
