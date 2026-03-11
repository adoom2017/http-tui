use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Environment {
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

impl Environment {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Environment::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read env file: {}", path.display()))?;
        let env: Environment = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse env file: {}", path.display()))?;
        Ok(env)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content =
            serde_yaml::to_string(self).context("Failed to serialize environment")?;
        std::fs::write(path, content).context("Failed to write env file")?;
        Ok(())
    }

    /// Replace all `{{KEY}}` occurrences in `text` with the corresponding variable value.
    /// Unknown variables are left as-is.
    pub fn substitute(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }

    /// Return a sorted list of (key, value) pairs for display.
    #[allow(dead_code)]
    pub fn sorted_vars(&self) -> Vec<(&String, &String)> {
        let mut pairs: Vec<(&String, &String)> = self.variables.iter().collect();
        pairs.sort_by_key(|(k, _)| k.as_str());
        pairs
    }
}
