// This part of puddler parses and writes the emby and jellyfin config files
use std::{
  fs,
  path::Path,
  io::{
    stdout,
    stdin,
    prelude::*
  }
};
use colored::Colorize;
use regex::Regex;

use crate::{
  APPNAME,
  is_numeric,
  mediaserver_information::{
    ConfigFileRaw,
    ConfigFile,
    ConfigFileUser,
    getch
  }
};


pub fn choose_config(server_kind: char, autologin: bool) -> Option<String> {
  let folder_suffix = if server_kind == '1' {
    "emby"
  } else {
    "jellyfin"
  };
  let app_root = dirs::config_dir().unwrap();
  let config_path_string = format!("{}/{}/{}", app_root.display(), APPNAME.to_lowercase(), folder_suffix);
  let config_path = Path::new(config_path_string.as_str());
  if ! config_path.exists() {
    fs::create_dir(config_path).expect("Couldn't create config directory. Check your permissions!");
  };
  let mut files = fs::read_dir(config_path).expect("Couldn't read config directory. Check your permissions!");
  let file_count = fs::read_dir(config_path).expect("Couldn't read config directory. Check your permissions!").count();
  if file_count == 0 {
    None
  } else if file_count == 1 {
    let file = files.next().expect("impossible").unwrap().path();
    if file.display().to_string().ends_with(".config.json") {
      Some(file.display().to_string())
    } else {
      None
    }
  } else {
    let mut file_list: Vec<String> = [].to_vec();
    for item in files {
      let file = item.expect("impossible part 2").path();
      if file.display().to_string().ends_with(".config.json") {
        file_list.append(&mut [file.display().to_string()].to_vec());
      }
    };
    if autologin {
      return Some(file_list.first().unwrap().to_string())
    };
    let copy = file_list.clone();
    println!("Please choose which configuration file you want to use.");
    for entry in &file_list {
      println!("  [{}] {}", &copy.iter().position(|y| y == entry).unwrap(), &entry)
    };
    print!(": ");
    let index: usize;
    loop {
      stdout().flush().expect("Failed to flush stdout");
      let mut index_raw: String = String::new();
      stdin().read_line(  &mut index_raw).unwrap();
      index_raw.trim().parse::<String>().unwrap();
      if ! is_numeric(&index_raw) {
        print!("Invalid input, please try again.\n: ")
      } else {
        index = index_raw.trim().parse::<usize>().unwrap();
        break
      }
    }
    println!();
    Some(file_list.get(index).unwrap().to_string())
  }
}


pub fn read_config(config_path_string: &String, autologin: bool) -> Result<(ConfigFile, ConfigFileRaw), (Option<ConfigFileRaw>, &str)> {
  let file = fs::read_to_string(config_path_string).unwrap();
  let local_config_file: Result<ConfigFileRaw, serde_json::Error> = serde_json::from_str::<ConfigFileRaw>(&file);
  match local_config_file {
    Ok(a) => {
      let user: &ConfigFileUser;
      let media_server_name: &str = if a.emby {
        "Emby"
      } else {
        "Jellyfin"
      };
      let reg: Regex = Regex::new(r"([^(/|\\)]+)(?:\.[a-zA-Z0-9]+\.config\.json)").unwrap();
      let server_name: &str = match reg.captures(config_path_string) {
        Some(yay) => {
          yay.get(1).map_or("", |m| m.as_str())
        },
        None => {
          "Host"
        }
      };
      if autologin {
        Ok((ConfigFile {
          emby: a.emby,
          server_name: server_name.to_string(),
          ipaddress: a.ipaddress.clone(),
          device_id: a.device_id.clone(),
          user_id: a.user.first().unwrap().user_id.to_string(),
          access_token: a.user.first().unwrap().access_token.to_string(),
          username: a.user.first().unwrap().username.to_string()
        }, a))
      } else {
        print!("Do you want to use this config?\n   {} ({}): {}\n   Username: {}\n (Y)es / (N)o", server_name.green(), media_server_name, a.ipaddress, a.user.first().unwrap().username);
        let input = getch("YyNn");
        stdout().flush().expect("Failed to flush stdout");
        if "yY".contains(input) {
          user = a.user.first().unwrap();
          Ok((ConfigFile {
            emby: a.emby,
            server_name: server_name.to_string(),
            ipaddress: a.ipaddress.clone(),
            device_id: a.device_id.clone(),
            user_id: user.user_id.clone(),
            access_token: user.access_token.clone(),
            username: user.username.clone()
          }, a))
        } else {
          print!("Please choose from the following options:\n   [1] Switch to a different {media_server_name}-user\n   [2] Switch to a different {media_server_name}-server");
          let hngfje = getch("12");
          match hngfje {
            '1' => {
              if a.user.len() == 1 {
                return Err((Some(a), "add user"))
              }
              println!("Please choose which user you want to switch to.\n(\"Add\" if you want to add a new user)");
              for (index, thing) in a.user.iter().enumerate() {
                println!("  [{}] {}", &index, thing.username);
              }
              print!(": ");
              stdout().flush().expect("Failed to flush stdout");
              let index: usize;
              loop {
                let mut index_raw: String = String::new();
                stdin().read_line(  &mut index_raw).unwrap();
                index_raw.trim().parse::<String>().unwrap();
                if ! is_numeric(&index_raw) {
                  if index_raw.trim() == "Add" {
                    return Err((Some(a), "add user"))
                  } else {
                    print!("Invalid input, please try again.\n: ");
                    stdout().flush().expect("Failed to flush stdout");
                  }
                } else {
                  index = index_raw.trim().parse().unwrap();
                  break
                }
              };
              println!();
              user = a.user.get(index).unwrap();
              Ok((ConfigFile {
                emby: a.emby,
                server_name: server_name.to_string(),
                ipaddress: a.ipaddress.clone(),
                device_id: a.device_id.clone(),
                user_id: user.user_id.clone(),
                access_token: user.access_token.clone(),
                username: user.username.clone()
              }, a))
            },
            '2' => {
              Err((None, "add server"))
            }
            _ => {
              Err((None, "lol"))
            }
          }
          
        }
      }
    },
    Err(_) => {
      println!("Config seems to be faulty.");
      Err((None, "faulty"))
    }
  }
}


pub fn write_config(config_path_string: String, config_file: &ConfigFile, other_users: Option<Vec<ConfigFileUser>>) {
  let config_file_user = ConfigFileUser {
    user_id: config_file.user_id.clone(),
    access_token: config_file.access_token.clone(),
    username: config_file.username.clone()
  };
  let config_file_raw = if let Some(mut other_users) = other_users {
    let mut user_vec: Vec<ConfigFileUser> = [config_file_user].to_vec();
    user_vec.append(&mut other_users);
    ConfigFileRaw {
      emby: config_file.emby,
      ipaddress: config_file.ipaddress.clone(),
      device_id: config_file.device_id.clone(),
      user: user_vec
    }
  } else {
    ConfigFileRaw {
      emby: config_file.emby,
      ipaddress: config_file.ipaddress.clone(),
      device_id: config_file.device_id.clone(),
      user: [config_file_user].to_vec()
    }
  };
  let result = fs::write(config_path_string, serde_json::to_string_pretty(&config_file_raw).unwrap());
  match result {
    Ok(()) => println!("Saved to config file ..."),
    Err(_e) => panic!("write access??")
  }
}


pub fn generate_config_path(server_kind: char, server_id: String, server_name: String) -> String {
  let folder_suffix = if server_kind == '1' {
    "emby"
  } else {
    "jellyfin"
  };
  let app_root = dirs::config_dir().unwrap();
  format!("{}/{}/{}/{}.{}.config.json", app_root.display(), APPNAME.to_lowercase(), folder_suffix, server_name, server_id)
}
