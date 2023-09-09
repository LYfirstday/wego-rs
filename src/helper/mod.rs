use serde::{Deserialize, Serialize};

use crate::{TemplateType, API_PREFIX};

pub mod utils;
#[derive(Clone, Debug)]
pub struct ConfigFile {
  pub github_name: String,
  pub repo_name: String,
  pub github_api_token: String,
  pub target_branch: String,
  pub templates_source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalConfigFile {
  pub github_name: String,
  pub repo_name: String,
  pub github_api_token: Option<String>,
  pub templates_source: Option<String>,
  pub target_branch: Option<String>,
}

impl ConfigFile {
  pub fn new(
    github_name: String,
    repo_name: String,
    github_api_token: String,
    target_branch: String,
    templates_source: String,
  ) -> Self {
    ConfigFile {
      github_name,
      repo_name,
      github_api_token,
      target_branch,
      templates_source,
    }
  }

  pub fn get_remote_yaml_url(&self) -> String {
    let api = String::clone(&API_PREFIX);
    // let target_branch = self.target_branch.clone().unwrap();

    let strs: Vec<String> = vec![
      api,
      String::clone(&self.github_name),
      String::clone(&self.repo_name),
      String::from("contents"),
      String::from("wego.yaml"), // format!("wego.yaml?ref={}", target_branch),
    ];

    strs.join("/")
  }

  pub fn get_remote_template_url(&self, file_type: TemplateType, file_name: &String) -> String {
    let file_type_str: String;

    match file_type {
      TemplateType::Pages => file_type_str = String::from("pages"),
      TemplateType::Components => file_type_str = String::from("components"),
      TemplateType::Project => file_type_str = String::from("projects"),
    }

    let url = format!(
      "{}/{}/{}/contents/{}/{}/{}",
      &API_PREFIX.to_string(),
      self.github_name,
      self.repo_name,
      self.templates_source,
      file_type_str,
      file_name,
    );

    url
  }
}
