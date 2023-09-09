use colored::Colorize;
use dialoguer::Input;

use crate::{constants::FILE_MARKER, CONFIG_FILE};
use std::{
  fs::File,
  io::{BufReader, Write},
  sync::Arc,
};

use super::LocalConfigFile;

fn get_local_config_file_path_string() -> String {
  let mut binding = std::env::current_dir()
    .unwrap()
    .to_str()
    .unwrap()
    .to_string();

  binding += &format!("{}{}", FILE_MARKER, "wego.yaml");

  binding
}

pub fn init_yaml_file() {
  let file_path = get_local_config_file_path_string();
  if let Ok(mut yaml) = File::create(file_path) {
    let content = format!(
        "github_name: \rrepo_name: \r# The github api token\r# github_api_token: \r \r# You can customize the templates dir path\r# templates_source: templates\r\r# You can customize the repo target branch name(default: main)\r# target_branch: main",
    );

    yaml.write_all(content.as_bytes()).unwrap();
  }
}

pub fn init_yaml_file_with_stdin() -> Result<(), std::io::Error> {
  let github_name: String = Input::new().with_prompt("Github Name").interact_text()?;

  let repo_name: String = Input::new().with_prompt("Repo Name").interact_text()?;

  let github_api_token: String = Input::new()
    .with_prompt("Github Api Token")
    .allow_empty(true)
    .interact_text()?;

  let mut template_source: String = Input::new()
    .with_prompt("Templates source remote dir path(Default: templates)")
    .allow_empty(true)
    .interact_text()?;

  let mut target_branch: String = Input::new()
    .with_prompt("Template repo target branch")
    .allow_empty(true)
    .interact_text()?;

  let mut token: String = String::from("");
  if github_api_token != "" {
    token = github_api_token;
  }

  if target_branch == "" {
    target_branch = "main".to_string();
  }
  if template_source == "" {
    template_source = String::from("templates");
  }
  let file_path = get_local_config_file_path_string();

  if let Ok(mut yaml) = File::create(file_path) {
    let content = format!(
        "github_name: {}\rrepo_name: {}\r# The github api token\r github_api_token: {}\r \r# You can customize the templates dir path\r# templates_source: {}\r\r# You can customize the repo target branch name(default: main)\r# target_branch: {}",
        github_name,
        repo_name,
        token,
        template_source,
        target_branch
    );

    yaml.write_all(content.as_bytes()).unwrap();
  }

  Ok(())
}

pub fn read_config_file_from_local() -> Result<(), ()> {
  let is_ok_file = File::open(get_local_config_file_path_string().as_str());

  if let Ok(file) = is_ok_file {
    let reader = BufReader::new(file);
    let config_file: LocalConfigFile = serde_yaml::from_reader(reader).expect("invalid yaml file");
    let github_api_token = config_file.github_api_token;
    let github_name = config_file.github_name;
    let repo_name = config_file.repo_name;
    let templates_source = config_file.templates_source;
    let target_branch = config_file.target_branch;

    if github_name == "" {
      println!(
        "{} {}",
        "Warning:".red().bold(),
        "github_name is required!".red()
      );
      std::process::exit(0);
    }

    if repo_name == "" {
      println!(
        "{} {}",
        "Warning:".red().bold(),
        "repo_name is required!".red()
      );
      std::process::exit(0);
    }

    let mut token_string = String::from("");

    if let Some(token) = github_api_token {
      token_string = token;
    }

    let mut t_branch = String::from("main");

    if let Some(branch) = target_branch {
      t_branch = branch;
    }

    let mut t_source = String::from("templates");

    if let Some(source) = templates_source {
      t_source = source;
    }

    let arc_file = Arc::clone(&CONFIG_FILE);
    let mut config = arc_file.write().unwrap();

    config.github_name = github_name;
    config.repo_name = repo_name;
    config.github_api_token = token_string;
    config.target_branch = t_branch;
    config.templates_source = t_source;
    Ok(())
  } else {
    println!(
      "{}",
      "Need a wego.yaml, you can use command wego init -y to generate the file.".red()
    );
    Err(())
  }
}
