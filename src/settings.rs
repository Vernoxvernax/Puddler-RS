use colored::Colorize;
use config::{Config, File};
use std::fs;
use toml;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use crate::mediaserver_information;
use mediaserver_information::getch;
use app_dirs::*;
use serde_derive::{Deserialize,Serialize};
use crate::APPNAME;
// use crate::VERSION;
use crate::APP_INFO;


#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    pub server_config: Option<String>,
    pub discord_presence: bool,
    // pub direct_stream: bool,
    pub fullscreen: bool,
    pub autologin: bool
}


fn read_settings() -> Settings {
    let config_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    let config_path_string = format!("{}/{}.toml", &config_path.display().to_string(), &APPNAME);
    if ! Path::new(&config_path_string).is_file() {
        println!("No settings file found!\nBuilding default settings ...\n");
        // Default <> server.
        let server_config: Option<String> = search_server_configs();
        // Discord Presence default setting.
        let discord_presence: bool = initiate_discord();
        // Activate encoded streaming (requires fully configured media-server)
        // let direct_stream: bool = direct_streaming(); // planned but not implemented
        // Wether mpv should start in fullscreen mode.
        let fullscreen: bool = start_fullscreen();
        // Wether the user should be prompted if the default login is correct
        let autologin: bool = automatically_login();
        let settings = Settings {
            server_config,
            discord_presence,
            // direct_stream,
            fullscreen,
            autologin
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
            Err(_) => {
                println!("{}", format!("Settings file is corrupt. Settings have to be reconfigured.\n").red());
                let server_config: Option<String> = search_server_configs();
                let discord_presence: bool = initiate_discord();
                let fullscreen: bool = start_fullscreen();
                let autologin: bool = automatically_login();
                let settings = Settings {
                    server_config,
                    discord_presence,
                    // direct_stream,
                    fullscreen,
                    autologin
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


fn initiate_discord() -> bool {
    print!("Do you want to activate Discord-Presence by default?\n (Y)es / (N)o");
    let presence = getch("YyNn");
    let connection: bool = match presence {
        'Y' | 'y' => {
            true
        },
        'N' | 'n' => {
            false
        },
        _ => (
            false
        )
    };
    connection
}


fn search_server_configs() -> Option<String> {
    let config_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    println!("Searching in \"{}\" for emby or jellyfin configuration files ...", &config_path.display());
    let path: Vec<_> = fs::read_dir(&config_path).unwrap().map(|r| r.unwrap()).collect();
    let mut files: Vec<String> = [].to_vec();
    for file in &path {
        if file.path().is_dir() {
            let depth2: Vec<_> = fs::read_dir(&file.path()).unwrap().map(|r| r.unwrap()).collect();
            for stuff in depth2 {
                let file_path: String = stuff.path().display().to_string();
                if file_path.contains(&".config.json") {
                    files.append(&mut [file_path].to_vec());  
                } else {
                    continue
                }
            }
        }
        let file_path: String = file.path().display().to_string();
        if file_path.contains(&".config.json") {
            files.append(&mut [file_path].to_vec());
        } else {
            continue
        }
    };
    if files.len() == 0 {
        println!("No configuration has been found.");
        return None
    } else {
        for (index, path) in files.iter().enumerate() {
            println!("  [{}] {}", index, path);
        }
    }
    print!("Select which one of the above server configs should be used by default, or skip with \"None\".\n: ");
    io::stdout().flush().ok().expect("Failed to flush stdout");
    let mut selection = String::new();
    io::stdin().read_line(&mut selection).unwrap();
    if selection.trim() == "None" {
        println!("Skipped default-server option.\n");
        return None
    }
    let num_selection: usize = selection.trim().parse().unwrap();
    println!("You've picked {}.\n", format!("{:?}", path[num_selection - 1].file_name()).green());
    Some(path[num_selection - 1].path().display().to_string())
}


// fn direct_streaming() -> bool {
//     print!("Is your internet speed capable to directly stream the media?\n  (e.g.: if the emby/jellyfin instance is running locally)\n (Y)es / (N)o [this requires server-side configuration]");
//     let encode = getch("YyNn");
//     match encode {
//         'Y' | 'y' => {
//             true
//         },
//         'N' | 'n' => {
//             false
//         },
//         _ => (
//             false
//         )
//     }
// }


fn start_fullscreen() -> bool {
    print!("Do you want mpv to start in fullscreen-mode?\n (Y)es / (N)o");
    let fullscreen = getch("YyNn");
    match fullscreen {
        'Y' | 'y' => {
            true
        },
        'N' | 'n' => {
            false
        },
        _ => (
            false
        )
    }
}


fn change_settings(mut settings: Settings) -> Settings {
    let config_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    let config_path_string = format!("{}/{}.toml", &config_path.display().to_string(), &APPNAME);
    loop {
        // print!("Which settings do you want to change?\n  [1] Default server configuration = {}\n  [2] Discord presence = {}\n  [3] Direct Stream = {}\n  [4] MPV fullscreen = {}\n\n  [S] Save and return to the menu", format!("{}", settings.server_config.as_ref().unwrap_or(&"None".to_string())).green(), format!("{}", settings.discord_presence).green(), format!("{}", settings.direct_stream).green(), format!("{}", settings.fullscreen).green());
        print!("Which settings do you want to change?\n  [1] Default server configuration = {}\n  [2] Discord presence = {}\n  [3] MPV fullscreen = {}\n  [4] Automatically login = {}\n\n  [S] Save and return to the menu", settings.server_config.as_ref().unwrap_or(&"None".to_string()).to_string().green(), settings.discord_presence.to_string().green(), settings.fullscreen.to_string().green(), settings.autologin.to_string().green());
        let menu = getch("1234Ss");
        match menu {
            '1' => {
                settings.server_config = search_server_configs();
            },
            '2' => {
                settings.discord_presence = initiate_discord();
            },
            '3' => {
            //     settings.direct_stream = direct_streaming();
            // },
            // '4' => {
                settings.fullscreen = start_fullscreen();
            },
            '4' => {
                settings.autologin = automatically_login();
            },
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
    // println!("\n Default server configuration = {}\n Discord presence = {}\n Direct Stream = {}\n MPV fullscreen = {}", format!("{}", settings.server_config.as_ref().unwrap_or(&"None".to_string())).green(), format!("{}", settings.discord_presence).green(), format!("{}", settings.direct_stream).green(), format!("{}", settings.fullscreen).green());
    println!(" Default server configuration = {}\n Discord presence = {}\n MPV fullscreen = {}\n Automatically login = {}\n", settings.server_config.as_ref().unwrap_or(&"None".to_string()).to_string().green(), settings.discord_presence.to_string().green(), settings.fullscreen.to_string().green(), settings.autologin.to_string().green());
}


fn automatically_login() -> bool {
    print!("Do you want to enable autologin on start?\n (Y)es / (N)o");
    let autologin = getch("YyNn");
    match autologin {
        'Y' | 'y' => {
            true
        },
        'N' | 'n' => {
            false
        },
        _ => (
            false
        )
    }
}