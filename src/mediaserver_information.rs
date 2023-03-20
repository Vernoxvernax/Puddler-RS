#![allow(non_snake_case)]
extern crate getch;
use std::char;
use std::io;
use std::io::prelude::*;
use std::time::Duration;
use std::net::UdpSocket;
use std::str::from_utf8;
use std::result::Result;
use std::fmt::Debug;
use uuid;
use app_dirs::*;
use http::Response;
use http::StatusCode;
use colored::Colorize;
use isahc::Body;
use isahc::Request;
use isahc::prelude::*;
use rpassword::read_password;
use serde_json::Value;
use serde_derive::{Deserialize,Serialize};

use crate::APPNAME;
use crate::VERSION;
use crate::APP_INFO;
use crate::settings::Settings;
use crate::config::*;


#[derive(Debug)]
pub struct HeadDict {
  pub media_server_name: String,
  pub media_server: String,
  pub config_file: ConfigFile,
  pub auth_header: AuthHeader,
  pub request_header: RequestHeader,
  pub session_id: String
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigFile {
  pub emby: bool,
  pub server_name: String,
  pub ipaddress: String,
  pub device_id: String,
  pub user_id: String,
  pub access_token: String,
  pub username: String
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigFileRaw {
  pub emby: bool,
  pub ipaddress: String,
  pub device_id: String,
  pub user: Vec<ConfigFileUser>
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigFileUser {
  pub user_id: String,
  pub access_token: String,
  pub username: String
}


#[derive(Debug, Deserialize, Serialize)]
pub struct UserLogin {
  pub username: String,
  pub pw: String
}


#[derive(Debug)]
pub struct RequestHeader {
  pub application: String,
  pub token: String
}


#[derive(Debug)]
pub struct AuthHeader {
  pub authorization: String,
}


pub fn getch(allowed: &str) -> char {
  let output: char;
  loop {
    print!("\n: ");
    io::stdout().flush().expect("Failed to flush stdout");
    let ch: char = getch::Getch::new().getch().unwrap() as char;
    if allowed.contains(ch) {
      if ch == '\n' {
        println!("\n");
      } else {
        println!("{ch}\n");
      }
      output = ch;
      break
    } else if ch == '\r' {
      println!("\n");
      output = '\n';
      break
    } else {
      print!("\nInvalid input, please try again.")
    }
  }
  output
}


pub fn check_information(settings: &Settings) -> Option<HeadDict> {
  let media_server: &str;
  let emby: bool;
  let media_server_name: &str;
  let mut auth_header: AuthHeader;
  let device_id = uuid::Uuid::new_v4().to_string();
  let server_kind = if settings.server_config.is_none() {
    print!("What kind of server do you want to stream from?\n   [1] Emby\n   [2] Jellyfin");
    getch("12")
  } else {
    match read_config(settings.server_config.as_ref().unwrap(), true) {
      Ok((config, _raw)) => {
        if config.emby {
          '1'
        } else {
          '2'
        }
      },
      Err(_no) => {
        print!("What kind of server do you want to stream from?\n   [1] Emby\n   [2] Jellyfin");
        getch("12")
      }
    }
  };
  match server_kind {
    '1' => {
      emby = true;
      media_server = "/emby";
      media_server_name = "Emby";
      auth_header = AuthHeader {
        authorization: format!("Emby UserId=\"\", Client=\"Emby Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\", Token=\"\"",
        APPNAME, device_id, VERSION)
      };
    }
    _ => {
      emby = false;
      media_server = "";
      media_server_name = "Jellyfin";
      auth_header = AuthHeader {
        authorization: format!("Emby UserId=\"\", Client=\"Emby Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\", Token=\"\"",
        APPNAME, device_id, VERSION)
      };
    }
  };
  let config_path: Option<String> = if settings.server_config.is_none() {
    choose_config(server_kind, settings.autologin)
  } else {
    settings.server_config.clone()
  };
  let request_header: RequestHeader;
  let session_id: String;
  let user_id: String;
  let access_token: String;
  let server_id: String;
  let mut device_id = uuid::Uuid::new_v4().to_string();
  let config_file: ConfigFile;
  if let Some(config_path_string) = config_path {
    println!("{}", "Configuration files found!".to_string().green());
    let config_file_raw: Result<(ConfigFile, ConfigFileRaw), (Option<ConfigFileRaw>, &str)> = read_config(&config_path_string, settings.autologin);
    match config_file_raw {
      Ok((mut file, mut raw_file)) => {
        let ipaddress = &file.ipaddress;
        device_id = file.device_id.clone();
        auth_header = AuthHeader {
          authorization: format!("Emby UserId={}, Client=\"Emby Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\", Token={}", &file.user_id, APPNAME, device_id, VERSION, &file.access_token)
        };
        if file.server_name != "Host" {
          println!("Logging in with {} on {}.", file.username.green(), file.server_name.green());
        } else {
          println!("Logging in with {}.", file.username.green());
        };
        let session_id_test = re_auth(media_server_name, media_server, ipaddress, &auth_header, &device_id);
        if let Ok(res) = session_id_test {
          if raw_file.user[0].username != file.username {
            let mut i = 0;
            while i < raw_file.user.len() {
              if raw_file.user[i].username == file.username {
                println!("Set {} as the default user.", raw_file.user.remove(i).username.green());
              } else {
                i += 1;
              }
            };
            write_config(config_path_string, &file, Some(raw_file.user));
          };
          request_header = get_request_header(&file.access_token);
          session_id = res;
        } else if session_id_test.as_ref().unwrap_err() == &"exp".to_string() {
          println!("\nYour {media_server_name} session expired! Please re-login.");
          let user_login = configure_new_login(media_server_name);
          let auth = test_auth(media_server_name, media_server, ipaddress, &auth_header, &user_login, &device_id);
          if let Some(pyld) = auth {
            auth_header = pyld.0;
            request_header = pyld.1;
            session_id = pyld.2;
            access_token = pyld.4;
          } else {
            return None;
          }
          file.access_token = access_token;
          let mut i = 0;
          while i < raw_file.user.len() {
            if raw_file.user[i].username == file.username {
              println!("Replaced {} in the config file.", raw_file.user.remove(i).username.green());
            } else {
              i += 1;
            }
          }
          write_config(config_path_string, &file, Some(raw_file.user));
        } else {
          println!("{}\n  Error: {}", "Failed to establish a working connection!".to_string().red(), session_id_test.unwrap_err());
          return None;
        }
        config_file = ConfigFile {
          emby,
          server_name: file.server_name,
          device_id,
          ipaddress: ipaddress.to_string(),
          user_id: file.user_id,
          access_token: file.access_token,
          username: file.username
        };
      },
      Err((Some(mut file), "add user")) => {
        let ipaddress = file.ipaddress;
        let user_login = configure_new_login(media_server_name);
        let auth = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
        if let Some(pyld) = auth {
          auth_header = pyld.0;
          request_header = pyld.1;
          session_id = pyld.2;
          user_id = pyld.3;
          access_token = pyld.4;
        } else {
          return None;
        }
        config_file = ConfigFile {
          emby,
          server_name: "Host".to_string(),
          device_id,
          ipaddress,
          user_id,
          access_token,
          username: user_login.username
        };
        let mut i = 0;
        while i < file.user.len() {
          if file.user[i].username == config_file.username {
            println!("Replaced {}.", file.user.remove(i).username.green());
          } else {
            i += 1;
          }
        };
        write_config(config_path_string, &config_file, Some(file.user));
      },
      Err((None, "add server")) => {
        loop {
          let (ipaddress, server_name) = configure_new_server(media_server_name);
          let user_login = configure_new_login(media_server_name);
          let auth = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
          if let Some(pyld) = auth {
            auth_header = pyld.0;
            request_header = pyld.1;
            session_id = pyld.2;
            user_id = pyld.3;
            access_token = pyld.4;
            server_id = pyld.5
          } else {
            continue;
          }
          config_file = ConfigFile {
            emby,
            server_name: server_name.clone(),
            device_id,
            ipaddress,
            user_id,
            access_token,
            username: user_login.username
          };
          let config_path_string = generate_config_path(server_kind, server_id, server_name);
          write_config(config_path_string, &config_file, None);
          break;
        }
      },
      _ => {
        let (ipaddress, server_name) = configure_new_server(media_server_name);
        let user_login = configure_new_login(media_server_name);
        device_id = uuid::Uuid::new_v4().to_string();
        let auth = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
        if let Some(pyld) = auth {
          auth_header = pyld.0;
          request_header = pyld.1;
          session_id = pyld.2;
          user_id = pyld.3;
          access_token = pyld.4;
        } else {
          return None;
        }
        config_file = ConfigFile {
          emby,
          server_name,
          device_id,
          ipaddress,
          user_id,
          access_token,
          username: user_login.username
        };
        write_config(config_path_string, &config_file, None);
      }
    }
  } else {
    app_root(AppDataType::UserConfig, &APP_INFO).expect("shit");
    let (ipaddress, server_name) = configure_new_server(media_server_name);
    let user_login = configure_new_login(media_server_name);
    let auth = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
    if let Some(pyld) = auth {
      auth_header = pyld.0;
      request_header = pyld.1;
      session_id = pyld.2;
      user_id = pyld.3;
      access_token = pyld.4;
      server_id = pyld.5;
    } else {
      return None;
    }
    config_file = ConfigFile { 
      emby,
      server_name: server_name.clone(),
      ipaddress,
      user_id,
      device_id,
      access_token,
      username: user_login.username
    };
    let config_path_string = generate_config_path(server_kind, server_id, server_name);
    write_config(config_path_string, &config_file, None);
  }
  Some(HeadDict {
    media_server_name: media_server_name.to_string(),
    media_server: media_server.to_string(),
    config_file,
    auth_header,
    request_header,
    session_id
  })
}


fn configure_new_server(media_server_name: &str) -> (String, String) {
  let mut ipaddress: String;
  let mut server_name: String;
  let who_is = if media_server_name == "Emby" {
    "who is EmbyServer?"
  } else {
    "who is JellyfinServer?"
  };
  println!("Searching for local media-server...");
  let socket:UdpSocket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to network socket.");
  socket.set_read_timeout(Some(Duration::new(5, 0))).expect("nothing");
  socket.set_broadcast(true).expect("errrrrrr");
  socket.send_to(&String::from(who_is).into_bytes(), "255.255.255.255:7359").expect("fdsfds");
  let mut buf  = [0; 4096];
  let udp_disco = socket.recv_from(&mut buf);
  match udp_disco {
    Ok(_t) => {
      let parsed: Udp = byte_array_to_json(buf);
      ipaddress = parsed.Address;
      server_name = parsed.Name;
      print!("Is the {} at the following address the correct one?\n \"{}\"\n (Y)es / (N)o", server_name.green(), ipaddress);
      let udp_question = getch("YyNn");
      match udp_question {
        'Y'|'y' => {
          println!("Nice, already done.");

        },
        'N'|'n' => {
          print!("Please specify the IP-Address manually\n(don't forget to add ports if not running on 80/443!)\n: ");
          io::stdout().flush().expect("Failed to flush stdout");
          let mut ipaddress2 = String::new();
          io::stdin().read_line(  &mut ipaddress2).unwrap();
          ipaddress = ipaddress2.trim().parse().unwrap();
          print!("\nPlease enter a nickname for your media-server.\n(It's recommended to use a unique one)\n: ");
          io::stdout().flush().expect("Failed to flush stdout");
          server_name = String::new();
          io::stdin().read_line(  &mut server_name).unwrap();
          server_name = server_name.trim().parse().unwrap();
        }
        _ => (),
      }
    },
    Err(_e) => {
      print!("Couldn't find any local media-server.\nIf your instance is running under a docker environment, configure the host network-option.\nOr just specify the IP-Address manually. (don't forget to add ports)\n: ");
      io::stdout().flush().expect("Failed to flush stdout");
      let mut ipaddress2 = String::new();
      io::stdin().read_line(  &mut ipaddress2).unwrap();
      ipaddress = ipaddress2.trim().parse().unwrap();
      print!("\nPlease enter a nickname for your media-server.\n(It's recommended to use a unique one)\n: ");
      io::stdout().flush().expect("Failed to flush stdout");
      server_name = String::new();
      io::stdin().read_line(  &mut server_name).unwrap();
      server_name = server_name.trim().parse().unwrap();
    },
  }
  if ! ipaddress.contains("http") {
    ipaddress = format!("http://{ipaddress}");
  }
  if ipaddress.ends_with('/') {
    ipaddress.pop();
  }
  (ipaddress, server_name)
}


#[derive(Serialize, Deserialize)]
struct Udp {
  Address: String,
  Id: String,
  Name: String,
}


fn byte_array_to_json(buf: [u8; 4096]) -> Udp {
  let response = from_utf8(&buf).expect("sos").trim_matches(char::from(0));
  serde_json::from_str(response).unwrap()
}


fn configure_new_login(media_server_name: &str) -> UserLogin {
  fn take_input(media_server_name: &str) -> (String, String) {
    let mut username = String::new();
    print!("\nPlease enter your {media_server_name} username: ");
    io::stdout().flush().expect("Failed to flush stdout");
    io::stdin().read_line(  &mut username).unwrap();
    print!("Please enter your {media_server_name} password (hidden): ");
    io::stdout().flush().expect("Failed to flush stdout");
    let password = read_password().unwrap();
    println!();
    (password.trim().parse().unwrap(), username.trim().parse().unwrap())
  }
  let mut repeat: bool = true;
  let mut password: String= "".to_string();
  let mut username: String= "".to_string();
  while repeat {
    (password, username) = take_input(media_server_name);
    print!("Do you want to confirm your input?\n (Y)es / (N)o");
    let fgndjk = getch("yYNn");
    match fgndjk {
      'Y' | 'y' => repeat = false,
      'N' | 'n' => continue,
      _ => ()
    }
  }
  UserLogin {
    username,
    pw: password
  }
}


fn test_auth (media_server_name: &str, media_server: &str, ipaddress: &String, auth_header: &AuthHeader, user_login: &UserLogin, device_id: &String) -> Option<(AuthHeader, RequestHeader, String, String, String, String)> {
  println!("Testing {media_server_name} connection ...");
  let username: String = user_login.username.clone();
  let password: String = user_login.pw.clone();
  let bod = format!("{{\"Username\":\"{username}\",\"pw\":\"{password}\"}}");
  let url = format!("{ipaddress}{media_server}/Users/AuthenticateByName");
  let json_response = post_puddler(url, auth_header, bod);
  match json_response {
    Ok(mut t) => {
      println!("{}", "Connection successfully established!".to_string().green());
      let json_response = t.json::<Value>().unwrap();
      let server_id = json_response.get("ServerId").unwrap();
      let session_obj = json_response.get("SessionInfo").unwrap();
      let user_id = session_obj["UserId"].as_str().unwrap();
      let session_id = session_obj["Id"].as_str().unwrap();
      let token = json_response["AccessToken"].as_str().unwrap();
      Some((
        AuthHeader {
          authorization: format!("Emby UserId={}, Client=\"Emby Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\", Token={}",
          user_id, APPNAME, device_id, VERSION, token)
        },
        RequestHeader {
          application: format!("{APPNAME}/{VERSION}"),
          token: token.to_string()
        },
        session_id.to_string(),
        user_id.to_string(),
        token.to_string(),
        server_id.as_str().unwrap().trim().to_string()
      ))
    },
    Err(e) => {
      println!("{}\n  Error: {}", "Failed to establish a working connection!".to_string().red(), e);
      None
    }
  }
}


pub fn post_puddler(url: String, auth_header: &AuthHeader, bod: String) -> Result<Response<Body>, String> {
  let mut response = Request::post(url)
    .header("Authorization", &auth_header.authorization)
    .header("Content-Type", "application/json")
    .body(bod).unwrap()
    .send().unwrap();
  let result = match response.status() {
    StatusCode::OK => {
      response
    },
    _ => {
      return Err(response.text().unwrap());
    }
  };
  Ok(result)
}


fn re_auth(media_server_name: &str, media_server: &str, ipaddress: &String, auth_header: &AuthHeader, device_id: &String) -> Result<String, String> {
  println!("Testing {media_server_name} connection ...");
  let re_auth_res = smol_puddler_get(format!("{}{}/Sessions?DeviceId={}", ipaddress, media_server, &device_id), auth_header);
  match re_auth_res {
    Ok(mut t) => {
      let response_text: &String = &t.text().unwrap();
      if let Ok(re_auth_json) = serde_json::from_str::<Value>(response_text) {
        println!("{}", "Connection successfully reestablished!".to_string().green());
        if re_auth_json[0].get("Id").is_some() {
          Ok(re_auth_json[0].get("Id").unwrap().to_string()[1..re_auth_json[0].get("Id").unwrap().to_string().len() - 1].to_string())
        }
        else {
          Err("exp".to_string())
        }
      } else {
        Err("This is not a valid emby/jellyfin webpage!".to_string())
      }
    }
    Err(e) => {
      Err(e)
    }
  }
}


fn smol_puddler_get(url: String, auth_header: &AuthHeader) -> Result<Response<Body>, String> {
  let response: Result<Response<Body>, isahc::Error> = Request::get(url)
    .timeout(Duration::from_secs(5))
    .header("Authorization", &auth_header.authorization)
    .header("Content-Type", "application/json")
    .body(()).unwrap()
    .send();
  match response {
    Ok(res) => {
      Ok(res)
    }
    Err(e) => {
      Err(e.to_string())
    }
  }
}


fn get_request_header(access_token: &str) -> RequestHeader {
  let token = access_token.to_owned();
  RequestHeader {
    application: format!("{APPNAME}/{VERSION}"),
    token
  }
}
