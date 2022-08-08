#![allow(non_snake_case)]
extern crate getch;
use std::char;
use std::fmt::Debug;
use std::io;
use app_dirs::*;
use http::Response;
use http::StatusCode;
use colored::Colorize;
use isahc::Body;
use uuid;
use isahc::Request;
use isahc::prelude::*;
use serde_json::Value;
use std::time::Duration;
use rpassword::read_password;
use std::io::prelude::*;
use std::net::UdpSocket;
use std::str::from_utf8;
use crate::APPNAME;
use crate::VERSION;
use crate::APP_INFO;
use crate::settings::Settings;
use crate::config::*;
use std::result::Result;
use serde_derive::{Deserialize,Serialize};


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
                println!("{}\n", ch);
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


pub fn check_information(settings: &Settings) -> HeadDict {
    let media_server: &str;
    let emby: bool;
    let media_server_name: &str;
    let mut auth_header: AuthHeader;
    let device_id = uuid::Uuid::new_v4().to_string();
    let server_kind = if settings.server_config.is_none() {
        print!("What kind of server do you want to stream from?\n   [1] Emby\n   [2] Jellyfin");
        getch("12")
    } else {
        match read_config(&settings.server_config.as_ref().unwrap(), true) {
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
                authorization: format!("Emby UserId=\"\", Client=\"Emby Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\", Token=\"\"", APPNAME, device_id, VERSION)
            };
        }
        _ => {
            emby = false;
            media_server = "";
            media_server_name = "Jellyfin";
            auth_header = AuthHeader {
                authorization: format!("Emby UserId=\"\", Client=\"Emby Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\", Token=\"\"", APPNAME, device_id, VERSION)
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
    if config_path.is_none() {
        app_root(AppDataType::UserConfig, &APP_INFO).expect("shit");
        let (ipaddress, server_name) = configure_new_server(media_server_name);
        let user_login = configure_new_login(media_server_name);
        (auth_header, request_header, session_id, user_id, access_token, server_id) = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
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
    } else {
        let config_path_string = config_path.unwrap();
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
                let session_id_test: Option<String> = re_auth(media_server_name, media_server, ipaddress, &auth_header, &device_id);
                if session_id_test.is_none() {
                    println!("\nYour {} session expired! Please re-login.", media_server_name);
                    let user_login = configure_new_login(media_server_name);
                    (auth_header, request_header, session_id, _, access_token, _) = test_auth(media_server_name, media_server, ipaddress, &auth_header, &user_login, &device_id);
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
                    session_id = session_id_test.unwrap();
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
                (auth_header, request_header, session_id, user_id, access_token, _) = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
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
                let (ipaddress, server_name) = configure_new_server(media_server_name);
                let user_login = configure_new_login(media_server_name);
                (auth_header, request_header, session_id, user_id, access_token, _) = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
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
            },
            _ => {
                let (ipaddress, server_name) = configure_new_server(media_server_name);
                let user_login = configure_new_login(media_server_name);
                device_id = uuid::Uuid::new_v4().to_string();
                (auth_header, request_header, session_id, user_id, access_token, _) = test_auth(media_server_name, media_server, &ipaddress, &auth_header, &user_login, &device_id);
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
    }
    HeadDict {
        media_server_name: media_server_name.to_string(),
        media_server: media_server.to_string(),
        config_file,
        auth_header,
        request_header,
        session_id
    }
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
    if cfg!(windows) {
        socket.send_to(&String::from(who_is).into_bytes(), "255.255.255.255:7359").expect("fdsfds");
    } else {
        socket.send_to(&String::from(who_is).into_bytes(), "255.255.255.255:7359").expect("fdsfds");
    }
    let mut buf  = [0; 4096];
    let udp_disco = socket.recv_from(&mut buf);
    match udp_disco {
        Ok(_t) => {
            let parsed: UDP = byte_array_to_json(buf);
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
        ipaddress = format!("http://{}", ipaddress);
    }
    if ipaddress.ends_with('/') {
        ipaddress.pop();
    }
    (ipaddress, server_name)
}


#[derive(Serialize, Deserialize)]
struct UDP {
    Address: String,
    Id: String,
    Name: String,
}


fn byte_array_to_json(buf: [u8; 4096]) -> UDP {
    let response = from_utf8(&buf).expect("sos").trim_matches(char::from(0));
    serde_json::from_str(response).unwrap()
}


fn configure_new_login(media_server_name: &str) -> UserLogin {
    fn take_input(media_server_name: &str) -> (String, String) {
        let mut username = String::new();
        print!("Please enter your {} username: ", media_server_name);
        io::stdout().flush().expect("Failed to flush stdout");
        io::stdin().read_line(  &mut username).unwrap();
        print!("Please enter your {} password (hidden): ", media_server_name);
        io::stdout().flush().expect("Failed to flush stdout");
        let password = read_password().unwrap();
        println!("");
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


fn test_auth (media_server_name: &str, media_server: &str, ipaddress: &String, auth_header: &AuthHeader, user_login: &UserLogin, device_id: &String) -> (AuthHeader, RequestHeader, String, String, String, String) {
    println!("Testing {} connection ...", media_server_name);
    let username: String = user_login.username.clone();
    let password: String = user_login.pw.clone();
    let bod = format!("{{\"Username\":\"{}\",\"pw\":\"{}\"}}", username, password);
    let url = format!("{}{}/Users/AuthenticateByName", ipaddress, media_server);
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
            (
                AuthHeader {
                    authorization: format!("Emby UserId={}, Client=\"Emby Theater\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\", Token={}", user_id, APPNAME, device_id, VERSION, token)
                },
                RequestHeader {
                    application: format!("{}/{}", APPNAME, VERSION),
                    token: token.to_string()
                },
                session_id.to_string(),
                user_id.to_string(),
                token.to_string(),
                server_id.as_str().unwrap().trim().to_string()
        )
        },
        _ => panic!("FCK")
    }
}


pub fn post_puddler (url: String, auth_header: &AuthHeader, bod: String) -> Result<Response<Body>, isahc::Error> {
    let response = Request::post(url)
        .header("Authorization", &auth_header.authorization)
        .header("Content-Type", "application/json")
        .body(bod)?
        .send()?;
    let result = match response.status() {
        StatusCode::OK => {
            response
        },
        StatusCode::NOT_FOUND => panic!("Not Found"),
        StatusCode::BAD_REQUEST => panic!("Bad Request"),
        _ => panic!("{} fdsfds", response.status())
    };
    Ok(result)
}


fn re_auth(media_server_name: &str, media_server: &str, ipaddress: &String, auth_header: &AuthHeader, device_id: &String) -> Option<String> {
    println!("Testing {} connection ...", media_server_name);
    let re_auth_res = smol_puddler_get(format!("{}{}/Sessions?DeviceId={}", ipaddress, media_server, &device_id), auth_header);
    match re_auth_res {
        Ok(mut t) => {
            match t.status() {
                StatusCode::OK => {
                    let response_text: &String = &t.text().unwrap();
                    let re_auth_json: Value = serde_json::from_str(response_text).unwrap();
                    println!("{}", "Connection successfully reestablished!".to_string().green());
                    if re_auth_json[0].get("Id").is_some() {
                        Some(re_auth_json[0].get("Id").unwrap().to_string()[1..re_auth_json[0].get("Id").unwrap().to_string().len() - 1].to_string())
                    }
                    else {
                        None
                    }
                }
                StatusCode::UNAUTHORIZED => {
                    None
                }
                _ => {
                    panic!("{}", t.status())
                }
            }
        }
        Err(_e) => {
            None
        }
    }
}


fn smol_puddler_get(url: String, auth_header: &AuthHeader) -> Result<Response<Body>, isahc::Error> {
    let response: Response<Body> = Request::get(url)
        .timeout(Duration::from_secs(5))
        .header("Authorization", &auth_header.authorization)
        .header("Content-Type", "application/json")
        .body(())?
        .send()?;
    match response.status() {
        StatusCode::OK => {
            Ok(response)
        }
        StatusCode::UNAUTHORIZED => {
            Ok(response)
        }
        _ => panic!("{} your server is missing some api endpoints, i think", response.status())
    }
}


fn get_request_header(access_token: &String) -> RequestHeader {
    let token = access_token.clone();
    RequestHeader {
        application: format!("{}/{}", APPNAME, VERSION),
        token
    }
}
