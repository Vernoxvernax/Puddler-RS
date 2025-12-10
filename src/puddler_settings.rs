#![allow(non_camel_case_types)]

use colored::Colorize;
use config::{Config, File};
use serde_derive::{Deserialize, Serialize};
use std::{
  fmt::Display,
  fs,
  io::prelude::*,
  path::{Path, PathBuf},
};

use crate::{
  APPNAME,
  error::PuddlerSettingsError,
  input::{InteractiveOption, InteractiveOptionType, interactive_select, take_string_input},
  media_config::get_mediacenter_folder,
  printing::{PrintMessageType, print_message},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuddlerSettings {
  pub default_media_server: Option<String>,
  pub discord_presence: bool,
  pub fullscreen: bool,
  pub gpu: bool,
  pub glsl_shaders: Vec<String>,
  pub mpv_config_location: Option<String>,
  pub mpv_debug_log: bool,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone)]
enum PuddlerSettingType {
  DefaultMediaServer,
  DiscordPresence,
  Fullscreen,
  GPU,
  GLSL_Shaders,
  MPV_Config_Location,
  MPV_Debug,
}

impl Display for PuddlerSettingType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let out = match self {
      PuddlerSettingType::DefaultMediaServer => String::from("Default media server"),
      PuddlerSettingType::DiscordPresence => String::from("Discord Presence"),
      PuddlerSettingType::Fullscreen => String::from("Fullscreen"),
      PuddlerSettingType::GPU => String::from("Hardware Acceleration"),
      PuddlerSettingType::GLSL_Shaders => String::from("GLSL Shaders"),
      PuddlerSettingType::MPV_Config_Location => String::from("MPV Config Location"),
      PuddlerSettingType::MPV_Debug => String::from("MPV Debug Log"),
    };
    write!(f, "{}", out)
  }
}

pub fn get_config_path() -> PathBuf {
  let config_path = dirs::config_dir().unwrap();
  let mut config_file_path = format!(
    "{}/{}/{}.toml",
    &config_path.display().to_string(),
    APPNAME.to_lowercase(),
    APPNAME
  );
  if cfg!(windows) {
    config_file_path = config_file_path.replace('/', "\\");
  }
  if !Path::new(&config_path).exists() {
    fs::create_dir(config_path.clone()).unwrap();
  }
  PathBuf::from(config_file_path)
}

impl PuddlerSettings {
  pub fn new() -> Result<Self, PuddlerSettingsError> {
    let config_file_path = get_config_path();
    if config_file_path.exists() {
      loop {
        let settings_file_raw = Config::builder()
          .add_source(File::from(config_file_path.clone()))
          .build()
          .unwrap();
        let serialized = settings_file_raw.try_deserialize::<PuddlerSettings>();
        match serialized {
          Ok(settings) => {
            return Ok(settings);
          },
          Err(e) => {
            if e.to_string().contains("missing field") {
              print_message(
                PrintMessageType::Warning,
                "Settings file is corrupt. Attempting to fix it ...",
              );
              let mut settings_file = fs::OpenOptions::new()
                .append(true)
                .open(&config_file_path)
                .unwrap();
              match &e.to_string()[e.to_string().find('`').unwrap() + 1..e.to_string().len() - 1] {
                "discord_presence" => {
                  let discord_presence =
                    Self::ask_for_setting(PuddlerSettingType::DiscordPresence).discord_presence;
                  writeln!(settings_file, "discord_presence = {discord_presence}").unwrap();
                  continue;
                },
                "fullscreen" => {
                  let fullscreen = Self::ask_for_setting(PuddlerSettingType::Fullscreen).fullscreen;
                  writeln!(settings_file, "fullscreen = {fullscreen}").unwrap();
                  continue;
                },
                "gpu" => {
                  let gpu = Self::ask_for_setting(PuddlerSettingType::GPU).gpu;
                  writeln!(settings_file, "gpu = {gpu}").unwrap();
                  continue;
                },
                "glsl_shaders" => {
                  let glsl_shaders =
                    Self::ask_for_setting(PuddlerSettingType::GLSL_Shaders).glsl_shaders;
                  writeln!(settings_file, "glsl_shaders = {glsl_shaders:?}").unwrap();
                  continue;
                },
                "mpv_debug_log" => {
                  let mpv_debug_log =
                    Self::ask_for_setting(PuddlerSettingType::MPV_Debug).mpv_debug_log;
                  writeln!(settings_file, "mpv_debug_log = {mpv_debug_log}").unwrap();
                  continue;
                },
                something => {
                  print_message(
                    PrintMessageType::Error,
                    format!("Failed to fix because of {}.", something).as_str(),
                  );
                  return Err(PuddlerSettingsError::Corrupt);
                },
              }
            }
          },
        }
      }
    } else {
      print_message(
        PrintMessageType::Warning,
        "No settings file found!\nBuilding default settings ...\n",
      );
      let mut config = Self::ask_for_everything();
      config.write();
      Ok(config)
    }
  }

  pub fn change_menu(&mut self) {
    let mut current_selection = 0;
    loop {
      let default_media_server = if let Some(path) = &self.default_media_server {
        path.to_string()
      } else {
        "None".to_string()
      };
      let discord_presence = if self.discord_presence {
        format!("{}", "Enabled".green())
      } else {
        format!("{}", "Disabled".red())
      };
      let fullscreen = if self.fullscreen {
        format!("{}", "Enabled".green())
      } else {
        format!("{}", "Disabled".red())
      };
      let hardware_acceleration = if self.gpu {
        format!("{}", "Enabled".green())
      } else {
        format!("{}", "Disabled".red())
      };
      let mpv_debug_log = if self.mpv_debug_log {
        format!("{}", "Enabled".green())
      } else {
        format!("{}", "Disabled".red())
      };
      let glsl_shaders = self.glsl_shaders.join(", ");
      let mpv_config_location = if let Some(path) = &self.mpv_config_location {
        path.to_string()
      } else {
        "None".to_string()
      };
      let settings = vec![
        InteractiveOption {
          text: String::from("General Settings:"),
          option_type: InteractiveOptionType::Header,
        },
        InteractiveOption {
          text: String::from("Default media server: ") + &default_media_server,
          option_type: InteractiveOptionType::Button,
        },
        InteractiveOption {
          text: String::from("Discord Presence:") + &discord_presence,
          option_type: InteractiveOptionType::ListButtons,
        },
        InteractiveOption {
          text: String::from("Fullscreen:") + &fullscreen,
          option_type: InteractiveOptionType::ListButtons,
        },
        InteractiveOption {
          text: String::from("Hardware Acceleration:") + &hardware_acceleration,
          option_type: InteractiveOptionType::ListButtons,
        },
        InteractiveOption {
          text: String::from("GLSL Shaders"),
          option_type: InteractiveOptionType::TextInput(glsl_shaders),
        },
        InteractiveOption {
          text: String::from("MPV Config Location"),
          option_type: InteractiveOptionType::TextInput(mpv_config_location),
        },
        InteractiveOption {
          text: String::from("MPV Debug Log:") + &mpv_debug_log,
          option_type: InteractiveOptionType::ListButtons,
        },
        InteractiveOption {
          text: String::from("Save"),
          option_type: InteractiveOptionType::Button,
        },
        InteractiveOption {
          text: String::from("Back"),
          option_type: InteractiveOptionType::Special,
        },
      ];
      match interactive_select(settings, current_selection) {
        (_, _, InteractiveOptionType::Special) => break,
        ((i1, _), Some(string), InteractiveOptionType::Button) if string == "Save" => {
          self.write();
          current_selection = i1;
        },
        ((i1, _), Some(text), InteractiveOptionType::TextInput(_)) => {
          current_selection = i1;
          match i1 {
            4 => {
              self.glsl_shaders.clear();
              if !text.is_empty() && !text.starts_with("None") {
                for part in text.split_terminator(",") {
                  self.glsl_shaders.push(part.trim().to_string());
                }
              }
            },
            5 => {
              if text.is_empty() || text.starts_with("None") {
                self.mpv_config_location = None;
              } else {
                self.mpv_config_location = Some(text.trim().to_string());
              }
            },
            _ => (),
          }
        },
        ((i1, _), _, _) => {
          current_selection = i1;
          match i1 {
            0 => {
              self.change_setting(PuddlerSettingType::DefaultMediaServer);
            },
            1 => {
              self.discord_presence = !self.discord_presence;
            },
            2 => {
              self.fullscreen = !self.fullscreen;
            },
            3 => self.gpu = !self.gpu,
            6 => {
              self.mpv_debug_log = !self.mpv_debug_log;
            },
            _ => (),
          }
        },
      }
    }
  }

  fn change_setting(&mut self, setting: PuddlerSettingType) {
    let change = Self::ask_for_setting(setting.clone());
    match setting {
      PuddlerSettingType::DefaultMediaServer => {
        self.default_media_server = change.default_media_server
      },
      PuddlerSettingType::DiscordPresence => self.discord_presence = change.discord_presence,
      PuddlerSettingType::Fullscreen => self.fullscreen = change.fullscreen,
      PuddlerSettingType::GPU => self.gpu = change.gpu,
      PuddlerSettingType::GLSL_Shaders => self.glsl_shaders = change.glsl_shaders,
      PuddlerSettingType::MPV_Config_Location => {
        self.mpv_config_location = change.mpv_config_location
      },
      PuddlerSettingType::MPV_Debug => self.mpv_debug_log = change.mpv_debug_log,
    }
  }

  fn ask_for_everything() -> Self {
    PuddlerSettings {
      default_media_server: Self::ask_for_setting(PuddlerSettingType::DefaultMediaServer)
        .default_media_server,
      discord_presence: Self::ask_for_setting(PuddlerSettingType::DiscordPresence).discord_presence,
      fullscreen: Self::ask_for_setting(PuddlerSettingType::Fullscreen).fullscreen,
      gpu: Self::ask_for_setting(PuddlerSettingType::GPU).gpu,
      glsl_shaders: Self::ask_for_setting(PuddlerSettingType::GLSL_Shaders).glsl_shaders,
      mpv_config_location: Self::ask_for_setting(PuddlerSettingType::MPV_Config_Location)
        .mpv_config_location,
      mpv_debug_log: Self::ask_for_setting(PuddlerSettingType::MPV_Debug).mpv_debug_log,
    }
  }

  fn ask_for_setting(setting: PuddlerSettingType) -> Self {
    let media_center_path = get_mediacenter_folder();
    let mut temp = PuddlerSettings {
      default_media_server: None,
      discord_presence: false,
      fullscreen: false,
      gpu: false,
      glsl_shaders: vec![],
      mpv_config_location: None,
      mpv_debug_log: false,
    };
    match setting {
      PuddlerSettingType::DefaultMediaServer => {
        println!(
          "Searching in \"{}\" for configuration files ...",
          &media_center_path.to_str().unwrap()
        );
        let path: Vec<_> = fs::read_dir(media_center_path)
          .unwrap()
          .map(|r| r.unwrap())
          .collect();
        let mut files: Vec<String> = vec![];
        for file in &path {
          if file.path().is_dir() {
            let depth2: Vec<_> = fs::read_dir(file.path())
              .unwrap()
              .map(|r| r.unwrap())
              .collect();
            for stuff in depth2 {
              let file_path: String = stuff.path().display().to_string();
              if file_path.contains(".json") {
                files.append(&mut [file_path].to_vec());
              } else {
                continue;
              }
            }
          }
          let file_path: String = file.path().display().to_string();
          if file_path.contains(".json") {
            files.append(&mut [file_path].to_vec());
          } else {
            continue;
          }
        }
        let mut file_selection: Vec<String> = vec![];
        if files.is_empty() {
          println!("No configuration has been found.\n");
          return temp;
        } else {
          for (index, path) in files.iter().enumerate() {
            file_selection.append(&mut vec![index.to_string()]);
            println!("  [{index}] {path}");
          }
        }
        file_selection.append(&mut vec!["None".to_string()]);
        println!(
          "Select which one of the above server configs should be used by default. Skip this option with \"None\"."
        );
        let selection = take_string_input(file_selection);
        if selection.trim() == "None" {
          println!("Skipped default-server option.\n");
          return temp;
        }
        let num_selection: usize = selection.trim().parse().unwrap();
        println!(
          "\nYou've picked {}.",
          format!("{:?}", files[num_selection]).green()
        );
        temp.default_media_server = Some(files.get(num_selection).unwrap().to_string());
      },
      _ => print_message(
        PrintMessageType::Error,
        "Asking for this setting is not implemented here.",
      ),
    }
    println!();
    temp
  }

  fn write(&mut self) {
    let pretty_string = toml::to_string_pretty(&self).unwrap();
    fs::write(get_config_path(), pretty_string).expect("Saving settings failed.");
    print_message(
      PrintMessageType::Success,
      "Saved changes to \"Puddler.toml\".",
    );
  }
}
