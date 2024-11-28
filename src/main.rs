#![allow(non_snake_case)]
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use puddler_settings::PuddlerSettings;
use std::process::{exit, ExitCode};

use crate::{
  input::{interactive_menuoption, interactive_select, InteractiveOption, InteractiveOptionType},
  media_center::{set_config, MediaCenter},
  media_config::{Config, MediaCenterType, Objective},
  printing::{print_message, PrintMessageType},
};

const APPNAME: &str = "Puddler";
const VERSION: &str = env!("CARGO_PKG_VERSION");

mod discord;
mod emby;
mod error;
mod input;
mod jellyfin;
mod media_center;
mod media_config;
mod mpv;
mod plex;
mod printing;
mod puddler_settings;

#[derive(Debug, Clone)]
pub enum MenuOptions {
  Default(String),
  Choose,
  Setup,
  Setting,
  Exit,
}

impl ToString for MenuOptions {
  fn to_string(&self) -> String {
    match self {
      MenuOptions::Default(message) => {
        if message.is_empty() {
          String::from("Stream from default Media-Center")
        } else {
          message.clone()
        }
      },
      MenuOptions::Choose => String::from("View Media-Centers"),
      MenuOptions::Setup => String::from("Add new Media-Center"),
      MenuOptions::Setting => String::from("Settings"),
      MenuOptions::Exit => String::from("Exit puddler"),
    }
  }
}

fn main() -> ExitCode {
  let command = Command::new("puddler")
    .display_name("Puddler")
    .about("A simplistic command-line client for Jellyfin, Emby and Plex.")
    .version(VERSION)
    .arg(
      Arg::new("glsl-shaders")
        .long("glsl-shader")
        .help("Play MPV using this shader-file.")
        .required(false)
        .action(ArgAction::Set)
        .num_args(1..),
    )
    .arg(
      Arg::new("debug")
        .long("debug")
        .help("Print MPV log messages to \"./mpv.log\".")
        .required(false)
        .action(ArgAction::SetTrue),
    )
    .get_matches();

  let mut settings: PuddlerSettings = PuddlerSettings::new().unwrap();

  if command.get_many::<String>("glsl-shaders").is_some() {
    settings.glsl_shaders = command
      .get_many::<String>("glsl-shaders")
      .unwrap()
      .map(|sh| sh.to_string())
      .collect();
  }

  if command.get_flag("debug") {
    settings.mpv_debug_log = true;
  }

  let mut options: Vec<MenuOptions> = vec![];

  if let Some(ref default_server) = settings.default_media_server {
    let mut handle = Config::default();
    handle.path = default_server.to_string();
    if let Ok(()) = handle.read() {
      options.append(&mut vec![MenuOptions::Default(format!(
        "{} - {}",
        handle.config.server_name,
        handle.config.media_center_type.to_string()
      ))]);
    }
  }

  options.append(&mut vec![
    MenuOptions::Choose,
    MenuOptions::Setup,
    MenuOptions::Setting,
    MenuOptions::Exit,
  ]);

  println!(
    "{}",
    r"      ____            __    ____
     / __ \__  ______/ /___/ / /__  _____
    / /_/ / / / / __  / __  / / _ \/ ___/
   / ____/ /_/ / /_/ / /_/ / /  __/ /
  /_/    \__,_/\__,_/\__,_/_/\___/_/"
      .to_string()
      .bright_cyan()
  );

  println!();
  loop {
    let mut handle = Config::default();
    let path: String;
    let mut center: Box<dyn MediaCenter>;
    match interactive_menuoption(options.clone()) {
      MenuOptions::Default(_) => {
        path = settings.default_media_server.clone().unwrap();
        handle.path = path;
        if let Err(e) = handle.read() {
          print_message(
            PrintMessageType::Error,
            format!("Failed reading config file: {:?}", e).as_str(),
          );
          continue;
        }
      },
      MenuOptions::Choose => {
        if let Ok(configs) = handle.new() {
          if configs.is_empty() {
            // print_message(PrintMessageType::Error, "No pre-existing config file found. Please add a new media-center.\n");
            continue;
          }
          let mut configs_str: Vec<InteractiveOption> = vec![];
          for config in configs.clone() {
            configs_str.append(&mut vec![InteractiveOption {
              text: format!(
                "{} - {}:Modify",
                config.config.server_name,
                config.config.media_center_type.to_string()
              ),
              option_type: InteractiveOptionType::MultiButton,
            }]);
          }
          configs_str.append(&mut vec![InteractiveOption {
            text: String::from("Back"),
            option_type: InteractiveOptionType::Special,
          }]);
          match interactive_select(configs_str) {
            ((i1, i2), _, InteractiveOptionType::MultiButton) => {
              handle = configs.get(i1).unwrap().clone();
              if i2 == 1 {
                center = set_config(handle, settings.clone());
                center.modify();
                continue;
              }
            },
            (_, Some(option), InteractiveOptionType::Special) => {
              if option == *"Back" {
                continue;
              }
            },
            _ => (),
          }
        } else {
          exit(1);
        }
      },
      MenuOptions::Setting => {
        settings.change_menu();
        continue;
      },
      MenuOptions::Exit => return ExitCode::SUCCESS,
      MenuOptions::Setup => {
        handle.ask_for_setting(Objective::MediaCenterType);
        match handle.config.media_center_type {
          MediaCenterType::Plex => (),
          _ => handle.ask_for_setting(Objective::SearchLocalInstance),
        }
      },
    };

    center = set_config(handle, settings.clone());

    center.re_authenticate();
    center.menu();
  }
}
