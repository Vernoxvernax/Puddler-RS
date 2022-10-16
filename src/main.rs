#![allow(non_snake_case)]
use colored::Colorize;
use progress_report::MediaSourceInfo;
use urlencoding::encode;
use std::time::Duration;
use std::process;
use http::Response;
use http::StatusCode;
use std::io::prelude::*;
use std::io;
use serde_derive::{Deserialize};
use isahc::Body;
use isahc::Request;
use isahc::prelude::*;
mod progress_report;
pub mod mediaserver_information;
pub mod player;
pub mod settings;
pub mod config;
pub mod discord;
use player::play;
use mediaserver_information::*;
use settings::*;
const APPNAME: &str = "Puddler";
const VERSION: &str = env!("CARGO_PKG_VERSION");
use app_dirs::AppInfo;
const APP_INFO: AppInfo = AppInfo{
    name: APPNAME,
    author: "VernoxVernax"
};



#[derive(Debug, Deserialize)]
struct ItemJson {
    Items: Vec<Items>,
    TotalRecordCount: Option<u16>
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Items {
    pub Name: String,
    pub Id: String,
    pub RunTimeTicks: Option<u64>,
    pub Type: String,
    pub UserData: UserData,
    pub SeriesName: Option<String>,
    pub SeriesId: Option<String>,
    pub SeasonName: Option<String>,
    pub SeasonId: Option<String>,
    pub PremiereDate: Option<String>,
    pub MediaSources: Option<Vec<MediaSourceInfo>>
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct UserData {
    pub PlayedPercentage: Option<f64>,
    pub PlaybackPositionTicks: u64,
    pub Played: bool,
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
struct SeriesStruct {
    Items: Vec<Seasons>
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
struct Seasons {
    Name: String,
    Id: String,
    Type: String,
    UserData: UserData,
    SeriesName: String,
    SeriesId: String,
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
struct SeasonStruct {
    Items: Vec<Items>
}


fn main() {
    let mut settings: Settings = initialize_settings(0);
    println!("{}", r"     ____            __    ____         
    / __ \__  ______/ /___/ / /__  _____
   / /_/ / / / / __  / __  / / _ \/ ___/
  / ____/ /_/ / /_/ / /_/ / /  __/ /    
 /_/    \__,_/\__,_/\__,_/_/\___/_/".to_string().bright_cyan());
    println!();
    loop {
        if settings.server_config.is_some() {
            print!("  [ENTER] Stream from default media-server\n  [1] Stream from either Emby or Jellyfin\n  [2] Change puddlers default settings\n  [3] Display current settings\n  [E] Exit puddler");
            let menu = getch("123Ee\n");
            match menu {
                '\n' => {
                    break
                }
                '1' => {
                    settings.server_config = None;
                    break
                },
                '2' => {
                    settings = initialize_settings(1);
                },
                '3' => {
                    settings = initialize_settings(2);
                },
                'e' | 'E' => {
                    process::exit(0x0100);
                },
                _ => (
                )
            };
        } else {
            print!("  [1] Stream from either Emby or Jellyfin\n  [2] Change puddlers default settings\n  [3] Display current settings\n  [E] Exit puddler");
            let menu = getch("123Ee");
            match menu {
                '1' => {
                    break
                },
                '2' => {
                    settings = initialize_settings(1);
                },
                '3' => {
                    settings = initialize_settings(2);
                },
                'e' | 'E' => {
                    process::exit(0x0100);
                },
                _ => (
                )
            };
        }
    }
    let head_dict = check_information(&settings);
    loop {
        choose_and_play(&head_dict, &settings);
    }
}


fn choose_and_play(head_dict: &HeadDict, settings: &Settings) {
    let ipaddress = &head_dict.config_file.ipaddress;
    let media_server = &head_dict.media_server;
    let user_id = &head_dict.config_file.user_id;
    // nextup & resume
    let mut item_list: Vec<Items> = Vec::new();
    let pick: Option<i32>;
    let nextup = puddler_get(format!("{}{}/Users/{}/Items/Resume?Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id), head_dict);
    let response: ItemJson = match nextup {
        Ok(mut t) => {
            let response_text = &t.text().unwrap();
            serde_json::from_str(response_text).unwrap()
        }
        Err(e) => panic!("failed to parse get request: {}", e)
    };
    if response.TotalRecordCount.unwrap() != 0 {
        println!("\nContinue Watching:");
        item_list = print_menu(&response, true, item_list);
    }
    // if media_server != "/emby" {
    let latest_series = puddler_get(format!("{}{}/Users/{}/Items/Latest?Limit=10&IncludeItemTypes=Episode&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id), head_dict);
    let latest_series_response: ItemJson = match latest_series {
        Ok(mut t) => {
            let response_text = &t.text().unwrap();
            let response_text = format!("{{\"Items\":{}}}", &response_text.to_string());
            serde_json::from_str(&response_text).unwrap()
        }
        Err(e) => panic!("failed to parse get request: {}", e)
    };
    if latest_series_response.Items.len() > 0 {
        println!("\nLatest:");
        // if response.TotalRecordCount.unwrap() == 0 {
        //     println!("\nContinue Watching:");
        // }
        item_list = print_menu(&latest_series_response, true, item_list);
    }
    // }
    // latest
    let latest = puddler_get(format!("{}{}/Users/{}/Items/Latest?Limit=10&IncludeItemTypes=Movie&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id), head_dict);
    let latest_response: ItemJson = match latest {
        Ok(mut t) => {
            let response_text: &String = &t.text().unwrap();
            let response_text = format!("{{\"Items\":{}}}", &response_text.to_string());
            serde_json::from_str(&response_text).unwrap()
        }
        Err(e) => panic!("failed to parse get request: {}", e)
    };
    if latest_response.Items.len() > 0 {
        if latest_series_response.Items.len() == 0 {
            println!("\nLatest:");
        }
        item_list = print_menu(&latest_response, true, item_list);
    }
    print!("Please choose from above, enter a search term, or type \"ALL\" to display literally everything.\n: ");
    io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    // processing input
    if input.trim() == "ALL" {
        let all = puddler_get(format!("{}{}/Items?UserId={}&Recursive=true&IncludeItemTypes=Series,Movie&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id), head_dict);
        let all_response: ItemJson = match all {
            Ok(mut t) => {
                let response_text: &String = &t.text().unwrap();
                serde_json::from_str(response_text).unwrap()
            }
            Err(e) => panic!("failed to parse get request: {}", e)
        };
        item_list = Vec::new();
        item_list = print_menu(&all_response, false, item_list);
        if all_response.TotalRecordCount.unwrap() > 1 {
            print!(": ");
            io::stdout().flush().expect("Failed to flush stdout");
        }
        pick = process_input(&item_list, None);
    } else if numbers(&input) {
        pick = process_input(&item_list, Some(input.trim().to_string()));
    } else {
        input = encode(input.trim()).into_owned();
        let search = puddler_get(format!("{}{}/Items?SearchTerm={}&UserId={}&Recursive=true&IncludeItemTypes=Series,Movie&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &input, &user_id), head_dict);
        let search_response: ItemJson = match search {
            Ok(mut t) => {
                let search_text: &String = &t.text().unwrap();
                serde_json::from_str(search_text).unwrap()
            }
            Err(e) => panic!("failed to parse get request: {}", e)
        };
        if &search_response.Items.len() > &0 {
            item_list = Vec::new();
            item_list = print_menu(&search_response, false, item_list);
            if search_response.TotalRecordCount.unwrap() > 1 {
                print!(": ");
                io::stdout().flush().expect("Failed to flush stdout");
            }
            pick = process_input(&item_list, None);
        } else {
            println!("\nNo results found for: {}.", input.to_string().bold());
            pick = None
        }
    }
    if pick.is_some() {
        item_parse(head_dict, &item_list, pick.unwrap(), settings);
    }
}


fn puddler_get(url: String, head_dict: &HeadDict) -> Result<Response<Body>, isahc::Error> {
    let request_header = &head_dict.request_header;
    let response: Response<Body> = Request::get(url)
        .timeout(Duration::from_secs(5))
        .header("X-Application", &request_header.application)
        .header("X-Emby-Token", &request_header.token)
        .header("Content-Type", "application/json")
        .body(())?
        .send()?;
    let result = match  response.status() {
        StatusCode::OK => {
            response
        }
        _ => panic!("{} your server is missing some api endpoints, i think", response.status())
    };
    Ok(result)
}


fn numbers(input: &String) -> bool {
    for x in input.trim().chars() {
        if x.is_alphabetic() {
            return false
        }
    }
    true
}


fn process_input(item_list: &Vec<Items>, number: Option<String>) -> Option<i32> {
    let items_in_list: i32 = item_list.len().try_into().unwrap();
    if items_in_list > 1 {
        let mut raw_input: String;
        if number.is_none() {
            raw_input = String::new();
            io::stdin().read_line(&mut raw_input).unwrap();
            raw_input = raw_input.trim().to_string();
        } else {
            raw_input = number.unwrap()
        }
        let pick = raw_input.parse::<i32>().unwrap();
        if pick < items_in_list + 1 && pick >= 0 {
            let item = item_list.iter().nth(pick as usize).unwrap();
            if "Episode Special".contains(&item.Type) {
                println!("\nYou've chosen {}.\n", format!("{} ({}) - {} - {}", item.SeriesName.as_ref().unwrap(), (&item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]), item.SeasonName.as_ref().unwrap(), item.Name).cyan());
            } else {
                println!("\nYou've chosen {}.\n", format!("{} ({})", item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]).cyan());
            }
        } else {
            println!("{}", "Are you stupid?!".to_string().red());
            process::exit(0x0100);
        }
        Some(pick)
    } else if items_in_list == 1 {
        let mut raw_input = String::new();
        io::stdin().read_line(&mut raw_input).unwrap();
        let pick: i32 = 0;
        Some(pick)
    } else {
        None
    }
}


fn item_parse(head_dict: &HeadDict, item_list: &Vec<Items>, pick: i32, settings: &Settings) {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let media_server: &String = &head_dict.media_server;
    let user_id: &String = &head_dict.config_file.user_id;
    if item_list.iter().nth(pick as usize).unwrap().Type == *"Movie" {
        let item = item_list.iter().nth(pick as usize).unwrap();
        println!("Starting mpv ...");
        play(settings, head_dict, item);
    } else if item_list.iter().nth(pick as usize).unwrap().Type == *"Series" {
        let series = &item_list.iter().nth(pick as usize).unwrap();
        println!("{}:", series.Name);
        let series_response = puddler_get(format!("{}{}/Users/{}/Items?ParentId={}&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id, &series.Id), head_dict);
        let series_json: SeriesStruct = match series_response {
            Ok(mut t) => {
                let parse_text: &String = &t.text().unwrap();
                serde_json::from_str(parse_text).unwrap()
            }
            Err(e) => panic!("failed to parse series request: {}", e)
        };
        let item_list: Vec<Items> = process_series(&series_json, head_dict, true);
        let filtered_input: i32;
        let items_in_list: i32 = item_list.len().try_into().unwrap();
        if items_in_list > 1 {
            print!("Please enter which episode you want to continue at.\n: ");
            io::stdout().flush().expect("Failed to flush stdout");
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input = encode(input.trim()).into_owned();
            filtered_input = input.parse::<i32>().unwrap();
            if filtered_input < items_in_list + 1 && filtered_input >= 0 {
                let item_listed = item_list.clone();
                let item = &item_listed.into_iter().nth(filtered_input as usize).unwrap();
                if "Episode Special".contains(&item.Type) {
                    println!("\nYou've chosen {}.\n", format!("{} ({}) - {} - {}", item.SeriesName.as_ref().unwrap(), (&item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]), item.SeasonName.as_ref().unwrap(), item.Name).cyan());
                } else {
                    println!("\nYou've chosen {}.\n", format!("{} ({})", item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]).cyan());
                }
            } else {
                println!("{}", "Are you stupid?!".to_string().red());
                process::exit(0x0100);
            }
        } else {
            filtered_input = 0;
        }
        series_play(&item_list, filtered_input, head_dict, settings);
    } else if "Special Episode".to_string().contains(&item_list.iter().nth(pick as usize).unwrap().Type) {
        let item: &Items = item_list.iter().nth(pick as usize).unwrap();
        let series_response = puddler_get(format!("{}{}/Users/{}/Items?ParentId={}&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id, &item.SeriesId.as_ref().unwrap()), head_dict);
        let series_json: SeriesStruct = match series_response {
            Ok(mut t) => {
                let parse_text: &String = &t.text().unwrap();
                serde_json::from_str(parse_text).unwrap()
            }
            Err(e) => panic!("failed to parse series request: {}", e)
        };
        let item_list: Vec<Items> = process_series(&series_json, head_dict, false);
        let mut item_pos: i32 = 0;
        for things in 0..item_list.len() {
            if item_list[things].Id == item.Id {
                if item_list[things].Type == "Special" {
                    item_pos = things.try_into().unwrap();
                    continue;
                };
                item_pos = things.try_into().unwrap();
                break;
            }
        };
        series_play(&item_list, item_pos, head_dict, settings);
    }
}


fn series_play(item_list: &Vec<Items>, mut pick: i32, head_dict: &HeadDict, settings: &Settings) {
    let episode_amount: i32 = item_list.len().try_into().unwrap();
    let item = &item_list.iter().nth(pick as usize).unwrap();
    println!("Starting mpv ...");
    play(settings, head_dict, item);
    loop {
        if ( pick + 2 ) > episode_amount { // +1 since episode_amount counts from 1 and +1 for next ep
            println!("\nYou've reached the end of your episode list. Returning to menu ...");
            break
        } else {
            pick += 1;
            if item_list.iter().nth(pick as usize).is_some() {
                let next_item = &item_list.iter().nth(pick as usize).unwrap();
                if next_item.UserData.Played {
                    continue
                };
                println!("\nWelcome back. Do you want to continue playback with:\n{}", format!("   {} ({}) - {} - {}", next_item.SeriesName.as_ref().unwrap(), &next_item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], next_item.SeasonName.as_ref().unwrap(), next_item.Name).cyan());
                print!(" (N)ext | (M)enu | (E)xit");
                let cont = getch("NnEeMm");
                match cont {
                    'N' | 'n' => {
                        let item = &item_list.iter().nth(pick as usize).unwrap();
                        play(settings, head_dict, item);
                    },
                    'M' | 'm' => break,
                    'E' | 'e' => {
                        process::exit(0x0100);
                    },
                    _ => (),
                }
            } else {
                break
            }
        }
    }
}


fn process_series(series: &SeriesStruct, head_dict: &HeadDict, printing: bool) -> Vec<Items> {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let media_server: &String = &head_dict.media_server;
    let user_id: &String = &head_dict.config_file.user_id;
    let mut index_iterator: i32 = 0;
    let mut episode_list: Vec<Items> = Vec::new();
    for season_numb in 0..series.Items.len() {
        let last_season = series.Items.len() == season_numb + 1;
        let season_branches = if last_season {
            "└─"
        } else {
            "├─"
        };
        let season: Seasons = series.Items[season_numb].clone();
        if printing {
            println!("  {} {}", season_branches, season.Name);
        }
        let season_res = puddler_get(format!("{}{}/Users/{}/Items?ParentId={}&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id, &season.Id), head_dict);
        let season_json: SeasonStruct = match season_res {
            Ok(mut t) => {
                let parse_text: &String = &t.text().unwrap().to_string();
                serde_json::from_str(parse_text).unwrap()
            }
            Err(e) => panic!("failed to parse series request: {}", e)
        };
        for episode_numb in 0..season_json.Items.len() { // for the code readers: the "season_json" vector is obviously different to "season" since the latter doesn't include any episodes.
            let episode: Items = season_json.Items[episode_numb].clone();
            let last_episode = season_json.Items.len() == episode_numb + 1;
            let episode_branches = if last_episode && last_season {
                "     └──"
            } else if last_episode &! last_season {
                "│    └──"
            } else if ! last_episode && last_season {
                "     ├──"
            } else {
                "│    ├──"
            };
            if ! episode_list.contains(&episode) {
                episode_list.push(season_json.Items[episode_numb].clone());
            } else if episode.SeasonName == Some("Specials".to_string()) {
                episode_list.push(season_json.Items[episode_numb].clone());
            }
            if ! printing {
                continue
            };
            if episode.UserData.PlayedPercentage.is_some() {
                let long_perc: f64 = episode.UserData.PlayedPercentage.unwrap();
                println!("  {} [{}] {} {} ", episode_branches, index_iterator, episode.Name, format!("{}%", long_perc.round() as i64))
            } else if episode.UserData.Played {
                println!("  {} [{}] {} {} ", episode_branches, index_iterator, episode.Name, "[PLAYED]".to_string().green());
            } else {
                println!("  {} [{}] {}", episode_branches, index_iterator, episode.Name);
            };
            index_iterator += 1;
        }
    };
    episode_list
}


fn print_menu(items: &ItemJson, recommendation: bool, mut item_list: Vec<Items>) -> Vec<Items> {
    let count: u16;
    if items.TotalRecordCount.is_some() && ! recommendation {
        count = items.TotalRecordCount.unwrap();
    } else {
        count = 10
    }
    if count > 1 && ! recommendation {
        println!("\nPlease choose from the following results:")
    }
    for h in 0..items.Items.len() {
        let x: Items = items.Items[h].clone();
        if ! item_list.contains(&x) {
            item_list.push(items.Items[h].clone());
            if ! x.UserData.Played {
                if x.UserData.PlayedPercentage.is_some() {
                    let long_perc: f64 = x.UserData.PlayedPercentage.unwrap();
                    let percentage = format!("{}%", long_perc.round() as i64); // Pardon the `.round`
                    if count != 1 {
                        if x.Type == *"Episode" || x.Type == *"Special" {
                            println!("      [{}] {} ({}) - {} - {} - ({}) {}", &item_list.iter().position(|y| y == &x).unwrap(), x.SeriesName.unwrap(), &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.SeasonName.unwrap(), x.Name, x.Type, percentage);
                        } else {
                            println!("      [{}] {} ({}) - ({}) {}", &item_list.iter().position(|y| y == &x).unwrap(), x.Name, &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.Type, percentage);
                        }
                    } else {
                        println!("\nOnly one item has been found.\nDo you want to select this title?\n      {}", format!("[Enter] {} ({}) - ({})", x.Name, &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.Type).cyan());
                    }
                } else if count != 1 {
                    if x.Type == *"Episode" || x.Type == *"Special" {
                        println!("      [{}] {} ({}) - {} - {} - ({})", &item_list.iter().position(|y| y == &x).unwrap(), x.SeriesName.unwrap(), &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.SeasonName.unwrap(), x.Name, x.Type);
                    } else {
                        println!("      [{}] {} ({}) - ({})", &item_list.iter().position(|y| y == &x).unwrap(), x.Name, &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.Type);
                    }
                } else {
                    println!("\nOnly one item has been found.\nDo you want to select this title?\n      {}", format!("[Enter] {} ({}) - ({})", x.Name, &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.Type).cyan());
                }
            } else if count != 1 {
                if x.Type == *"Episode" || x.Type == *"Special" {
                    println!("      [{}] {} ({}) - {} - {} - ({})  {}", &item_list.iter().position(|y| y == &x).unwrap(), x.SeriesName.unwrap(), &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.SeasonName.unwrap(), x.Name, x.Type, "[PLAYED]".to_string().green());
                } else {
                    println!("      [{}] {} ({}) - ({})  {}", &item_list.iter().position(|y| y == &x).unwrap(), x.Name, &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.Type, "[PLAYED]".to_string().green());
                }
            } else {
                println!("\nOnly one item has been found.\nDo you want to select this title?\n      {}", format!("[Enter] {} ({}) - ({})", x.Name, &x.PremiereDate.unwrap_or("????".to_string())[0..4], x.Type).cyan());
            }
        }
    }
    item_list
}
