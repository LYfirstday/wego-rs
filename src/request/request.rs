use async_recursion::async_recursion;
use std::{fs, path::Path, sync::Arc, time::Instant, vec};

use colored::Colorize;
use constants::FILE_MARKER;
use dialoguer::{console::Term, theme::ColorfulTheme, Input, Select};

use hyper::header::{AUTHORIZATION, USER_AGENT};
use hyper::{Body, Request};

use crate::request::find_all_deps;
use crate::CLIENT;
use crate::{
  constants,
  request::{ContentsResponse, RemoteYaml},
  TemplateType, CONFIG_FILE,
};

use super::RemoteGithubDir;

pub async fn get_remote_yaml_config(t_type: TemplateType) {
  let uri;
  let token;

  {
    let arc_file = Arc::clone(&CONFIG_FILE);
    let config = arc_file.read().unwrap();

    let url: String = config.get_remote_yaml_url();

    let target_branch = config.target_branch.clone();

    uri = format!("{}?{}={}", &url, "ref", &target_branch);
    token = config.github_api_token.clone();
  }

  let req_result = Request::builder()
    .uri(uri.clone())
    .method("GET")
    .header(AUTHORIZATION, format!("Bearer {}", token))
    .header(USER_AGENT, "wego")
    .body(Body::empty());

  match req_result {
    Ok(req) => {
      let content_res = CLIENT.request(req).await;

      match content_res {
        Ok(res) => {
          let body_bytes = hyper::body::to_bytes(res.into_body())
            .await
            .expect("Parse error!!");
          let content_res = serde_json::from_slice::<ContentsResponse>(&body_bytes);
          match content_res {
            Ok(content) => {
              show_templates_by_type(content, t_type).await;
            }
            Err(_) => {
              println!(
                "{} Request url: {}",
                "There is no wego.yaml in your repo!".red(),
                uri
              );
            }
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
}

pub async fn show_templates_by_type(content: ContentsResponse, t_type: TemplateType) {
  let target_branch;
  let token;
  {
    let arc_file = Arc::clone(&CONFIG_FILE);
    let config = arc_file.read().unwrap();
    target_branch = config.target_branch.clone();
    token = config.github_api_token.clone();
  }
  let content_string = content.decode_base64_to_string().unwrap();
  let info: RemoteYaml = serde_yaml::from_str(&content_string).unwrap();

  match t_type {
    TemplateType::Pages => {
      let vec_pages = info.get_page_templates_items_by_type(t_type);
      let index = show_selection(&vec_pages);
      if index != 99999 {
        select_string_items(info.pages[index].name.clone(), t_type).await;
        // let deps = &info.pages[index].dependencies;

        if let Some(deps) = &info.pages[index].dependencies {
          if deps.len() > 0 {
            let all = find_all_deps(deps.clone(), info.components.clone());
            println!("{} {:?}", "Start loading dependencies ---->".green(), &deps);

            download_components_to_local(all, token, &target_branch).await;
          }
        }
      }
    }
    TemplateType::Components => {
      let vec_coms = info.get_page_templates_items_by_type(t_type);
      let index = show_selection(&vec_coms);
      if index != 99999 {
        select_string_items(info.components[index].name.clone(), t_type).await;

        if let Some(deps) = &info.components[index].dependencies {
          if deps.len() > 0 {
            let all = find_all_deps(deps.clone(), info.components.clone());
            println!("{} {:?}", "Start loading dependencies ---->".green(), &deps);

            download_components_to_local(all, token, &target_branch).await;
          }
        }
      }
    }
    TemplateType::Project => {
      let vec_coms = info.get_page_templates_items_by_type(t_type);
      let index = show_selection(&vec_coms);
      if index != 99999 {
        select_string_items(info.projects[index].name.clone(), t_type).await;
      }
    }
  }
}

pub fn show_selection(items: &Vec<String>) -> usize {
  let selection = Select::with_theme(&ColorfulTheme::default())
    .items(items)
    .default(0)
    .interact_on_opt(&Term::stderr());

  if let Ok(Some(index)) = selection {
    index
  } else {
    99999
  }
}

pub async fn select_string_items(this_page_name: String, t_type: TemplateType) {
  let page_name = &this_page_name;

  let request_url;
  let tb;
  let gt;

  {
    let arc_file = Arc::clone(&CONFIG_FILE);
    let config = arc_file.read().unwrap();

    request_url = config.get_remote_template_url(t_type, page_name);
    tb = config.target_branch.clone();

    gt = config.github_api_token.clone();
  }
  let mut final_file_name: String = page_name.clone();
  if let Ok(custom_name) = Input::new()
    .with_prompt("Custom file name(Not required)")
    .allow_empty(true)
    .interact_text()
  {
    if custom_name == "" {
      final_file_name = page_name.clone();
    } else {
      final_file_name = custom_name;
    }
  }
  let token: String = String::from(&gt);
  let res = fetch_remote_dir(request_url.clone(), &token, &tb).await;

  if res.len() > 0 {
    let local_path = create_dir_to_local(final_file_name, t_type);

    let start_time = Instant::now();

    run_job(res, local_path, token, request_url.clone(), &tb).await;

    println!("Done in {:?} ms!", start_time.elapsed().as_millis());
  }
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

pub fn get_item_url(file_type: TemplateType, file_name: &String) -> String {
  let arc_config = Arc::clone(&crate::CONFIG_FILE);
  let config = arc_config.read().unwrap();

  config.get_remote_template_url(file_type, &file_name)
}

pub async fn download_components_to_local(
  coms: Vec<String>,
  token: String,
  target_branch: &String,
) {
  if coms.len() > 0 {
    let start_time = Instant::now();
    let mut async_tasks = vec![];

    for item in coms {
      let local_path = get_local_dir(item.clone(), TemplateType::Components);
      let request_url = get_item_url(TemplateType::Components, &item);
      let p = Path::new(&local_path);

      if p.exists() {
        println!(
          "{} {} is already existed!",
          "Warning:".red().bold(),
          local_path.red()
        );
      } else {
        let res = fetch_remote_dir(request_url.clone(), &token, &target_branch).await;
        if res.len() > 0 {
          let local_path = create_dir_to_local(item.clone(), TemplateType::Components);
          let handle = run_job(
            res,
            local_path,
            token.to_string(),
            request_url.clone(),
            &target_branch,
          );
          async_tasks.push(handle);
        }
      }
    }
    futures::future::join_all(async_tasks).await;
    println!("Done in {:?} ms!", start_time.elapsed().as_millis());
  }
}

pub async fn fetch_repo_yaml_file(t_type: TemplateType) {
  get_remote_yaml_config(t_type).await;
}

pub async fn fetch_remote_dir(url: String, token: &str, tb: &str) -> RemoteGithubDir {
  let uri = format!("{}?{}={}", &url, "ref", &tb);

  let req_result = Request::builder()
    .uri(uri)
    .method("GET")
    .header(AUTHORIZATION, format!("Bearer {}", token))
    .header(USER_AGENT, "wego")
    .body(Body::empty());

  match req_result {
    Ok(req) => {
      let fetch_res = CLIENT.request(req).await.expect("");

      let body_bytes = hyper::body::to_bytes(fetch_res.into_body())
        .await
        .expect("Parse error!!");
      let content_res = serde_json::from_slice::<RemoteGithubDir>(&body_bytes);

      match content_res {
        Ok(content) => {
          return content;
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
  return vec![];
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

async fn run_job(
  info: RemoteGithubDir,
  local_path: String,
  token: String,
  _request_path: String,
  target_branch: &String,
) {
  let mut async_tasks = vec![];
  for data in info {
    let this_dir_path = vec![local_path.clone(), data.name.clone()].join(FILE_MARKER);
    if data.file_type == "dir" {
      let handle = create_local_dir(
        this_dir_path.clone(),
        token.to_string(),
        data.url,
        target_branch.clone(),
      );

      async_tasks.push(handle);
    } else {
      let handle = create_local_file(
        data.url,
        this_dir_path.clone(),
        token.to_string(),
        target_branch.clone(),
        data.html_url,
      );
      async_tasks.push(handle);
    }
  }
  futures::future::join_all(async_tasks).await;
}

#[async_recursion]
async fn create_local_dir(
  local_path: String,
  token: String,
  request_path: String,
  target_branch: String,
) {
  if let Err(e) = fs::create_dir_all(&local_path) {
    println!("Create dir failure: {:#?}", e);
  }
  println!("{}, {}", &local_path.green(), "Create done!");
  let req_result = Request::builder()
    .uri(&request_path)
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
                futures::join!(run_job(
                  content,
                  local_path.clone(),
                  token.clone(),
                  request_path.clone(),
                  &target_branch,
                ));
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
}

#[async_recursion]
async fn create_local_file(
  request_path: String,
  local_path: String,
  token: String,
  _target_branch: String,
  preview_url: String,
) {
  let req_result = Request::builder()
    .uri(&request_path)
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
            let content_res = serde_json::from_slice::<ContentsResponse>(&body_bytes);

            match content_res {
              Ok(content) => {
                if let Ok(content_u8) = content.decode_base64_to_u8() {
                  if let Err(e) = fs::write(local_path.clone(), content_u8) {
                    println!("Write local config file failure: {:#?}", e);
                  }
                  if request_path.contains("README") {
                    println!("{}, {}", &preview_url.white(), "Write done!");
                  } else {
                    println!("{}, {}", &local_path.green(), "Write done!");
                  }
                }
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
}
