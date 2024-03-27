#![allow(non_snake_case)]
use std::{
  char,
  fmt::Debug,
  fs,
  io::{
    stdout,
    stdin
  },
  io::prelude::*,
  net::UdpSocket,
  result::Result,
  time::Duration,
  str::from_utf8,
  process::exit,
};
use crossterm::{
  event::{
    KeyCode,
    KeyEventKind,
    Event,
    KeyEvent,
    KeyModifiers,
    poll,
    read
  },
  terminal::{
    ClearType,
    Clear,
    disable_raw_mode,
    enable_raw_mode
  },
  cursor::{
    MoveToNextLine,
    MoveToColumn,
    EnableBlinking,
    Show
  },
  execute
};
use isahc::{
  Body,
  Request,
  Response,
  prelude::*,
  http::StatusCode
};
use uuid;
use colored::Colorize;
use serde_json::Value;
use serde_derive::{Deserialize,Serialize};
use rpassword::read_password;

use crate::APPNAME;
use crate::VERSION;
use crate::settings::Settings;
use crate::config::*;


#[derive(Debug, Clone)]
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


#[derive(Debug, Clone)]
pub struct RequestHeader {
  pub application: String,
  pub token: String
}


#[derive(Debug, Clone)]
pub struct AuthHeader {
  pub authorization: String,
}


pub fn getch(allowed: &str) -> char {
  adv_getch(allowed, false, None, "").unwrap()
}


pub fn clear_stdin() {
  enable_raw_mode().unwrap();
  loop {
    if poll(Duration::from_millis(100)).unwrap() {
      if let Ok(_) = read() {
        continue;
      }
    } else {
      disable_raw_mode().unwrap();
      return;
    }
  }
}


pub fn adv_getch(allowed: &str, any_key: bool, timeout_secs: Option<u64>, message: &str) -> Option<char> {
  let mut stdout = stdout();
  let mut timer = timeout_secs.map(|seconds| seconds * 2);

  if let Some(time) = timer {
    print!("\n{} [{}]: ", message, time / 2);
  } else {
    print!("\n{}: ", message);
  }
  stdout.flush().expect("Failed to flush stdout");

  enable_raw_mode().unwrap();
  execute!(stdout, EnableBlinking, Show).unwrap();

  loop {
    if poll(Duration::from_millis(500)).unwrap() {
      if let Ok(event) = read() {
        if let Event::Key(KeyEvent { code, modifiers, state: _, kind }) = event {
          if modifiers == KeyModifiers::NONE && kind == KeyEventKind::Press && ! any_key {
            for ch in allowed.chars() {
              if code == KeyCode::Char(ch) {
                disable_raw_mode().unwrap();
                println!("{}\n", ch);
                return Some(ch);
              }
            }
            writeln!(stdout).unwrap();
            execute!(stdout, MoveToNextLine(1)).unwrap();
            writeln!(stdout, "Invalid input, please try again.").unwrap();
            execute!(stdout, MoveToNextLine(1)).unwrap();
            if let Some(time) = timer {
              write!(stdout, "{} [{}]: ", message, time / 2).unwrap();
            } else {
              write!(stdout, "{}: ", message).unwrap();
            }
            stdout.flush().expect("Failed to flush stdout");
          } else if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
            write!(stdout, "^C").unwrap();
            disable_raw_mode().unwrap();
            exit(1);
          } else if any_key && kind == KeyEventKind::Press {
            disable_raw_mode().unwrap();
            println!("\n");
            return Some('_'); // this is a smiley
          }
        }
      }
    } else if let Some(time) = timer {
      timer = Some(time - 1);
      execute!(stdout, MoveToColumn(0)).unwrap();
      execute!(stdout, Clear(ClearType::CurrentLine)).unwrap();
      if timer == Some(0) {
        disable_raw_mode().unwrap();
        return None;
      } else {
        write!(stdout, "{} [{}]: ", message, time / 2).unwrap();
        stdout.flush().expect("Failed to flush stdout");
      }
    }
  }
}


pub fn validate_settings(settings: &Settings) -> Option<HeadDict> {
  let mut auth_header: AuthHeader;
  let media_server: &str;
  let emby: bool;
  let media_server_name: &str;
  let device_id = uuid::Uuid::new_v4().to_string();
  
  let server_type = if settings.server_config.is_none() {
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
  match server_type {
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
    choose_config(server_type, settings.autologin)
  } else {
    settings.server_config.clone()
  };

  let request_header: RequestHeader;
  let session_id: String;
  let user_id: String;
  let access_token: String;
  let server_id: String;
  let config_file: ConfigFile;
  let mut device_id = uuid::Uuid::new_v4().to_string();

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
        let session_id_test = reauthenticate(media_server_name, media_server, ipaddress, &auth_header, &device_id);
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
        } else if session_id_test == Err("exp".to_string()) {
          println!("\nYour {media_server_name} session has expired! Please re-login.");
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
          let config_path_string = generate_config_path(server_type, server_id, server_name);
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
    let app_root = dirs::config_dir().unwrap();
    if fs::read_dir(&app_root).is_err() {
      fs::create_dir_all(&app_root).expect("Could not create config directory!")
    };
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
    let config_path_string = generate_config_path(server_type, server_id, server_name);
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
  println!("Searching for local media-server ...");
  let socket: UdpSocket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to network socket.");
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
          stdout().flush().expect("Failed to flush stdout");
          let mut ipaddress2 = String::new();
          stdin().read_line(  &mut ipaddress2).unwrap();
          ipaddress = ipaddress2.trim().parse().unwrap();
          print!("\nPlease enter a nickname for your media-server.\n(It's recommended to use a unique one)\n: ");
          stdout().flush().expect("Failed to flush stdout");
          server_name = String::new();
          stdin().read_line(  &mut server_name).unwrap();
          server_name = server_name.trim().parse().unwrap();
        }
        _ => (),
      }
    },
    Err(_e) => {
      print!("Couldn't find any local media-server.\nIf your instance is running under a docker environment, configure the host network-option.\nOr just specify the IP-Address manually. (don't forget to add ports)\n: ");
      stdout().flush().expect("Failed to flush stdout");
      let mut ipaddress2 = String::new();
      stdin().read_line(  &mut ipaddress2).unwrap();
      ipaddress = ipaddress2.trim().parse().unwrap();
      print!("\nPlease enter a nickname for your media-server.\n(It's recommended to use a unique one)\n: ");
      stdout().flush().expect("Failed to flush stdout");
      server_name = String::new();
      stdin().read_line(  &mut server_name).unwrap();
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
    stdout().flush().expect("Failed to flush stdout");
    stdin().read_line(  &mut username).unwrap();
    print!("Please enter your {media_server_name} password (hidden): ");
    stdout().flush().expect("Failed to flush stdout");
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


fn test_auth(media_server_name: &str, media_server: &str, ipaddress: &String, auth_header: &AuthHeader, user_login: &UserLogin, device_id: &String) -> Option<(AuthHeader, RequestHeader, String, String, String, String)> {
  print!("Testing {media_server_name} connection ... ");
  let username: String = user_login.username.clone();
  let password: String = user_login.pw.clone();
  let bod = format!("{{\"Username\":\"{username}\",\"pw\":\"{password}\"}}");
  let url = format!("{ipaddress}{media_server}/Users/AuthenticateByName");
  let json_response = server_post(url, auth_header, bod);
  match json_response {
    Ok(mut t) => {
      println!("{}", "successfull!".to_string().green());
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
      println!("{}", "error!".to_string().red());
      println!("{}\n  Error: {}", "Failed to establish a working connection!".to_string().red(), e);
      None
    }
  }
}


pub fn server_post(url: String, auth_header: &AuthHeader, body: String) -> Result<Response<Body>, String> {
  let mut response = Request::post(url)
    .header("Authorization", &auth_header.authorization)
    .header("Content-Type", "application/json")
    .body(body).unwrap()
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


fn reauthenticate(media_server_name: &str, media_server: &str, ipaddress: &String, auth_header: &AuthHeader, device_id: &String) -> Result<String, String> {
  print!("Testing {media_server_name} connection ... ");
  let re_auth_res = http_get(format!("{}{}/Sessions?DeviceId={}", ipaddress, media_server, &device_id), auth_header);
  match re_auth_res {
    Ok(mut t) => {
      let response_text: &String = &t.text().unwrap();
      if let Ok(re_auth_json) = serde_json::from_str::<Value>(response_text) {
        if let Some(id) = re_auth_json[0].get("Id") {
          println!("{}", "successful!".to_string().green());
          Ok(id.to_string().replace('"', ""))
        } else {
          println!("{}", "error!".to_string().red());
          Err("exp".to_string())
        }
      } else {
        println!("{}", "error!".to_string().red());
        Err("This is not a valid emby/jellyfin webpage!".to_string())
      }
    }
    Err(e) => {
      println!("{}", "error!".to_string().red());
      Err(e)
    }
  }
}


fn http_get(url: String, auth_header: &AuthHeader) -> Result<Response<Body>, String> {
  let authorization = &auth_header.authorization;
  let response: Result<Response<Body>, isahc::Error> = Request::get(url)
    .timeout(Duration::from_secs(5))
    .header("Authorization", authorization)
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
