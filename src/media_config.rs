#![allow(non_snake_case)]
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::{
  fmt::Debug,
  fs::{self, remove_file},
  path::{Path, PathBuf},
  result::Result,
};

use crate::{
  APPNAME,
  error::MediaCenterConfigError,
  input::{getch, take_string_input},
  media_center::broadcast_search,
  printing::{PrintMessageType, print_message},
};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Copy)]
pub enum MediaCenterType {
  Jellyfin,
  Emby,
  Plex,
}

impl ToString for MediaCenterType {
  fn to_string(&self) -> String {
    match self {
      MediaCenterType::Jellyfin => String::from("Jellyfin"),
      MediaCenterType::Emby => String::from("Emby"),
      MediaCenterType::Plex => String::from("Plex"),
    }
  }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MediaCenterConfig {
  pub media_center_type: MediaCenterType,
  pub server_name: String,
  pub transcoding: bool,
  pub specific_values: Value,
}

#[derive(Clone)]
pub struct Config {
  pub config: MediaCenterConfig,
  pub path: String,
  pub old_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
  pub device_id: String,
  pub address: String,
  pub users: Vec<UserConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserConfig {
  pub access_token: String,
  pub username: String,
  pub user_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserCredentials {
  pub username: String,
  pub password: String,
}

#[derive(Clone, Debug)]
pub enum Objective {
  DeviceID,
  Address,
  MediaCenterType,
  ServerName,
  Users,
  SearchLocalInstance,
  User,
}

pub fn get_mediacenter_folder() -> PathBuf {
  let config_path = dirs::config_dir().unwrap();
  let mut media_center_path = format!(
    "{}/{}/media-center",
    &config_path.display().to_string(),
    APPNAME.to_lowercase()
  );
  if cfg!(windows) {
    media_center_path = media_center_path.replace('/', "\\");
  }
  if !Path::new(&media_center_path).exists() {
    fs::create_dir_all(media_center_path.clone()).unwrap();
  }
  PathBuf::from(media_center_path)
}

impl Config {
  pub fn default() -> Self {
    Config {
      path: String::new(),
      old_path: None,
      config: MediaCenterConfig {
        media_center_type: MediaCenterType::Emby,
        server_name: String::new(),
        transcoding: false,
        specific_values: serde_json::from_str("{}").unwrap(),
      },
    }
  }

  pub fn new(&mut self) -> Result<Vec<Config>, MediaCenterConfigError> {
    let mut files: Vec<Config> = vec![];
    let media_center_folder = get_mediacenter_folder();
    if fs::read_dir(media_center_folder.clone()).unwrap().count() == 0 {
      print_message(
        PrintMessageType::Warning,
        "No media center configuration found. Creating new config...",
      );
      self.ask_for_setting(Objective::MediaCenterType);
      match self.config.media_center_type {
        MediaCenterType::Plex => (),
        _ => self.ask_for_setting(Objective::SearchLocalInstance),
      }
      self.save();
    } else {
      for file in fs::read_dir(media_center_folder).unwrap() {
        let file_path = file.unwrap().path().display().to_string();
        if !file_path.ends_with(".json") {
          continue;
        }
        let content = fs::read_to_string(file_path.clone()).unwrap();
        if let Ok(serialized) = serde_json::from_str::<MediaCenterConfig>(&content) {
          files.append(&mut vec![Config {
            path: file_path,
            config: serialized,
            old_path: None,
          }]);
        }
      }
    }

    Ok(files)
  }

  pub fn read(&mut self) -> Result<(), MediaCenterConfigError> {
    if let Ok(content) = fs::read_to_string(self.path.clone()) {
      if let Ok(serialized) = serde_json::from_str::<MediaCenterConfig>(&content) {
        self.config.media_center_type = serialized.media_center_type;
        self.config.server_name = serialized.server_name;
        self.config.transcoding = serialized.transcoding;
        self.config.specific_values = serialized.specific_values;
        return Ok(());
      }
      Err(MediaCenterConfigError::Corrupt)
    } else {
      Err(MediaCenterConfigError::MissingFile)
    }
  }

  pub fn remove_specific_value(&mut self, setting: Objective, value: String) {
    let address = if let Some(address) = self.config.specific_values.get("address") {
      address.as_str().unwrap().to_string()
    } else {
      String::new()
    };
    let device_id = if let Some(device_id) = self.config.specific_values.get("device_id") {
      device_id.as_str().unwrap().to_string()
    } else {
      String::new()
    };
    let users = if let Some(users) = self.config.specific_values.get("users") {
      serde_json::from_value::<Vec<UserConfig>>(users.clone()).unwrap()
    } else {
      vec![]
    };
    let mut temp = ServerConfig {
      address,
      device_id,
      users,
    };
    match setting {
      Objective::DeviceID => {
        temp.device_id = value;
      },
      Objective::Address => {
        temp.address = value;
        if !temp.address.ends_with('/') {
          temp.address.push('/');
        }
        if !temp.address.starts_with("http://") && !temp.address.starts_with("https://") {
          temp.address = format!("http://{}", temp.address);
        }
      },
      Objective::User => {
        let user_index = temp
          .users
          .iter()
          .position(|u| u.access_token == value)
          .unwrap();
        temp.users.remove(user_index);
      },
      _ => eprintln!("THAT is not a specific config setting."),
    }
    self.config.specific_values = serde_json::to_value(temp).unwrap();
  }

  pub fn insert_specific_value(&mut self, setting: Objective, value: String) {
    let address = if let Some(address) = self.config.specific_values.get("address") {
      address.as_str().unwrap().to_string()
    } else {
      String::new()
    };
    let device_id = if let Some(device_id) = self.config.specific_values.get("device_id") {
      device_id.as_str().unwrap().to_string()
    } else {
      String::new()
    };
    let users = if let Some(users) = self.config.specific_values.get("users") {
      serde_json::from_value::<Vec<UserConfig>>(users.clone()).unwrap()
    } else {
      vec![]
    };
    let mut temp = ServerConfig {
      address,
      device_id,
      users,
    };
    match setting {
      Objective::DeviceID => {
        temp.device_id = value;
      },
      Objective::Address => {
        temp.address = value;
        if !temp.address.ends_with('/') {
          temp.address.push('/');
        }
        if !temp.address.starts_with("http://") && !temp.address.starts_with("https://") {
          temp.address = format!("http://{}", temp.address);
        }
      },
      Objective::Users => {
        temp.users = serde_json::from_str(&value).unwrap();
      },
      Objective::User => {
        temp
          .users
          .append(&mut vec![serde_json::from_str(&value).unwrap()]);
      },
      _ => eprintln!("THAT is not a specific config setting."),
    }
    self.config.specific_values = serde_json::to_value(temp).unwrap();
  }

  pub fn save(&mut self) {
    match fs::write(
      self.path.clone(),
      serde_json::to_string_pretty(&self.config).unwrap(),
    ) {
      Ok(()) => {
        print_message(PrintMessageType::Warning, "Saved media-center config.");
        if let Some(old_path) = &self.old_path {
          if remove_file(old_path).is_ok() {
            print_message(
              PrintMessageType::Warning,
              "Deleted old media-center config.",
            );
            self.old_path = None;
          } else {
            print_message(
              PrintMessageType::Error,
              "Failed to delete old media-center config.",
            );
          }
        }
      },
      Err(e) => print_message(
        PrintMessageType::Error,
        format!("Failed to save config file: {}", e).as_str(),
      ),
    }
  }

  pub fn delete(&mut self) {
    print!(
      "Are you sure you want to delete \"{}\"?\n (Y)es / (N)o",
      self.config.server_name
    );
    match getch("YyNn") {
      'Y' | 'y' => {
        fs::remove_file(self.path.clone()).unwrap();
      },
      _ => (),
    };
    println!();
  }

  pub fn get_device_id(&mut self) -> String {
    if let Some(device_id) = self.config.specific_values.get("device_id") {
      let device_id_string = device_id.as_str().unwrap().to_string();
      if !device_id_string.is_empty() {
        return device_id_string;
      }
    }
    let new_device_id = uuid::Uuid::new_v4().to_string();
    self.insert_specific_value(Objective::DeviceID, new_device_id.clone());
    new_device_id
  }

  pub fn get_address(&mut self) -> Option<String> {
    let serde_address = self.config.specific_values.get("address");
    if serde_address.is_some() {
      let address = serde_address.unwrap().as_str().unwrap();
      match self.config.media_center_type {
        MediaCenterType::Plex => Some(address.to_string()),
        MediaCenterType::Emby => Some(address.to_owned() + "emby/"),
        MediaCenterType::Jellyfin => Some(address.to_string()),
      }
    } else {
      None
    }
  }

  pub fn set_active_user(&mut self, identifier: String) {
    if let Ok(mut old_users) = serde_json::from_value::<Vec<UserConfig>>(
      self.config.specific_values.get("users").unwrap().clone(),
    ) {
      let mut user_index = 0;
      for (index, user) in old_users.iter().enumerate() {
        if user.access_token == identifier {
          user_index = index;
          break;
        }
      }
      let special_user = old_users[user_index].clone();
      old_users.remove(user_index);
      old_users.insert(0, special_user);
      self.insert_specific_value(
        Objective::Users,
        serde_json::to_string_pretty(&old_users).unwrap(),
      );
    }
  }

  pub fn get_active_user(&mut self) -> Option<UserConfig> {
    if let Some(value) = self.config.specific_values.get("users") {
      if let Ok(user) = serde_json::from_value::<UserConfig>(value[0].clone()) {
        return Some(user);
      }
    }
    None
  }

  pub fn remove_user(&mut self, identifier: String) {
    match self.config.media_center_type {
      MediaCenterType::Plex => panic!("this too"),
      _ => {
        self.remove_specific_value(Objective::User, identifier);
      },
    }
  }

  pub fn ask_for_setting(&mut self, setting: Objective) {
    match setting {
      Objective::MediaCenterType => {
        print!(
          "Which kind of server do you want to stream from?\n  [1] Jellyfin\n  [2] Emby\n  [3] Plex"
        );
        self.config.media_center_type = match getch("123") {
          '1' => MediaCenterType::Jellyfin,
          '2' => MediaCenterType::Emby,
          _ => MediaCenterType::Plex,
        };
      },
      Objective::SearchLocalInstance => {
        match self.config.media_center_type {
          MediaCenterType::Jellyfin => {
            if let Some(server_info) = broadcast_search(self.config.media_center_type) {
              self.config.server_name = server_info.Name;
              self.insert_specific_value(Objective::Address, server_info.Address);
            } else {
              self.ask_for_setting(Objective::ServerName);
              self.ask_for_setting(Objective::Address);
            }
          },
          MediaCenterType::Emby => {
            if let Some(server_info) = broadcast_search(self.config.media_center_type) {
              self.config.server_name = server_info.Name;
              self.insert_specific_value(Objective::Address, server_info.Address);
            } else {
              self.ask_for_setting(Objective::ServerName);
              self.ask_for_setting(Objective::Address);
            }
          },
          _ => panic!("Plex servers do not support this feature."),
        }
        if self.check_existing_config() {
          print_message(
            PrintMessageType::Warning,
            "A media-center configuration with that name already exists. Please try again.",
          );
          self.ask_for_setting(Objective::ServerName);
        }
        return;
      },
      Objective::ServerName => {
        if !self.path.is_empty() {
          self.old_path = Some(self.path.clone());
        }
        println!("How do you want to name this media-center?");
        loop {
          self.config.server_name = take_string_input(vec![]).replace(' ', "_");
          let config_path = dirs::config_dir().unwrap();
          self.path = format!(
            "{}/{}/media-center/{}.json",
            &config_path.display().to_string(),
            APPNAME.to_lowercase(),
            self.config.server_name
          );
          if self.check_existing_config() {
            print_message(
              PrintMessageType::Error,
              "A media-center configuration with that name already exists.\nPlease choose a different name.",
            );
          } else {
            break;
          }
        }
      },
      Objective::Address => {
        println!("What's the IP-Adress/Domain to connect to the server?");
        let address: String;
        let reg = regex::Regex::new(r#"^(https?:\/\/)?((([0-9]{1,3}\.){3}([0-9]{1,3}))|([a-z]{3}\.)?([a-zA-Z]+)(\.([a-z]{2,}))+)(:[0-9]{1,5})?(\/[a-zA-Z0-9]+)*\/?$"#).unwrap();
        loop {
          let temp = take_string_input(vec![]);
          if reg.is_match(&temp) {
            address = temp;
            break;
          } else {
            println!("That is not a valid IP-Address or Domain. Please try again.");
          }
        }
        self.insert_specific_value(Objective::Address, address);
      },
      _ => {},
    }
    println!();
  }

  pub fn check_existing_config(&mut self) -> bool {
    if self.path.is_empty() {
      self.config.server_name = self.config.server_name.replace(' ', "_");
      let config_path = dirs::config_dir().unwrap();
      let config_file_path = format!(
        "{}/{}/media-center/{}.json",
        &config_path.display().to_string(),
        APPNAME.to_lowercase(),
        self.config.server_name
      );
      self.path = config_file_path;
    }
    Path::is_file(Path::new(&self.path))
  }
}
