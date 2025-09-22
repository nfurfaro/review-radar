use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub orgs: Vec<String>,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub html_url: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct GhRepo {
    pub name: String,
    #[serde(skip)]
    pub org: String,
}

#[derive(Debug, Deserialize)]
pub struct GhPullRequest {
    pub number: u32,
    pub title: String,
    pub url: String,
    pub author: GhUser,
    #[serde(rename = "reviewRequests")]
    pub review_requests: Vec<GhUser>,
}

#[derive(Debug, Deserialize)]
pub struct GhUser {
    pub login: String,
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        Ok(config_dir.join("review-radar").join("config.toml"))
    }

    pub fn config_path_in_dir(dir: &PathBuf) -> PathBuf {
        dir.join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Configuration not found. Run 'review-radar init <orgs> <username>' to set up."
            ));
        }
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn add_org(&mut self, org: String) -> bool {
        if !self.orgs.contains(&org) {
            self.orgs.push(org);
            true
        } else {
            false
        }
    }

    pub fn remove_org(&mut self, org: &str) -> bool {
        if let Some(pos) = self.orgs.iter().position(|x| x == org) {
            self.orgs.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn set_orgs(&mut self, orgs: Vec<String>) {
        self.orgs = orgs;
    }

    pub fn set_repo_pattern(&mut self, pattern: Option<String>) -> Result<()> {
        if let Some(ref p) = pattern {
            if p.to_lowercase() == "none" {
                self.repo_pattern = None;
            } else {
                // Validate the regex
                Regex::new(p)
                    .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", p, e))?;
                self.repo_pattern = pattern;
            }
        } else {
            self.repo_pattern = pattern;
        }
        Ok(())
    }
}

pub fn parse_org_modification(org_str: &str) -> OrgModification {
    if let Some(stripped) = org_str.strip_prefix('+') {
        OrgModification::Add(stripped.trim().to_string())
    } else if let Some(stripped) = org_str.strip_prefix('-') {
        OrgModification::Remove(stripped.trim().to_string())
    } else {
        let orgs: Vec<String> = org_str.split(',').map(|s| s.trim().to_string()).collect();
        OrgModification::Replace(orgs)
    }
}

#[derive(Debug, PartialEq)]
pub enum OrgModification {
    Add(String),
    Remove(String),
    Replace(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_creation() {
        let config = Config {
            orgs: vec!["org1".to_string(), "org2".to_string()],
            username: "testuser".to_string(),
            repo_pattern: Some("test-.*".to_string()),
        };

        assert_eq!(config.orgs.len(), 2);
        assert_eq!(config.username, "testuser");
        assert_eq!(config.repo_pattern, Some("test-.*".to_string()));
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            orgs: vec!["org1".to_string()],
            username: "testuser".to_string(),
            repo_pattern: None,
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = Config::config_path_in_dir(&temp_dir.path().to_path_buf());

        let config = Config {
            orgs: vec!["test-org".to_string()],
            username: "testuser".to_string(),
            repo_pattern: Some("backend-.*".to_string()),
        };

        // Save config
        config.save_to_path(&config_path).unwrap();

        // Load config
        let loaded_config = Config::load_from_path(&config_path).unwrap();

        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_config_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");

        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_org() {
        let mut config = Config {
            orgs: vec!["org1".to_string()],
            username: "testuser".to_string(),
            repo_pattern: None,
        };

        // Add new org
        assert!(config.add_org("org2".to_string()));
        assert_eq!(config.orgs.len(), 2);
        assert!(config.orgs.contains(&"org2".to_string()));

        // Try to add existing org
        assert!(!config.add_org("org1".to_string()));
        assert_eq!(config.orgs.len(), 2);
    }

    #[test]
    fn test_remove_org() {
        let mut config = Config {
            orgs: vec!["org1".to_string(), "org2".to_string()],
            username: "testuser".to_string(),
            repo_pattern: None,
        };

        // Remove existing org
        assert!(config.remove_org("org1"));
        assert_eq!(config.orgs.len(), 1);
        assert!(!config.orgs.contains(&"org1".to_string()));

        // Try to remove non-existent org
        assert!(!config.remove_org("org3"));
        assert_eq!(config.orgs.len(), 1);
    }

    #[test]
    fn test_set_orgs() {
        let mut config = Config {
            orgs: vec!["org1".to_string()],
            username: "testuser".to_string(),
            repo_pattern: None,
        };

        let new_orgs = vec!["new1".to_string(), "new2".to_string(), "new3".to_string()];
        config.set_orgs(new_orgs.clone());

        assert_eq!(config.orgs, new_orgs);
    }

    #[test]
    fn test_set_repo_pattern() {
        let mut config = Config {
            orgs: vec!["org1".to_string()],
            username: "testuser".to_string(),
            repo_pattern: None,
        };

        // Set valid pattern
        config
            .set_repo_pattern(Some("test-.*".to_string()))
            .unwrap();
        assert_eq!(config.repo_pattern, Some("test-.*".to_string()));

        // Clear pattern with "none"
        config.set_repo_pattern(Some("none".to_string())).unwrap();
        assert_eq!(config.repo_pattern, None);

        // Set invalid regex pattern
        let result = config.set_repo_pattern(Some("[invalid".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_org_modification() {
        // Test add
        let result = parse_org_modification("+new-org");
        assert_eq!(result, OrgModification::Add("new-org".to_string()));

        // Test remove
        let result = parse_org_modification("-old-org");
        assert_eq!(result, OrgModification::Remove("old-org".to_string()));

        // Test replace
        let result = parse_org_modification("org1,org2,org3");
        assert_eq!(
            result,
            OrgModification::Replace(vec![
                "org1".to_string(),
                "org2".to_string(),
                "org3".to_string()
            ])
        );

        // Test replace with single org
        let result = parse_org_modification("single-org");
        assert_eq!(
            result,
            OrgModification::Replace(vec!["single-org".to_string()])
        );
    }
}
