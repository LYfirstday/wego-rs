#![deny(clippy::all)]

pub mod constants;
pub mod helper;
pub mod request;
use helper::ConfigFile;
use hyper::{Body, Client};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use napi::bindgen_prelude::*;
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;

use crate::request::request::get_remote_yaml_config;

#[macro_use]
extern crate napi_derive;

/**
 * 模板类型枚举
 */
#[napi]
pub enum TemplateType {
  Pages,
  Components,
  Project,
}

lazy_static! {
  pub static ref API_PREFIX: String = String::from("https://api.github.com/repos");
  pub static ref CONFIG_FILE: Arc<RwLock<ConfigFile>> = Arc::new(RwLock::new(ConfigFile {
    github_api_token: String::from(""),
    github_name: String::from(""),
    repo_name: String::from(""),
    target_branch: String::from("main"),
    templates_source: String::from("templates")
  }));
  static ref CLIENT: Client<HttpsConnector<hyper::client::HttpConnector>> = {
    let https = HttpsConnectorBuilder::new()
      .with_native_roots()
      .https_only()
      .enable_http1()
      .build();
    let client = Client::builder().build::<_, Body>(https);

    client
  };
}

#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}

/**
 * 初始化本地yaml配置文件
 */
#[napi]
pub fn init_yaml_file() {
  helper::utils::init_yaml_file();
}

/**
 * 根据用户输入生成yaml配置文件
 */
#[napi]
pub fn init_yaml_file_with_stdin() {
  if let Err(e) = helper::utils::init_yaml_file_with_stdin() {
    println!("Create yaml file failure, cause: {:?}", e);
  }
}

/**
 * 请求模板
 */
#[napi]
pub async fn request_remote_templates(template_type: TemplateType) {
  if let Ok(_) = helper::utils::read_config_file_from_local() {
    get_remote_yaml_config(template_type).await;
  }
}
