use colored::Colorize;
use config::{Config, File};
use std::fs;
use toml;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use serde_derive::{Deserialize,Serialize};

use crate::APPNAME;
use crate::mediaserver_information;
use mediaserver_information::getch;


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
  pub server_config: Option<String>,
  pub discord_presence: bool,
  pub transcoding: bool,
  pub fullscreen: bool,
  pub autologin: bool,
  pub autoplay: bool,
  pub gpu: bool,
  pub load_config: bool,
  pub glsl_shader: Option<Vec<String>>,
  pub mpv_debug: Option<bool>,
  pub mpv_config_location: Option<String>
}


fn read_settings() -> Settings {
  let config_path = dirs::config_dir().unwrap();
  let config_path_string = format!("{}/{}/{}.toml", &config_path.display().to_string(), APPNAME.to_lowercase(), APPNAME);
  if ! Path::new(&config_path_string).is_file() {
    println!("No settings file found!\nBuilding default settings ...\n");
    // Default <> server.
    let server_config: Option<String> = ask_search_server_configs();
    // Discord Presence default setting.
    let discord_presence: bool = ask_initiate_discord();
    // Activate encoded streaming (requires fully configured media-server).
    let transcoding: bool = ask_transcoding();
    // Whether mpv should start in fullscreen mode.
    let fullscreen: bool = ask_start_fullscreen();
    // Whether the user should be prompted if the default login is correct.
    let autologin: bool = ask_automatically_login();
    // Whether the user should be prompted to continue after an episode has been finished.
    let autoplay: bool = ask_autoplay();
    // Whether mpv should try to use hardware decoding.
    let gpu: bool = ask_gpu();
    // Whether to load the config file at $MPV_HOME
    let load_config: bool = ask_load_config();

    let settings = Settings {
      server_config,
      discord_presence,
      transcoding,
      fullscreen,
      autologin,
      autoplay,
      gpu,
      load_config,
      glsl_shader: None,
      mpv_debug: None,
      mpv_config_location: None
    };
    let settings_file = toml::to_string_pretty(&settings).unwrap();
    std::fs::write(config_path_string, settings_file).expect("Saving settings.");
    settings
  } else {
    let settings_file_raw = Config::builder().add_source(File::from(Path::new(&config_path_string))).build().unwrap();
    let serialized = settings_file_raw.try_deserialize::<Settings>();
    match serialized {
      Ok(settings) => {
        settings
      },
      Err(e) => {
        if e.to_string().contains("missing field") {
          println!("{}", "Settings file is corrupt. Attempting to fix it ...".to_string().red());
          let mut settings_file = fs::OpenOptions::new().write(true).append(true).open(&config_path_string).unwrap();
          match &e.to_string()[e.to_string().find('`').unwrap() + 1..e.to_string().len() - 1] {
            "server_config" => {
              let server_config: Option<String> = ask_search_server_configs();
              write!(settings_file, "server_config = {server_config:?}\n").unwrap();
              let settings = read_settings();
              return settings;
            },
            "discord_presence" => {
              let discord_presence: bool = ask_initiate_discord();
              write!(settings_file, "discord_presence = {discord_presence}\n").unwrap();
              let settings = read_settings();
              return settings;
            },
            "transcoding" => {
              let transcoding: bool = ask_transcoding();
              write!(settings_file, "transcoding = {transcoding}\n").unwrap();
              let settings = read_settings();
              return settings;
            },
            "fullscreen" => {
              let fullscreen: bool = ask_start_fullscreen();
              write!(settings_file, "fullscreen = {fullscreen}\n").unwrap();
              let settings = read_settings();
              return settings;
            },
            "autologin" => {
              let autologin: bool = ask_automatically_login();
              write!(settings_file, "autologin = {autologin}\n").unwrap();
              let settings = read_settings();
              return settings;
            },
            "autoplay" => {
              let autoplay: bool = ask_autoplay();
              write!(settings_file, "autoplay = {autoplay}\n").unwrap();
              let settings = read_settings();
              return settings;
            },
            "gpu" => {
              let gpu: bool = ask_gpu();
              write!(settings_file, "gpu = {gpu}\n").unwrap();
              let settings = read_settings();
              return settings;
            }
            "load_config" => {
              let load_config: bool = ask_load_config();
              write!(settings_file, "load_config = {load_config}\n").unwrap();
              let settings = read_settings();
              return settings;
            }
            _ => {
              println!("{}", "Failure.".to_string().red())
            }
          }
        } else {
          println!("{}", "Settings file is corrupt. Settings have to be reconfigured.\n".to_string().red());
        }
        let server_config: Option<String> = ask_search_server_configs();
        let discord_presence: bool = ask_initiate_discord();
        let transcoding: bool = ask_transcoding();
        let fullscreen: bool = ask_start_fullscreen();
        let autologin: bool = ask_automatically_login();
        let autoplay: bool = ask_autoplay();
        let gpu: bool = ask_gpu();
        let load_config: bool = ask_load_config();
        let settings = Settings {
          server_config,
          discord_presence,
          transcoding,
          fullscreen,
          autologin,
          autoplay,
          gpu,
          load_config,
          glsl_shader: None,
          mpv_debug: None,
          mpv_config_location: None
        };
        let settings_file = toml::to_string_pretty(&settings).unwrap();
        std::fs::write(config_path_string, settings_file).expect("Saving settings.");
        settings
      }
    }
  }
}


pub fn initialize_settings(mode: u8) -> Settings {
  // Modes
  //  0 -> read settings
  //  1 -> change settings
  //  2 -> display settings
  let mut settings: Settings = read_settings();
  if mode == 1 {
    settings = change_settings(settings);
  } else if mode == 2 {
    display_settings(&settings);
  };
  settings
}


fn ask_initiate_discord() -> bool {
  print!("Do you want to activate Discord-Presence by default?\n (Y)es / (N)o");
  let presence = getch("YyNn");
  let connection: bool = match presence {
    'Y' | 'y' => {
      true
    },
    'N' | 'n' => {
      false
    },
    _ => false
  };
  connection
}


fn ask_search_server_configs() -> Option<String> {
  let config_path = dirs::config_dir().unwrap().join(APPNAME.to_lowercase());
  println!("Searching in \"{}\" for emby or jellyfin configuration files ...", &config_path.display());
  if fs::read_dir(&config_path).is_err() {
    fs::create_dir_all(&config_path).expect("Could not create config directory!")
  };
  let path: Vec<_> = fs::read_dir(&config_path).unwrap().map(|r| r.unwrap()).collect();
  let mut files: Vec<String> = [].to_vec();
  for file in &path {
    if file.path().is_dir() {
      let depth2: Vec<_> = fs::read_dir(&file.path()).unwrap().map(|r| r.unwrap()).collect();
      for stuff in depth2 {
        let file_path: String = stuff.path().display().to_string();
        if file_path.contains(".config.json") {
          files.append(&mut [file_path].to_vec());  
        } else {
          continue
        }
      }
    }
    let file_path: String = file.path().display().to_string();
    if file_path.contains(".config.json") {
      files.append(&mut [file_path].to_vec());
    } else {
      continue
    }
  };
  if files.is_empty() {
    println!("No configuration has been found.\n");
    return None
  } else {
    for (index, path) in files.iter().enumerate() {
      println!("  [{index}] {path}");
    }
  }
  print!("Select which one of the above server configs should be used by default, or skip with \"None\".\n: ");
  io::stdout().flush().expect("Failed to flush stdout");
  let mut selection = String::new();
  io::stdin().read_line(&mut selection).unwrap();
  if selection.trim() == "None" {
    println!("Skipped default-server option.\n");
    return None
  }
  let num_selection: usize = selection.trim().parse().unwrap();
  println!("You've picked {}.\n", format!("{:?}", files[num_selection]).green());
  Some(files[num_selection].to_string())
}


fn ask_transcoding() -> bool {
  print!("Do you want to transcode the video to hevc to save bandwidth?\n  (e.g.: if the emby/jellyfin instance isn't running locally)\n (Y)es / (N)o");
  let encode = getch("YyNn");
  match encode {
    'Y' | 'y' => {
      true
    },
    'N' | 'n' => {
      false
    },
    _ => false
  }
}


fn ask_start_fullscreen() -> bool {
  print!("Do you want mpv to start in fullscreen-mode?\n (Y)es / (N)o");
  let fullscreen = getch("YyNn");
  match fullscreen {
    'Y' | 'y' => {
      true
    },
    'N' | 'n' => {
      false
    },
    _ => false
  }
}


fn change_settings(mut settings: Settings) -> Settings {
  let config_path = dirs::config_dir().unwrap();
  let config_path_string = format!("{}/{}/{}.toml", &config_path.display().to_string(), APPNAME.to_lowercase(), APPNAME);
  loop {
    print!("Which settings do you want to change?
  [1] Default server configuration = {}
  [2] Discord presence = {}
  [3] Transcoding = {}
  [4] MPV fullscreen = {}
  [5] Automatically login = {}
  [6] Autoplay = {}
  [7] Hardware decoding = {}
  [8] Load MPV config and lua scripts = {}
\n  [S] Save and return to the menu",
settings.server_config.as_ref().unwrap_or(&"None".to_string()).to_string().green(),
settings.discord_presence.to_string().green(),
settings.transcoding.to_string().green(),
settings.fullscreen.to_string().green(),
settings.autologin.to_string().green(),
settings.autoplay.to_string().green(),
settings.gpu.to_string().green(),
settings.load_config.to_string().green()
    );
    let menu = getch("12345678Ss");
    match menu {
      '1' => {
        settings.server_config = ask_search_server_configs();
      },
      '2' => {
        settings.discord_presence = ask_initiate_discord();
      },
      '3' => {
        settings.transcoding = ask_transcoding();
      },
      '4' => {
        settings.fullscreen = ask_start_fullscreen();
      },
      '5' => {
        settings.autologin = ask_automatically_login();
      },
      '6' => {
        settings.autoplay = ask_autoplay();
      },
      '7' => {
        settings.gpu = ask_gpu();
      },
      '8' => {
        settings.load_config = ask_load_config();
      }
      'S' | 's' => {
        break
      },
      _ => (
      )
    };
  }
  let settings_file = toml::to_string_pretty(&settings).unwrap();
  std::fs::write(config_path_string, settings_file).expect("Saving settings failed.");
  settings
}


fn display_settings(settings: &Settings) {
  println!("  Default server configuration = {}
  Discord presence = {}
  Transcoding = {}
  MPV fullscreen = {}
  Automatically login = {}
  Autoplay = {}
  Hardware decoding = {}
  Load MPV config and lua scripts = {}
",
  settings.server_config.as_ref().unwrap_or(&"None".to_string()).to_string().green(),
  settings.discord_presence.to_string().green(),
  settings.transcoding.to_string().green(),
  settings.fullscreen.to_string().green(),
  settings.autologin.to_string().green(),
  settings.autoplay.to_string().green(),
  settings.gpu.to_string().green(),
  settings.load_config.to_string().green()
  );
}


fn ask_automatically_login() -> bool {
  print!("Do you want to enable autologin on start?\n (Y)es / (N)o");
  let autologin = getch("YyNn");
  match autologin {
    'Y' | 'y' => {
      true
    },
    'N' | 'n' => {
      false
    },
    _ => false
  }
}

fn ask_autoplay() -> bool {
  print!("Do you want to enable autoplay for episodes?\n(You can only exit by CTRL+C)\n (Y)es / (N)o");
  let autologin = getch("YyNn");
  match autologin {
    'Y' | 'y' => {
      true
    },
    'N' | 'n' => {
      false
    },
    _ => false
  }
}

fn ask_gpu() -> bool {
  print!("Do you want to enable hardware decoding for MPV?\n(Using \"auto-safe\" api)\n (Y)es / (N)o");
  let autologin = getch("YyNn");
  match autologin {
    'Y' | 'y' => {
      true
    },
    'N' | 'n' => {
      false
    },
    _ => false
  }
}

fn ask_load_config() -> bool {
  print!("Do you want to load the mpv config-file?\n(including lua scripts)\n (Y)es / (N)o");
  let load_config = getch("YyNn");
  match load_config {
    'Y' | 'y' => {
      true
    },
    'N' | 'n' => {
      false
    },
    _ => false
  }
}
