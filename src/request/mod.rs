use std::{fs, path::Path, pin::Pin, time::Instant};

use base64::decode;
use colored::Colorize;
use dialoguer::{console::Term, theme::ColorfulTheme, Input, Select};
use futures::Future;
use hyper::{
  header::{AUTHORIZATION, USER_AGENT},
  Body, Request,
};
use serde::Deserialize;

use crate::{constants::FILE_MARKER, helper::ConfigFile, TemplateType, CLIENT};

pub mod request;

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigYaml {
  pub name: String,
  pub description: String,
  pub dependencies: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Projects {
  name: String,
  description: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoteYaml {
  pub components: Vec<ConfigYaml>,
  pub pages: Vec<ConfigYaml>,
  pub projects: Vec<Projects>,
}

pub fn get_local_dir_path() -> String {
  let binding = std::env::current_dir()
    .unwrap()
    .to_str()
    .unwrap()
    .to_string();

  binding
}

pub fn create_dir_to_local(name: String, temp_type: TemplateType) -> String {
  let curr_path = get_local_dir_path();
  let type_url: String;
  match temp_type {
    TemplateType::Components => type_url = String::from("src/components"),
    TemplateType::Pages => type_url = String::from("src/pages"),
    TemplateType::Project => type_url = String::from(""),
  }

  let path_vec = vec![curr_path, type_url, name];
  if let Err(e) = fs::create_dir_all(path_vec.join(FILE_MARKER)) {
    println!("Create dir failure: {:#?}", e);
  }

  let path_vec: Vec<String> = path_vec
    .iter()
    .filter(|item| !item.is_empty())
    .cloned()
    .collect();

  path_vec.join(FILE_MARKER)
}

pub fn get_local_dir(name: String, temp_type: TemplateType) -> String {
  let curr_path = get_local_dir_path();
  let type_url: String;
  match temp_type {
    TemplateType::Components => type_url = String::from("src/components"),
    TemplateType::Pages => type_url = String::from("src/pages"),
    TemplateType::Project => type_url = String::from(""),
  }
  let path_vec = vec![curr_path, type_url, name];

  path_vec.join(FILE_MARKER)
}

pub async fn fetch_remote_dir(
  url: String,
  token: &str,
  tb: &str,
) -> Result<RemoteGithubDir, String> {
  let uri = format!("{}?{}={}", &url, "ref", &tb);
  let req_result = Request::builder()
    .uri(uri)
    .method("GET")
    .header(AUTHORIZATION, format!("Bearer {}", token))
    .header(USER_AGENT, "wego")
    .body(Body::empty());

  match req_result {
    Ok(req) => {
      let fetch_res = CLIENT.request(req).await;
      match fetch_res {
        Ok(res) => {
          if res.status() == 200 {
            let body_bytes = hyper::body::to_bytes(res.into_body())
              .await
              .expect("Parse error!!");
            let content_res = serde_json::from_slice::<RemoteGithubDir>(&body_bytes);

            match content_res {
              Ok(content) => {
                return Ok(content);
              }
              Err(e) => {
                println!("{:?}", e.to_string().red());
              }
            }
          } else {
            println!(
              "{} {}",
              "Request remote dir failure, code: ".red(),
              res.status().as_str().red().bold()
            )
          }
        }
        Err(e) => {
          println!("{:?}", e.to_string().red());
        }
      }
    }
    Err(e) => {
      println!("{:?}", e.to_string().red());
    }
  }
  return Err(String::from(""));
}

pub fn find_all_deps(deps: Vec<String>, list: Vec<ConfigYaml>) -> Vec<String> {
  let mut this_deps = deps.clone();
  for item in &list {
    if this_deps.contains(&item.name) {
      if &item.dependencies.len() > &0 {
        for dep in &item.dependencies {
          if !this_deps.contains(dep) {
            this_deps.push(dep.to_string());
            this_deps = find_all_deps(this_deps, list.clone());
          }
        }
      }
    }
  }

  this_deps
}

impl RemoteYaml {
  pub fn get_output_string_vec(&self, output_type: TemplateType) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    match output_type {
      TemplateType::Components => {
        for component in &self.components {
          result.push(format!(
            "{} ----> {} ----> {:?}",
            component.name, component.description, component.dependencies
          ));
        }
        result
      }
      TemplateType::Pages => {
        for component in &self.pages {
          result.push(format!(
            "{} ----> {} ----> {:?}",
            component.name, component.description, component.dependencies
          ));
        }
        result
      }
      TemplateType::Project => {
        for component in &self.projects {
          result.push(format!(
            "{} ----> {}",
            component.name, component.description
          ));
        }
        result
      }
    }
  }

  pub async fn show_projects_templates<F>(
    &self,
    run_job: F,
    config_file: &ConfigFile,
  ) -> Result<(), Box<dyn std::error::Error>>
  where
    F: Fn(Vec<GithubRequestDir>, String, String, String) -> Pin<Box<dyn Future<Output = ()>>>,
  {
    let projects = &self.projects;

    let items = self.get_output_string_vec(TemplateType::Project);

    let selection = Select::with_theme(&ColorfulTheme::default())
      .items(&items)
      .default(0)
      .interact_on_opt(&Term::stderr())?;

    if let Some(index) = selection {
      let this_project = &projects[index];
      let project_name = &this_project.name;

      let custom_name: String = Input::new()
        .with_prompt("Custom file name(Not required)")
        .allow_empty(true)
        .interact_text()?;

      let final_file_name: String;

      if custom_name == "" {
        final_file_name = project_name.clone().to_string();
      } else {
        final_file_name = custom_name;
      }
      let token: String = String::from(&config_file.github_api_token);

      let request_url = config_file.get_remote_template_url(TemplateType::Project, project_name);

      let res = fetch_remote_dir(request_url.clone(), &token, &config_file.target_branch).await;
      match res {
        Ok(info) => {
          let local_path = create_dir_to_local(final_file_name, TemplateType::Project);

          let start_time = Instant::now();

          run_job(info, local_path, token, request_url.clone()).await;

          println!("Done in {:?} ms!", start_time.elapsed().as_millis());
        }
        Err(e) => {
          println!("Error: {:}", e);
        }
      }
    }

    Ok(())
  }

  pub fn get_page_templates_items_by_type(&self, t_type: TemplateType) -> Vec<String> {
    self.get_output_string_vec(t_type)
  }

  pub async fn show_pages_templates<F>(
    &self,
    run_job: F,
    config_file: &ConfigFile,
  ) -> Result<(), Box<dyn std::error::Error>>
  where
    F: Fn(Vec<GithubRequestDir>, String, String, String) -> Pin<Box<dyn Future<Output = ()>>>,
  {
    let pages = &self.pages;

    let items = self.get_output_string_vec(TemplateType::Pages);

    let selection = Select::with_theme(&ColorfulTheme::default())
      .items(&items)
      .default(0)
      .interact_on_opt(&Term::stderr())?;

    if let Some(index) = selection {
      let this_page = &pages[index];
      let file_name = &this_page.name;
      let deps = &this_page.dependencies;

      let custom_name: String = Input::new()
        .with_prompt("Custom file name(Not required)")
        .allow_empty(true)
        .interact_text()?;

      let final_file_name: String;

      if custom_name == "" {
        final_file_name = file_name.clone().to_string();
      } else {
        final_file_name = custom_name;
      }

      let token: String = String::from(&config_file.github_api_token);

      let request_url = config_file.get_remote_template_url(TemplateType::Pages, file_name);
      let res = fetch_remote_dir(request_url.clone(), &token, &config_file.target_branch).await;
      if let Ok(info) = res {
        let local_path = create_dir_to_local(final_file_name, TemplateType::Pages);

        let start_time = Instant::now();
        let _ = &run_job(info, local_path, token.to_string(), request_url.clone()).await;
        println!("Done in {:?} ms!", start_time.elapsed().as_millis());
        if &deps.len() > &0 {
          println!("{} {:?}", "Start loading dependencies ---->".green(), &deps);
          let all = find_all_deps(deps.clone(), self.components.clone());
          self
            .download_components_to_local(all, token.clone(), &run_job, config_file)
            .await;
        }
      } else {
        println!(
          "{} {}",
          "Error: request templates failure! ".red(),
          &request_url
        );
      }
    }

    Ok(())
  }

  pub async fn show_components_templates<F>(
    &self,
    run_job: F,
    config_file: &ConfigFile,
  ) -> Result<(), Box<dyn std::error::Error>>
  where
    F: Fn(Vec<GithubRequestDir>, String, String, String) -> Pin<Box<dyn Future<Output = ()>>>,
  {
    let components = &self.components;

    let items = self.get_output_string_vec(TemplateType::Components);

    let selection = Select::with_theme(&ColorfulTheme::default())
      .items(&items)
      .default(0)
      .interact_on_opt(&Term::stderr())?;

    if let Some(index) = selection {
      let this_comp = &components[index];
      let file_name = &this_comp.name;
      let mut deps = this_comp.dependencies.clone();
      deps.push(String::from(file_name));

      let all = find_all_deps(deps, components.to_vec());
      let token: String = String::from(&config_file.github_api_token);
      self
        .download_components_to_local(all, token, &run_job, config_file)
        .await;
    }
    Ok(())
  }

  pub async fn download_components_to_local<F>(
    &self,
    coms: Vec<String>,
    token: String,
    run_job: F,
    config_file: &ConfigFile,
  ) where
    F: Fn(Vec<GithubRequestDir>, String, String, String) -> Pin<Box<dyn Future<Output = ()>>>,
  {
    if coms.len() > 0 {
      let start_time = Instant::now();
      let mut async_tasks = vec![];
      for item in coms {
        let local_path = get_local_dir(item.clone(), TemplateType::Components);

        let p = Path::new(&local_path);

        if p.exists() {
          println!(
            "{} {} is already existed!",
            "Warning:".red().bold(),
            local_path.red()
          );
        } else {
          let request_url = config_file.get_remote_template_url(TemplateType::Components, &item);
          let res = fetch_remote_dir(request_url.clone(), &token, &config_file.target_branch).await;

          if let Ok(info) = res {
            let local_path = create_dir_to_local(item.clone(), TemplateType::Components);
            let handle = run_job(info, local_path, token.to_string(), request_url.clone());
            async_tasks.push(handle);
          }
        }
      }
      futures::future::join_all(async_tasks).await;
      println!("Done in {:?} ms!", start_time.elapsed().as_millis());
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct Links {
  #[serde(rename = "self")]
  pub _self: String,
  pub git: String,
  pub html: String,
}
#[derive(Debug, Deserialize)]
pub struct ContentsResponse {
  pub name: String,
  pub path: String,
  pub sha: String,
  pub size: u32,
  pub url: String,
  pub html_url: String,
  pub git_url: String,
  pub download_url: String,
  #[serde(rename = "type")]
  pub file_type: String,
  pub content: String,
  pub encoding: String,
  pub _links: Links,
}

impl ContentsResponse {
  pub fn decode_base64_to_string(&self) -> Result<String, Box<dyn std::error::Error>> {
    let r = self
      .content
      .as_bytes()
      .iter()
      .filter(|b| !b" \n\t\r\x0b\x0c".contains(&b))
      .copied()
      .collect::<Vec<u8>>();
    let standard_base64_string = String::from_utf8(r)?;
    let decode_base64_string = decode(&standard_base64_string).unwrap();
    let content_string = String::from_utf8(decode_base64_string).unwrap();
    Ok(content_string)
  }

  pub fn decode_base64_to_u8(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let r = self
      .content
      .as_bytes()
      .iter()
      .filter(|b| !b" \n\t\r\x0b\x0c".contains(&b))
      .copied()
      .collect::<Vec<u8>>();
    let standard_base64_string = String::from_utf8(r)?;
    let decode_base64_u8 = decode(&standard_base64_string).unwrap();
    Ok(decode_base64_u8)
  }
}

#[derive(Debug, Deserialize)]
pub struct GithubRequestDir {
  pub _links: Links,
  pub url: String,
  pub html_url: String,
  pub git_url: String,
  pub download_url: Option<String>,
  pub name: String,
  pub path: String,
  #[serde(rename = "type")]
  pub file_type: String,
  pub sha: String,
  pub size: u32,
}

pub type RemoteGithubDir = Vec<GithubRequestDir>;
