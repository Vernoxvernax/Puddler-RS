#![allow(non_snake_case)]
use clap::Arg;
use clap::ArgAction;
use clap::Command;
use urlencoding::encode;
use colored::ColoredString;
use colored::Colorize;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
use std::process;
use http::Response;
use http::StatusCode;
use std::io::prelude::*;
use std::io;
use serde_derive::Deserialize;
use isahc::Body;
use isahc::Request;
use isahc::prelude::*;

pub mod progress_report;
pub mod mediaserver_information;
pub mod player;
pub mod settings;
pub mod config;
pub mod discord;
use player::play;
use settings::*;
use mediaserver_information::*;
use progress_report::mark_playstate;

const APPNAME: &str = "Puddler";
const VERSION: &str = env!("CARGO_PKG_VERSION");
use app_dirs::AppInfo;

const APP_INFO: AppInfo = AppInfo {
  name: APPNAME,
  author: "VernoxVernax"
};


#[derive(Debug, Deserialize)]
struct ItemJson {
  Items: Vec<Item>,
  TotalRecordCount: Option<u16>
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Item {
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
pub struct MediaSourceInfo {
  pub Id: String,
  pub SupportsTranscoding: bool,
  pub MediaStreams: Vec<MediaStream>,
  pub Bitrate: Option<u64>,
  pub TranscodingUrl: Option<String>
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct MediaStream {
  pub Index: usize,
  pub Type: String,
  pub Language: Option<String>,
  pub DisplayTitle: Option<String>,
  pub DisplayLanguage: Option<String>,
  pub Title: Option<String>,
  pub Codec: Option<String>,
  pub Width: Option<u32>,
  pub Height: Option<u32>,
  pub IsDefault: bool,
  pub IsExternal: bool,
  pub SupportsExternalStream: bool,
  pub Path: Option<String>
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct UserData {
  pub PlayedPercentage: Option<f64>,
  pub PlaybackPositionTicks: i64,
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
  Items: Vec<Item>
}


fn main() -> ExitCode {
  let command = Command::new("puddler")
    .about("Simple command-line client for Emby and Jellyfin.")
    .version(VERSION)
    .author("VernoxVernax")
    .arg(
      Arg::new("glsl-shader")
      .long("glsl-shader")
      .help("Play MPV using this shader-file.")
      .required(false)
      .action(ArgAction::Set)
      .num_args(1..)
    )
    .arg(
      Arg::new("mpv-config-dir")
      .long("mpv-config-dir")
      .help("Overwrite MPV's default config location.")
      .required(false)
      .action(ArgAction::Set)
      .num_args(1)
    )
    .arg(
      Arg::new("debug")
      .long("debug")
      .help("Print MPV log messages to \"./mpv.log\".")
      .required(false)
      .action(ArgAction::SetTrue)
    )
  .get_matches();

  let mut settings: Settings = initialize_settings(0);

  settings.glsl_shader = if command.get_many::<String>("glsl-shader").is_some() {
    Some(command.get_many::<String>("glsl-shader").unwrap().map(|sh| sh.to_string()).collect())
  } else {
    None
  };

  settings.mpv_debug = Some(command.get_flag("debug"));

  settings.mpv_config_location = command.get_one::<String>("mpv-config-dir").cloned();

  println!("{}",
r"     ____            __    ____         
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
          return ExitCode::SUCCESS;
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
          return ExitCode::SUCCESS;
        },
        _ => (
        )
      };
    }
  }
  if let Some(head_dict) = check_information(&settings) {
    loop {
      choose_and_play(&head_dict, &settings);
    }
  } else {
    ExitCode::FAILURE
  }
}


fn choose_and_play(head_dict: &HeadDict, settings: &Settings) {
  let ipaddress = &head_dict.config_file.ipaddress;
  let media_server = &head_dict.media_server;
  let user_id = &head_dict.config_file.user_id;

  // nextup & resume
  let mut item_list: Vec<Item> = vec![];
  let pick: Option<i32>;
  let nextup = puddler_get(format!("{}{}/Users/{}/Items/Resume?Fields=PremiereDate,MediaSources&MediaTypes=Video&Limit=15", &ipaddress, &media_server, &user_id), head_dict);
  let response: ItemJson = match nextup {
    Ok(mut t) => {
      let response_text = &t.text().unwrap();
      serde_json::from_str(response_text).unwrap()
    }
    Err(e) => {
      println!("Your network connection seems to be limited. Error: {e}\nUnable to continue.");
      process::exit(0x0100);
    }
  };

  if response.TotalRecordCount.unwrap() != 0 {
    println!("\nContinue Watching:");
    item_list = print_menu(&response, true, item_list);
  }
  
  if media_server != "/emby" {
    let jellyfin_nextup = puddler_get(format!("{}{}/Shows/NextUp?Fields=PremiereDate,MediaSources&UserId={}", &ipaddress, &media_server, &user_id), head_dict);
    let jellyfin_response: ItemJson = match jellyfin_nextup {
      Ok(mut t) => {
        let jellyfin_response_text = &t.text().unwrap();
        serde_json::from_str(jellyfin_response_text).unwrap()
      }
      Err(e) => panic!("failed to parse get request: {e}")
    };
    if jellyfin_response.TotalRecordCount.unwrap() != 0 {
      if response.TotalRecordCount.unwrap() == 0 {
        println!("\nContinue Watching:");
      }
      item_list = print_menu(&jellyfin_response, true, item_list);
    }
  }

  // latest
  let latest_series = puddler_get(format!("{}{}/Users/{}/Items/Latest?Limit=10&IncludeItemTypes=Episode&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id), head_dict);
  let latest_series_response: ItemJson = match latest_series {
    Ok(mut t) => {
      let response_text = format!("{{\"Items\":{}}}", t.text().unwrap());
      serde_json::from_str(&response_text).unwrap()
    }
    Err(e) => panic!("failed to parse get request: {e}")
  };

  if !latest_series_response.Items.is_empty() {
    println!("\nLatest:");
    item_list = print_menu(&latest_series_response, true, item_list);
  }

  let latest = puddler_get(format!("{}{}/Users/{}/Items/Latest?Limit=10&IncludeItemTypes=Movie&Fields=PremiereDate,MediaSources", &ipaddress, &media_server, &user_id), head_dict);
  let latest_response: ItemJson = match latest {
    Ok(mut t) => {
      let response_text = format!("{{\"Items\":{}}}", t.text().unwrap());
      serde_json::from_str(&response_text).unwrap()
    }
    Err(e) => panic!("failed to parse get request: {e}")
  };

  if !latest_response.Items.is_empty() {
    if latest_series_response.Items.is_empty() {
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
    let all = puddler_get(format!("{}{}/Items?UserId={}&Recursive=true&IncludeItemTypes=Series,Movie&Fields=PremiereDate,MediaSources&collapseBoxSetItems=False", &ipaddress, &media_server, &user_id), head_dict);
    let all_response: ItemJson = match all {
      Ok(mut t) => {
        let response_text: &String = &t.text().unwrap();
        serde_json::from_str(response_text).unwrap()
      }
      Err(e) => panic!("failed to parse get request: {e}")
    };
    let item_list = print_menu(&all_response, false, vec![]);

    if all_response.Items.len() > 1 {
      print!(": ");
      io::stdout().flush().expect("Failed to flush stdout");
    }
    pick = process_input(&item_list, None);
  } else if is_numeric(&input) {
    pick = process_input(&item_list, Some(input.trim().to_string()));
  } else {
    input = encode(input.trim()).into_owned();
    let search = puddler_get(format!("{}{}/Items?SearchTerm={}&UserId={}&Recursive=true&IncludeItemTypes=Series,Movie&Fields=PremiereDate,MediaSources&collapseBoxSetItems=False", &ipaddress, &media_server, &input, &user_id), head_dict);
    let search_response: ItemJson = match search {
      Ok(mut t) => {
        let search_text: &String = &t.text().unwrap();
        serde_json::from_str(search_text).unwrap()
      }
      Err(e) => panic!("failed to parse get request: {e}")
    };

    if !search_response.Items.is_empty() {
      item_list = print_menu(&search_response, false, vec![]);
      if search_response.Items.len() > 1 {
        print!(": ");
        io::stdout().flush().expect("Failed to flush stdout");
      }
      pick = process_input(&item_list, None);
    } else {
      println!("\nNo results found for: {}.", input.to_string().bold());
      pick = None
    }
  }

  if let Some(pick) = pick {
    item_parse(head_dict, &item_list, pick, settings);
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
  let result = match response.status() {
    StatusCode::OK => {
      response
    }
    _ => panic!("{} your server is missing some api endpoints, i think", response.status())
  };
  Ok(result)
}


fn is_numeric(input: &str) -> bool {
  if input.is_empty() {
    return false;
  }
  for x in input.trim().chars() {
    if x.is_alphabetic() {
      return false
    }
  }
  true
}


fn process_input(item_list: &Vec<Item>, number: Option<String>) -> Option<i32> {
  let items_in_list = item_list.len().try_into().unwrap();
  match items_in_list {
    n if n > 1 => {
      let mut raw_input: String;
      if let Some(res) = number.as_ref() {
        raw_input = res.to_string();
      } else {
        raw_input = String::new();
        io::stdin().read_line(&mut raw_input).unwrap();
        raw_input = raw_input.trim().to_string();
      }
      
      let pick = if is_numeric(raw_input.trim()) {
        raw_input.trim().parse::<i32>().unwrap()
      } else {
        item_list.iter().position(|i| !i.UserData.Played).unwrap() as i32
      };

      if pick < items_in_list + 1 && pick >= 0 {
        let item = item_list.get(pick as usize).unwrap();
        if item.SeasonName == Some("Specials".to_string()) {
          let first_occurence = item_list.iter().position(|i| i.Id == item.Id);
          let first = first_occurence == Some(pick as usize);
          let embedded: ColoredString = if number.is_some() || ! first {
            "Embedded".bold()
          } else {
            "Embedded".strikethrough()
          };
          println!("\nYou've chosen {}. ({})\n",
            format!("{} ({}) - {} - {}", item.SeriesName.as_ref().unwrap(),
              (&item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
              item.SeasonName.as_ref().unwrap(),
              item.Name
            ).cyan(),
            embedded
          );
        } else if item.Type == "Episode" {
          println!("\nYou've chosen {}.\n",
            format!("{} ({}) - {} - {}", item.SeriesName.as_ref().unwrap(),
              (&item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
              item.SeasonName.as_ref().unwrap(),
              item.Name
            ).cyan()
          );
        } else {
          println!("\nYou've chosen {}.\n", format!("{} ({})", item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]).cyan());
        }
      } else {
        println!("{}", "Are you ok?!".red());
        process::exit(0x0100);
      }
      Some(pick)
    },
    1 => {
      let mut raw_input = String::new();
      io::stdin().read_line(&mut raw_input).unwrap();
      let pick: i32 = 0;
      Some(pick)
    },
    _ => None
  }
}


fn item_parse(head_dict: &HeadDict, item_list: &[Item], pick: i32, settings: &Settings) {
  let ipaddress: &String = &head_dict.config_file.ipaddress;
  let media_server: &String = &head_dict.media_server;
  let user_id: &String = &head_dict.config_file.user_id;

  if item_list.get(pick as usize).unwrap().Type == *"Movie" {
    let item = &mut item_list.get(pick as usize).unwrap().clone();
    play(settings, head_dict, item);
  } else if item_list.get(pick as usize).unwrap().Type == *"Series" {
    let series = &item_list.get(pick as usize).unwrap();
    println!("{}:", series.Name);
    let series_response = puddler_get(format!("{}{}/Users/{}/Items?ParentId={}&Fields=PremiereDate,MediaSources&collapseBoxSetItems=False", &ipaddress, &media_server, &user_id, &series.Id), head_dict);
    let series_json: SeriesStruct = match series_response {
      Ok(mut t) => {
        let parse_text: &String = &t.text().unwrap();
        serde_json::from_str(parse_text).unwrap()
      }
      Err(e) => panic!("failed to parse series request: {e}")
    };

    let item_list: Vec<Item> = process_series(&series_json, head_dict, true);
    let items_in_list: i32 = item_list.len().try_into().unwrap();

    let filtered_input: i32 = if items_in_list > 1 {
      loop {
        print!("Enter which episode you want to play, or use the \"mark\" command to mark something as played. (\"2\", \"2-6\", \"2,3,6\")\n: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.contains("mark") {
          let played: bool;
          let parameters: String;
          if input.contains("unmark") {
            parameters = input.replace("unmark", "");
            played = false;
          } else {
            parameters = input.replace("mark ", "");
            played = true;
          };
          let mut temp = 0;
          let mut indexes: Vec<u32> = vec![];
          let mut range: bool = false;
          for ch in parameters.chars() {
            if ch == ',' {
              indexes.append(&mut vec![temp]);
            } else if ch == '-' {
              range = true;
            } else if range {
              for num in temp + 1..ch.to_digit(10).unwrap() + 1 {
                indexes.append(&mut vec![num]);
              }
              range = false;
            } else if ch.is_alphanumeric() {
              temp = ch.to_digit(10).unwrap();
            }
          }
          indexes.append(&mut vec![temp]);
          mark_items(&item_list, indexes, played, head_dict);
          continue;
        } else {
          break process_input(&item_list, Some(input)).unwrap();
        }
      }
    } else {
      0
    };
    series_play(&item_list, filtered_input, head_dict, settings);
  } else if "Episode".to_string().contains(&item_list.get(pick as usize).unwrap().Type) {
    let item: &Item = item_list.get(pick as usize).unwrap();
    let series_response = puddler_get(format!("{}{}/Users/{}/Items?ParentId={}&Fields=PremiereDate,MediaSources&collapseBoxSetItems=False", &ipaddress, &media_server, &user_id, &item.SeriesId.as_ref().unwrap()), head_dict);
    let series_json: SeriesStruct = match series_response {
      Ok(mut t) => {
        let parse_text: &String = &t.text().unwrap();
        serde_json::from_str(parse_text).unwrap()
      }
      Err(e) => panic!("failed to parse series request: {e}")
    };
    let item_list: Vec<Item> = process_series(&series_json, head_dict, false);
    let mut item_pos: i32 = 0;
    let mut amount = item_list.iter().filter(|&i| i.Id == item.Id).count(); // how many times the episode exists in the list
    if item.SeasonName == Some("Specials".to_string()) && amount > 1 {
      for (things, item1) in item_list.iter().enumerate() {
        if item1.Id == item.Id {
          if amount == 1 {
            item_pos = things.try_into().unwrap();
            break;
          } else {
            amount -= 1;
          }
        }
      }
    } else {
      for (things, item1) in item_list.iter().enumerate() {
        if item1.Id == item.Id {
          item_pos = things.try_into().unwrap();
          break;
        }
      };
    }
    series_play(&item_list, item_pos, head_dict, settings);
  }
}


fn mark_items(item_list: &[Item], indexes: Vec<u32>, played: bool, head_dict: &HeadDict) {
  println!();
  for index in indexes {
    let item = item_list.get(index as usize).unwrap();
    if played {
      println!("Marking {} as played.", item.Name.cyan());
    } else {
      println!("Marking {} as un-played.", item.Name.cyan());
    }
    mark_playstate(head_dict, item, played);
  }
  println!();
}


fn series_play(item_list: &Vec<Item>, mut pick: i32, head_dict: &HeadDict, settings: &Settings) {
  let episode_amount: i32 = item_list.len().try_into().unwrap();
  let item = &mut item_list.get(pick as usize).unwrap().clone();
  let watched_full_item: bool = play(settings, head_dict, item);
  loop {
    if ( pick + 2 ) > episode_amount { // +1 since episode_amount doesn't start at 0 AND +1 for next ep
      println!("\nYou've reached the end of your episode list. Returning to menu ...");
      break
    } else {
      pick += 1;
      if item_list.get(pick as usize).is_some() {
        let next_item = &item_list.get(pick as usize).unwrap();
        
        if next_item.UserData.Played {
          continue
        };

        if ! watched_full_item {
          println!("\nWelcome back. Do you want to finish the current episode or play the next one?:\n{}",
            format!("   {} ({}) - {} - {}",
              next_item.SeriesName.as_ref().unwrap(),
              &next_item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4],
              next_item.SeasonName.as_ref().unwrap(), next_item.Name).cyan()
          );
          print!(" (F)inish episode | (M)ark watched | (N)ext episode | (R)eturn to menu | (E)xit");
          let cont = getch("FfRrNnEeMm");
          match cont {
            'F' | 'f' => {
              pick -= 1;
              play(settings, head_dict, item);
            },
            'M' | 'm' => {
              pick -= 1;
              mark_items(item_list, vec![(pick) as u32], true, head_dict);
            },
            'N' | 'n' => {
              let item = &mut item_list.get(pick as usize).unwrap().clone();
              play(settings, head_dict, item);
            },
            'R' | 'r' => break,
            'E' | 'e' => {
              process::exit(0x0100);
            },
            _ => (),
          }
        } else if settings.autoplay {
          println!("\nWelcome back. Continuing playback in 5 seconds:\n{}",
            format!("   {} ({}) - {} - {}",
              next_item.SeriesName.as_ref().unwrap(),
              &next_item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4],
              next_item.SeasonName.as_ref().unwrap(), next_item.Name).cyan()
          );
          thread::sleep(Duration::from_secs(5));
          let item = &mut item_list.get(pick as usize).unwrap().clone();
          play(settings, head_dict, item);
        } else {
          println!("\nWelcome back. Do you want to continue playback with:\n{}",
            format!("   {} ({}) - {} - {}",
              next_item.SeriesName.as_ref().unwrap(),
              &next_item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4],
              next_item.SeasonName.as_ref().unwrap(), next_item.Name).cyan()
          );
          print!(" (N)ext | (R)eturn to menu | (E)xit");
          let cont = getch("RrNnEe");
          match cont {
            'N' | 'n' => {
              let item = &mut item_list.get(pick as usize).unwrap().clone();
              play(settings, head_dict, item);
            },
            'R' | 'r' => break,
            'E' | 'e' => {
              process::exit(0x0100);
            },
            _ => (),
          }
        }
      } else {
        break
      }
    }
  }
}


fn process_series(series: &SeriesStruct, head_dict: &HeadDict, printing: bool) -> Vec<Item> {
  let ipaddress: &String = &head_dict.config_file.ipaddress;
  let media_server: &String = &head_dict.media_server;
  let user_id: &String = &head_dict.config_file.user_id;
  let mut index_iterator: i32 = 0;
  let mut episode_list: Vec<Item> = Vec::new();

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
    let season_res = puddler_get(format!("{}{}/Users/{}/Items?ParentId={}&Fields=PremiereDate,MediaSources&collapseBoxSetItems=False", &ipaddress, &media_server, &user_id, &season.Id), head_dict);
    let season_json: SeasonStruct = match season_res {
      Ok(mut t) => {
        let parse_text: &String = &t.text().unwrap().to_string();
        serde_json::from_str(parse_text).unwrap()
      }
      Err(e) => panic!("failed to parse series request: {e}")
    };
    for episode_numb in 0..season_json.Items.len() { // for the code readers: the "season_json" vector is obviously different to "season" since the latter doesn't include any episodes.
      let episode: Item = season_json.Items[episode_numb].clone();
      let last_episode = season_json.Items.len() == episode_numb + 1;
      let episode_branches = if last_episode && last_season {
        "     └──"
      } else if last_episode && ! last_season {
        "│    └──"
      } else if ! last_episode && last_season {
        "     ├──"
      } else {
        "│    ├──"
      };
      if ! episode_list.contains(&episode) || episode.SeasonName == Some("Specials".to_string()) {
        episode_list.push(season_json.Items[episode_numb].clone());
      }
      if ! printing {
        continue
      };
      let extra = if episode.SeasonName != Some(season.Name.clone()) { // If the special is listed in a normal season, the season name of it is different from the actual season which the special is assigned to (kinda makes sense to avoid duplicate items)
        " (S)".to_string()
      } else {
        "".to_string()
      };
      if episode.UserData.PlayedPercentage.is_some() {
        let long_perc: f64 = episode.UserData.PlayedPercentage.unwrap();
        println!("  {} [{}] {}{} {}% ", episode_branches, index_iterator, episode.Name, extra, long_perc.round() as i64)
      } else if episode.UserData.Played {
        println!("  {} [{}] {}{} {} ", episode_branches, index_iterator, episode.Name, extra, "[PLAYED]".to_string().green());
      } else {
        println!("  {} [{}] {}{}", episode_branches, index_iterator, episode.Name, extra);
      };
      index_iterator += 1;
    }
  };
  episode_list
}


fn print_menu(items: &ItemJson, recommendation: bool, mut item_list: Vec<Item>) -> Vec<Item> {
  let count: usize = if recommendation {
    2
  } else {
    items.Items.len()
  };
  if count > 1 && ! recommendation {
    println!("\nPlease choose from the following results:")
  }
  for h in 0..items.Items.len() {
    let x: Item = items.Items[h].clone();
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
