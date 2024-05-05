#![allow(non_camel_case_types)]

use colored::Colorize;
use config::{Config, File};
use std::{
  fs,
  io::prelude::*,
  path::Path
};
use serde_derive::{Deserialize,Serialize};

use crate::{error::PuddlerSettingsError, input::{getch, take_string_input}, printing::print_message, APPNAME};
use crate::printing::PrintMessageType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuddlerSettings {
  pub default_media_server: Option<String>,
  pub discord_presence: bool,
  pub fullscreen: bool,
  pub gpu: bool,
  pub glsl_shaders: Vec<String>,
  pub mpv_config_location: Option<String>,
  pub mpv_debug_log: bool
}

#[derive(Clone)]
enum PuddlerSettingType {
  DefaultMediaServer,
  DiscordPresence,
  Fullscreen,
  GPU,
  GLSL_Shaders,
  MPV_Config_Location,
  MPV_Debug
}

impl ToString for PuddlerSettingType {
  fn to_string(&self) -> String {
    match self {
      PuddlerSettingType::DefaultMediaServer => String::from("Default media server"),
      PuddlerSettingType::DiscordPresence => String::from("Discord Presence"),
      PuddlerSettingType::Fullscreen => String::from("Fullscreen"),
      PuddlerSettingType::GPU => String::from("Hardware Encoding"),
      PuddlerSettingType::GLSL_Shaders => String::from("GLSL Shaders"),
      PuddlerSettingType::MPV_Config_Location => String::from("MPV Config Location"),
      PuddlerSettingType::MPV_Debug => String::from("MPV Debug Log"),
    }
  }
}

impl PuddlerSettingType {
  fn all_types() -> Vec<PuddlerSettingType> {
    vec![
      PuddlerSettingType::DefaultMediaServer,
      PuddlerSettingType::DiscordPresence,
      PuddlerSettingType::Fullscreen,
      PuddlerSettingType::GPU,
      PuddlerSettingType::GLSL_Shaders,
      PuddlerSettingType::MPV_Config_Location,
      PuddlerSettingType::MPV_Debug
    ]
  }
}

impl PuddlerSettings {
  pub fn new() -> Result<Self, PuddlerSettingsError> {
    let config_path = dirs::config_dir().unwrap();
    let settings_path_string = format!("{}/{}/{}.toml", &config_path.display().to_string(), APPNAME.to_lowercase(), APPNAME);
    let config_files_dir_str = &(config_path.to_str().unwrap().to_owned()+"/media-center");
    let config_files_dir = Path::new(config_files_dir_str);
    if Path::new(&format!("{}/{}", &config_path.display().to_string(), APPNAME.to_lowercase())).exists() {
      if !config_files_dir.exists() {
        fs::create_dir_all(config_files_dir).unwrap();
      }
      if Path::new(&settings_path_string).is_file() {
        loop {
          let settings_file_raw = Config::builder().add_source(File::from(Path::new(&settings_path_string))).build().unwrap();
          let serialized = settings_file_raw.try_deserialize::<PuddlerSettings>();
          match serialized {
            Ok(settings) => {
              return Ok(settings);
            },
            Err(e) => {
              if e.to_string().contains("missing field") {
                print_message(PrintMessageType::Warning, "Settings file is corrupt. Attempting to fix it ...");
                let mut settings_file = fs::OpenOptions::new().append(true).open(&settings_path_string).unwrap();
                match &e.to_string()[e.to_string().find('`').unwrap() + 1..e.to_string().len() - 1] {
                  "discord_presence" => {
                    let discord_presence = Self::ask_for_setting(PuddlerSettingType::DiscordPresence).discord_presence;
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
                    let glsl_shaders = Self::ask_for_setting(PuddlerSettingType::GLSL_Shaders).glsl_shaders;
                    writeln!(settings_file, "glsl_shaders = {glsl_shaders:?}").unwrap();
                    continue;
                  },
                  "mpv_debug_log" => {
                    let mpv_debug_log = Self::ask_for_setting(PuddlerSettingType::MPV_Debug).mpv_debug_log;
                    writeln!(settings_file, "mpv_debug_log = {mpv_debug_log}").unwrap();
                    continue;
                  },
                  something => {
                    print_message(PrintMessageType::Error, format!("Failed to fix because of {}.", something).as_str());
                    return Err(PuddlerSettingsError::Corrupt);
                  }
                }
              }
            }
          }
        }
      } else {
        print_message(PrintMessageType::Warning, "No settings file found!\nBuilding default settings ...\n");
        let mut config = Self::ask_for_everything();
        config.write();
        Ok(config)
      }
    } else {
      if !config_files_dir.exists() {
        fs::create_dir_all(config_files_dir).unwrap();
      }
      print_message(PrintMessageType::Warning, "No settings file found!\nBuilding default settings ...\n");
      let mut config = Self::ask_for_everything();
      config.write();
      Ok(config)
    }
  }

  fn get_setting_value(&mut self, setting: PuddlerSettingType) -> String {
    match setting {
      PuddlerSettingType::DefaultMediaServer => format!("{:?}", self.default_media_server),
      PuddlerSettingType::DiscordPresence => format!("{}", self.discord_presence),
      PuddlerSettingType::Fullscreen => format!("{}", self.fullscreen),
      PuddlerSettingType::GPU => format!("{}", self.gpu),
      PuddlerSettingType::GLSL_Shaders => format!("{:?}", self.glsl_shaders),
      PuddlerSettingType::MPV_Config_Location => format!("{:?}", self.mpv_config_location),
      PuddlerSettingType::MPV_Debug => format!("{}", self.mpv_debug_log),
    }
  }

  pub fn change_menu(&mut self) {
    loop {
      let mut allowed = String::new();
      println!("Which settings do you want to change?");
      for (index, setting_type) in PuddlerSettingType::all_types().iter().enumerate() {
        println!("  [{}] {}: {}", index, setting_type.to_string(), self.get_setting_value(setting_type.clone()).underline());
        allowed.push_str(&index.to_string());
      }
      print!(" [S] Save and return to the menu.");
      allowed.push_str("Ss");
      let input = getch(&allowed);
      println!();
      if input == 's' {
        self.write();
        break;
      }
      let selection: usize = input.to_digit(10).unwrap() as usize;
      self.change_setting(PuddlerSettingType::all_types().get(selection).unwrap().clone());
    }
  }

  fn change_setting(&mut self, setting: PuddlerSettingType) {
    let change = Self::ask_for_setting(setting.clone());
    match setting {
      PuddlerSettingType::DefaultMediaServer => self.default_media_server = change.default_media_server,
      PuddlerSettingType::DiscordPresence => self.discord_presence = change.discord_presence,
      PuddlerSettingType::Fullscreen => self.fullscreen = change.fullscreen,
      PuddlerSettingType::GPU => self.gpu = change.gpu,
      PuddlerSettingType::GLSL_Shaders => self.glsl_shaders = change.glsl_shaders,
      PuddlerSettingType::MPV_Config_Location => self.mpv_config_location = change.mpv_config_location,
      PuddlerSettingType::MPV_Debug => self.mpv_debug_log = change.mpv_debug_log,
    }
  }

  fn ask_for_everything() -> Self {
    PuddlerSettings {
      default_media_server: Self::ask_for_setting(PuddlerSettingType::DefaultMediaServer).default_media_server,
      discord_presence: Self::ask_for_setting(PuddlerSettingType::DiscordPresence).discord_presence,
      fullscreen: Self::ask_for_setting(PuddlerSettingType::Fullscreen).fullscreen,
      gpu: Self::ask_for_setting(PuddlerSettingType::GPU).gpu,
      glsl_shaders: Self::ask_for_setting(PuddlerSettingType::GLSL_Shaders).glsl_shaders,
      mpv_config_location: Self::ask_for_setting(PuddlerSettingType::MPV_Config_Location).mpv_config_location,
      mpv_debug_log: Self::ask_for_setting(PuddlerSettingType::MPV_Debug).mpv_debug_log
    }
  }

  fn ask_for_setting(setting: PuddlerSettingType) -> Self {
    let config_path = dirs::config_dir().unwrap().join(APPNAME.to_lowercase());
    let config_files_dir = &(config_path.to_str().unwrap().to_owned()+"/media-center");
    let mut temp = PuddlerSettings {
      default_media_server: None,
      discord_presence: false,
      fullscreen: false,
      gpu: false,
      glsl_shaders: vec![],
      mpv_config_location: None,
      mpv_debug_log: false
    };
    match setting {
      PuddlerSettingType::DefaultMediaServer => {
        println!("Searching in \"{}\" for configuration files ...", &config_files_dir);
        let path: Vec<_> = fs::read_dir(config_files_dir).unwrap().map(|r| r.unwrap()).collect();
        let mut files: Vec<String> = vec![];
        for file in &path {
          if file.path().is_dir() {
            let depth2: Vec<_> = fs::read_dir(&file.path()).unwrap().map(|r| r.unwrap()).collect();
            for stuff in depth2 {
              let file_path: String = stuff.path().display().to_string();
              if file_path.contains(".json") {
                files.append(&mut [file_path].to_vec());  
              } else {
                continue
              }
            }
          }
          let file_path: String = file.path().display().to_string();
          if file_path.contains(".json") {
            files.append(&mut [file_path].to_vec());
          } else {
            continue
          }
        };
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
        println!("Select which one of the above server configs should be used by default. Skip this option with \"None\".");
        let selection = take_string_input(file_selection);
        if selection.trim() == "None" {
          println!("Skipped default-server option.\n");
          return temp;
        }
        let num_selection: usize = selection.trim().parse().unwrap();
        println!("\nYou've picked {}.", format!("{:?}", files[num_selection]).green());
        temp.default_media_server = Some(files.get(num_selection).unwrap().to_string());
      },
      PuddlerSettingType::DiscordPresence => {
        print!("Do you want to activate Discord-Presence by default?\n (Y)es / (N)o");
        let presence = getch("YyNn");
        temp.discord_presence = match presence {
          'Y' | 'y' => {
            true
          }
          _ => false
        };
      },
      PuddlerSettingType::Fullscreen => {
        print!("Do you want mpv to start in fullscreen-mode?\n (Y)es / (N)o");
        let fullscreen = getch("YyNn");
        temp.fullscreen = match fullscreen {
          'Y' | 'y' => {
            true
          }
          _ => false
        };
      },
      PuddlerSettingType::GPU => {
        print!("Do you want to enable hardware decoding for MPV?\n(Using \"auto-safe\" api)\n (Y)es / (N)o");
        let gpu = getch("YyNn");
        temp.gpu = match gpu {
          'Y' | 'y' => {
            true
          }
          _ => false
        };
      },
      PuddlerSettingType::GLSL_Shaders => {
        println!("Do you want to configure any GLSL-Shaders for MPV?\n(Multiple paths can be added whilst confirming with ENTER; to finish, enter nothing)");
        let mut glsl_shaders: Vec<String> = vec![];
        loop {
          let path = take_string_input(vec![]);
          if path.trim().is_empty() {
            break;
          }
          glsl_shaders.append(&mut vec![path]);
        }
        temp.glsl_shaders = glsl_shaders;
      },
      PuddlerSettingType::MPV_Config_Location => {
        println!("Do you want to load an mpv-config?\n(Type the path to the config-directory. f.e. \"~/.config/mpv\"| <Empty input for no>)");
        let path = take_string_input(vec![]);
        if !path.trim().is_empty() {
          temp.mpv_config_location = Some(path);
        }
      },
      PuddlerSettingType::MPV_Debug => {
        print!("Do you want MPV to debug-log to \"./mpv.log\"?\n (Y)es / (N)o");
        let debug = getch("YyNn");
        temp.mpv_debug_log = match debug {
          'Y' | 'y' => {
            true
          },
          _ => false
        };
      }
    }
    println!();
    temp
  }

  fn write(&mut self) {
    let config_path = dirs::config_dir().unwrap().join(APPNAME.to_lowercase());
    let settings_path_string = format!("{}/{}.toml", &config_path.display().to_string(), APPNAME);
    let pretty_string = toml::to_string_pretty(&self).unwrap();
    fs::write(settings_path_string, pretty_string).expect("Saving settings failed.");
    print_message(PrintMessageType::Success, "Saved changes to \"Puddler.toml\".");
  } 
}
