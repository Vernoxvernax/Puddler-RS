use async_trait::async_trait;
use crossterm::{
  cursor::{EnableBlinking, Hide, MoveToColumn, RestorePosition, SavePosition, Show},
  event::{Event, KeyCode, KeyEvent, KeyModifiers, poll, read},
  execute,
  style::Stylize,
  terminal::{
    self, Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
    LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
  },
};
use isolanguage_1::LanguageCode;
use regex::Regex;
use reqwest::{
  StatusCode,
  blocking::{Client, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
  fmt,
  io::{Write, stdin, stdout},
  net::UdpSocket,
  process::exit,
  str::{FromStr, from_utf8},
  sync::{Arc, Mutex},
  thread,
  time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  APPNAME, VERSION,
  emby::EmbyServer,
  input::{
    InteractiveOption, InteractiveOptionType, SeriesOptions, getch, hidden_string_input,
    interactive_select, jelly_series_select, take_string_input,
  },
  jellyfin::JellyfinServer,
  media_config::{Config, MediaCenterType, Objective, UserConfig},
  mpv::Player,
  plex::PlexServer,
  printing::{PrintMessageType, print_message},
  puddler_settings::PuddlerSettings,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UDPAnswer {
  pub Address: String,
  pub Name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserCredentials {
  pub username: String,
  pub password: String,
}

#[derive(Debug)]
pub enum MediaCenterValues {
  Header,
  SessionID,
  PlaybackInfo,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Item {
  pub Name: String,
  pub Id: String,
  pub IndexNumber: Option<u32>,
  pub IndexNumberEnd: Option<u32>,
  pub ParentIndexNumber: Option<u32>,
  pub RunTimeTicks: Option<u64>,
  pub Type: String,
  pub UserData: UserData,
  pub SeriesName: Option<String>,
  pub SeriesId: Option<String>,
  pub SeasonName: Option<String>,
  pub SeasonId: Option<String>,
  pub PremiereDate: Option<String>,
  pub ProductionYear: Option<u32>,
  pub Status: Option<String>,
  pub EndDate: Option<String>,
  pub MediaSources: Option<Vec<MediaSourceInfo>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct MediaSourceInfo {
  pub Id: String,
  pub Path: Option<String>,
  pub SupportsTranscoding: bool,
  pub MediaStreams: Vec<MediaStream>,
  pub Bitrate: Option<u64>,
  pub TranscodingUrl: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct MediaStream {
  pub Index: u32,
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
  pub Path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct PlayRequest {
  CanSeek: bool,
  ItemId: String,
  SessionId: String,
  MediaSourceId: String,
  AudioStreamIndex: u32,
  SubtitleStreamIndex: u32,
  IsPaused: bool,
  IsMuted: bool,
  PlaybackStartTimeTicks: u64,
  PlaySessionId: String,
  PlayMethod: PlayMethod,
  RepeatMode: RepeatMode,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Capabilities {
  PlayableMediaTypes: String,
  SupportsMediaControl: bool,
  SupportedCommands: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
enum EventName {
  TimeUpdate,
  Pause,
  Unpause,
  VolumeChange,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct PlaybackStopInfo {
  ItemId: String,
  PlaySessionId: String,
  SessionId: String,
  MediaSourceId: String,
  PositionTicks: String,
  Failed: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct PlaybackProgressInfo {
  CanSeek: bool,
  ItemId: String,
  SessionId: String,
  MediaSourceId: String,
  AudioStreamIndex: u32,
  SubtitleStreamIndex: u32,
  IsPaused: bool,
  IsMuted: bool,
  PositionTicks: u64,
  VolumeLevel: u32,
  PlayMethod: PlayMethod,
  PlaySessionId: String,
  EventName: EventName,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
enum PlayMethod {
  Transcode,
  DirectStream,
  DirectPlay,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
enum RepeatMode {
  RepeatNone,
  RepeatAll,
  RepeatOne,
}

impl fmt::Display for MediaStream {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.IsDefault {
      write!(
        f,
        "Title = \"{}\", Language = \"{}\", Codec = \"{}\" {}",
        self
          .Title
          .as_ref()
          .unwrap_or(self.DisplayTitle.as_ref().unwrap_or(&String::new())),
        self
          .DisplayLanguage
          .as_ref()
          .unwrap_or(self.Language.as_ref().unwrap_or(&"undefined".to_string())),
        self
          .Codec
          .as_ref()
          .unwrap_or(&"???".to_string())
          .to_uppercase(),
        "[Default]".to_string().green()
      )
    } else {
      write!(
        f,
        "Title = \"{}\", Language = \"{}\", Codec = \"{}\"",
        self
          .Title
          .as_ref()
          .unwrap_or(self.DisplayTitle.as_ref().unwrap_or(&String::new())),
        self
          .DisplayLanguage
          .as_ref()
          .unwrap_or(self.Language.as_ref().unwrap_or(&"undefined".to_string())),
        self
          .Codec
          .as_ref()
          .unwrap_or(&"???".to_string())
          .to_uppercase()
      )
    }
  }
}

pub trait IsNumeric {
  fn is_numeric(&self) -> bool;
}

impl IsNumeric for str {
  fn is_numeric(&self) -> bool {
    if self.is_empty() {
      return false;
    }
    for ch in self.chars() {
      if !ch.is_ascii_digit() {
        return false;
      }
    }
    true
  }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct UserData {
  pub PlayedPercentage: Option<f64>,
  pub PlaybackPositionTicks: u64,
  pub Played: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionCapabilities {
  UserId: String,
  StartTimeTicks: u64,
  MediaSourceId: String,
  AudioStreamIndex: u32,
  SubtitleStreamIndex: u32,
  MaxStaticBitrate: u64,
  MaxStreamingBitrate: u64,
  EnableDirectPlay: bool,
  EnableDirectStream: bool,
  EnableTranscoding: bool,
  AllowVideoStreamCopy: bool,
  AllowAudioStreamCopy: bool,
  DeviceProfile: DeviceProfile,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeviceProfile {
  Name: String,
  Id: String,
  SupportedMediaTypes: String,
  MaxStreamingBitrate: u64,
  MaxStaticMusicBitrate: u64,
  TranscodingProfiles: Vec<TranscodingProfile>,
  SubtitleProfiles: Vec<SubtitleProfile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirectPlayProfile {
  Type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TranscodingProfile {
  Container: String,
  Type: String,
  VideoCodec: String,
  TranscodeSeekInfo: String,
  Context: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SubtitleProfile {
  Format: String,
  Method: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PlaybackInfo {
  pub MediaSources: Vec<MediaSourceInfo>,
  pub PlaySessionId: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UserDto {
  Configuration: UserConfiguration,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UserConfiguration {
  PlayDefaultAudioTrack: bool,
  AudioLanguagePreference: Option<String>,
  SubtitleLanguagePreference: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Series {
  item_id: String,
  pub seasons: Vec<Season>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Season {
  pub item: Item,
  pub episodes: Vec<Item>,
}

pub trait ToStringAdv {
  fn to_string_split(&self) -> Vec<String>;
  fn to_string_full(&self) -> String;
  fn to_string_ext(&self) -> String;
}

// Provides a few functions to output the proper title of the Item
// with some additional changes.
// * `to_string_split` -> same as `to_string()`, but split into a vector (for easier management in player.rs)
// * `to_string_full` -> `to_string()` with an additional on/off indicator, whether the Item has been played already
// * `to_string_ext` -> same as `to_string_full()`, but using a percentage indicator
impl ToStringAdv for Item {
  fn to_string_split(&self) -> Vec<String> {
    let time = if let (Some(start), Some(end)) = (self.PremiereDate.clone(), self.EndDate.clone()) {
      if start[0..4] == end[0..4] {
        format!("({})", &start[0..4])
      } else {
        format!("({}-{})", &start[0..4], &end[0..4])
      }
    } else if self.Status == Some(String::from("Continuing")) {
      format!(
        "({}-)",
        &self.PremiereDate.clone().unwrap_or(String::from("????"))[0..4]
      )
    } else if let Some(premiere_date) = &self.PremiereDate {
      format!("({})", &premiere_date[0..4])
    } else if let Some(production_year) = &self.ProductionYear {
      format!("({})", production_year)
    } else {
      "(???)".to_string()
    };
    let mut name: String;
    match self.Type.as_str() {
      "Season" | "Episode" => name = self.SeriesName.clone().unwrap_or(String::from("???")),
      _ => name = self.Name.clone(),
    }
    if name.contains('(') {
      let re = Regex::new(r" \(\d{4}\)").unwrap();
      name = re.replace_all(&name, String::new()).to_string();
    }

    match self.Type.as_str() {
      "Movie" | "Series" => {
        vec![format!("{} {}", name, time)]
      },
      "Season" => {
        vec![
          self.to_string(),
          format!("{} {}", name, time),
          self.Name.clone(),
        ]
      },
      "Episode" => match self.IndexNumberEnd {
        Some(indexend) => {
          vec![
            self.to_string(),
            format!("{} {}", name, time),
            format!(
              "S{:02}E{:02}-{:02} ({})",
              self.ParentIndexNumber.unwrap_or(0),
              self.IndexNumber.unwrap_or(0),
              indexend,
              self.Name
            ),
          ]
        },
        None => {
          vec![
            self.to_string(),
            format!("{} {}", name, time),
            format!(
              "S{:02}E{:02} ({})",
              self.ParentIndexNumber.unwrap_or(0),
              self.IndexNumber.unwrap_or(0),
              self.Name
            ),
          ]
        },
      },
      _ => vec![format!("{} {} (unknown media type)", self.Name, time)],
    }
  }

  fn to_string_ext(&self) -> String {
    let full = self.to_string_full();
    if let Some(percentage) = self.UserData.PlayedPercentage {
      format!("{} {}%", full, percentage.round())
    } else {
      full
    }
  }

  fn to_string_full(&self) -> String {
    let basic = self.to_string();
    if self.UserData.Played {
      format!("{} - {}", basic, "(Played)".green())
    } else {
      basic
    }
  }
}

// Properly compiles titles, dates and other metadata into one string.
impl ToString for Item {
  fn to_string(&self) -> String {
    let time = if let (Some(start), Some(end)) = (self.PremiereDate.clone(), self.EndDate.clone()) {
      if start[0..4] == end[0..4] {
        format!("({})", &start[0..4])
      } else {
        format!("({}-{})", &start[0..4], &end[0..4])
      }
    } else if self.Status == Some(String::from("Continuing")) {
      format!(
        "({}-)",
        &self.PremiereDate.clone().unwrap_or(String::from("????"))[0..4]
      )
    } else if let Some(premiere_date) = &self.PremiereDate {
      format!("({})", &premiere_date[0..4])
    } else if let Some(production_year) = &self.ProductionYear {
      format!("({})", production_year)
    } else {
      "(???)".to_string()
    };
    let mut name: String;
    match self.Type.as_str() {
      "Season" | "Episode" => name = self.SeriesName.clone().unwrap_or(String::from("???")),
      _ => name = self.Name.clone(),
    }
    if name.contains('(') {
      let re = Regex::new(r" \(\d{4}\)").unwrap();
      name = re.replace_all(&name, String::new()).to_string();
    }

    match self.Type.as_str() {
      "Movie" | "Series" => {
        format!("{} {}", name, time)
      },
      "Season" => {
        format!("{} {} - {}", name, time, self.Name.clone())
      },
      "Episode" => match self.IndexNumberEnd {
        Some(indexend) => {
          format!(
            "{} {} - S{:02}E{:02}-{:02} - {}",
            name,
            time,
            self.ParentIndexNumber.unwrap_or(0),
            self.IndexNumber.unwrap_or(0),
            indexend,
            self.Name
          )
        },
        None => {
          format!(
            "{} {} - S{:02}E{:02} - {}",
            name,
            time,
            self.ParentIndexNumber.unwrap_or(0),
            self.IndexNumber.unwrap_or(0),
            self.Name
          )
        },
      },
      _ => format!("{} {} (unknown media type)", self.Name, time),
    }
  }
}

pub fn set_config(handle: Config, settings: PuddlerSettings) -> Box<dyn MediaCenter> {
  match handle.config.media_center_type {
    MediaCenterType::Emby => Box::new(EmbyServer::new(handle, settings)),
    MediaCenterType::Jellyfin => Box::new(JellyfinServer::new(handle, settings)),
    _ => Box::new(PlexServer::new(handle, settings)),
  }
}

// Defaulting to a Jellyfin/Emby instance. Other API's need to re-implement several if not all traits and structs for full functionality.
#[async_trait]
pub trait MediaCenter: Send {
  fn new(config: Config, settings: PuddlerSettings) -> Self
  where
    Self: Sized;

  fn get_config_handle(&mut self) -> &mut Config;
  fn get_headers(&mut self) -> Vec<(String, String)>;
  fn get_settings(&mut self) -> &mut PuddlerSettings;

  fn modify(&mut self) {
    loop {
      let handle = self.get_config_handle();
      let config = &mut handle.config;
      let users = serde_json::from_value::<Vec<UserConfig>>(
        config.specific_values.get("users").unwrap().clone(),
      )
      .unwrap();
      let user_name_list = users
        .iter()
        .map(|u| u.username.clone())
        .collect::<Vec<String>>()
        .join(":");
      let transcoding = if config.transcoding {
        format!("{}", "Enabled".green())
      } else {
        format!("{}", "Disabled".red())
      };
      let settings = vec![
        InteractiveOption {
          text: format!("Settings: {}", config.server_name.clone().cyan()),
          option_type: InteractiveOptionType::Header,
        },
        InteractiveOption {
          text: String::from("Set default user:") + &user_name_list,
          option_type: InteractiveOptionType::ListButtons,
        },
        InteractiveOption {
          text: String::from("Add User"),
          option_type: InteractiveOptionType::Button,
        },
        InteractiveOption {
          text: String::from("Delete User:") + &user_name_list,
          option_type: InteractiveOptionType::ListButtons,
        },
        InteractiveOption {
          text: String::from("Transcoding:") + &transcoding,
          option_type: InteractiveOptionType::ListButtons,
        },
        InteractiveOption {
          text: String::from("Change Name"),
          option_type: InteractiveOptionType::Button,
        },
        InteractiveOption {
          text: format!("{}", "Save".green()),
          option_type: InteractiveOptionType::Button,
        },
        InteractiveOption {
          text: format!("{}", "Delete".red()),
          option_type: InteractiveOptionType::Button,
        },
        InteractiveOption {
          text: String::from("Back"),
          option_type: InteractiveOptionType::Special,
        },
      ];
      match interactive_select(settings) {
        (_, _, InteractiveOptionType::Special) => break,
        ((i1, i2), _, InteractiveOptionType::ListButtons) => {
          if i1 == 0 {
            handle.set_active_user(users[i2 - 1].clone().access_token);
          } else if i1 == 2 {
            if users.len() == 1 {
              print_message(
                PrintMessageType::Error,
                "Please add a second user before deleting this one.",
              );
            } else {
              handle.remove_user(users[i2 - 1].clone().access_token);
            }
          } else if i1 == 3 {
            config.transcoding = !config.transcoding;
          }
        },
        ((i1, _), _, InteractiveOptionType::Button) => {
          if i1 == 1 {
            self.login();
          } else if i1 == 4 {
            self
              .get_config_handle()
              .ask_for_setting(Objective::ServerName);
            self.get_config_handle().save();
          } else if i1 == 5 {
            handle.save();
          } else if i1 == 6 {
            handle.delete();
            break;
          }
        },
        _ => (),
      }
    }
  }

  fn menu(&mut self) {
    let media_center_type = self.get_config_handle().config.media_center_type;
    let user = self.get_config_handle().get_active_user().unwrap();
    let mut stdout = stdout();
    execute!(stdout, Hide).unwrap();
    print!("Loading menu [0/4]");
    stdout.flush().unwrap();

    let mut total: Vec<Item> = vec![];
    let mut options: Vec<InteractiveOption> = vec![];
    if let Ok(items) = self.get_items(
      format!(
        "Users/{}/Items/Resume?Limit=15&MediaTypes=Video",
        user.user_id
      ),
      false,
    ) {
      if !items.is_empty() {
        options.append(&mut vec![InteractiveOption {
          text: String::from("Continue Watching:"),
          option_type: InteractiveOptionType::Header,
        }]);
      }
      for item in items.clone() {
        options.append(&mut vec![InteractiveOption {
          text: item.to_string_ext(),
          option_type: InteractiveOptionType::Button,
        }]);
      }
      total.extend(items);
    } else {
      exit(1);
    };

    enable_raw_mode().unwrap();
    execute!(stdout, MoveToColumn(0)).unwrap();
    disable_raw_mode().unwrap();
    print!("Loading menu [1/4]");
    stdout.flush().unwrap();

    if media_center_type == MediaCenterType::Jellyfin {
      if let Ok(mut items) = self.get_items(format!("Shows/NextUp?UserId={}", user.user_id), false)
      {
        if total.is_empty() && !items.is_empty() {
          options.append(&mut vec![InteractiveOption {
            text: String::from("Continue Watching:"),
            option_type: InteractiveOptionType::Header,
          }]);
        }
        items.retain(|i| !total.contains(i));
        for item in items.clone() {
          options.append(&mut vec![InteractiveOption {
            text: item.to_string_ext(),
            option_type: InteractiveOptionType::Button,
          }]);
        }
        total.extend(items);
      } else {
        exit(1);
      };
    }

    enable_raw_mode().unwrap();
    execute!(stdout, MoveToColumn(0)).unwrap();
    disable_raw_mode().unwrap();
    print!("Loading menu [2/4]");
    stdout.flush().unwrap();

    let latest_episode_size = if let Ok(items) = self.get_items(
      format!(
        "Users/{}/Items/Latest?Limit=10&IncludeItemTypes=Episode",
        user.user_id
      ),
      true,
    ) {
      if !total.is_empty() {
        options.append(&mut vec![InteractiveOption {
          text: String::new(),
          option_type: InteractiveOptionType::Header,
        }]);
      }
      if !items.is_empty() {
        options.append(&mut vec![InteractiveOption {
          text: String::from("Latest:"),
          option_type: InteractiveOptionType::Header,
        }]);
      }
      for item in items.clone() {
        options.append(&mut vec![InteractiveOption {
          text: item.to_string_ext(),
          option_type: InteractiveOptionType::Button,
        }]);
      }
      total.extend(items.clone());
      items.len()
    } else {
      exit(1);
    };

    enable_raw_mode().unwrap();
    execute!(stdout, MoveToColumn(0)).unwrap();
    disable_raw_mode().unwrap();
    print!("Loading menu [3/4]");
    stdout.flush().unwrap();

    if let Ok(items) = self.get_items(
      format!(
        "Users/{}/Items/Latest?Limit=10&IncludeItemTypes=Movie",
        user.user_id
      ),
      true,
    ) {
      if latest_episode_size == 0 && !items.is_empty() {
        options.append(&mut vec![InteractiveOption {
          text: String::from("Latest:"),
          option_type: InteractiveOptionType::Header,
        }]);
      }
      for item in items.clone() {
        options.append(&mut vec![InteractiveOption {
          text: item.to_string_ext(),
          option_type: InteractiveOptionType::Button,
        }]);
      }
      total.extend(items);
    } else {
      exit(1);
    };

    enable_raw_mode().unwrap();
    execute!(stdout, MoveToColumn(0), Clear(ClearType::FromCursorDown)).unwrap();
    disable_raw_mode().unwrap();

    options.append(&mut vec![
      InteractiveOption {
        text: String::new(),
        option_type: InteractiveOptionType::Header,
      },
      InteractiveOption {
        text: String::from("Search"),
        option_type: InteractiveOptionType::TextInput,
      },
      InteractiveOption {
        text: format!("Return to {} Menu", APPNAME),
        option_type: InteractiveOptionType::Special,
      },
    ]);

    let menu = options.clone();

    let mut current_items = total.clone();
    loop {
      match interactive_select(options.clone()) {
        (selection, _, InteractiveOptionType::Button) => {
          self.process_item(current_items[selection.0].clone());
          continue;
        },
        (_, Some(mut search), InteractiveOptionType::TextInput) => {
          search = search.trim().to_owned();
          if let Ok(items) = self.get_items(
            format!(
              "Items?SearchTerm={}&UserId={}&Recursive=true&IncludeItemTypes=Series,Movie",
              urlencoding::encode(&search),
              user.user_id
            ),
            false,
          ) {
            options.clear();
            options.append(&mut vec![InteractiveOption {
              text: format!("Search-Result for \"{}\":", search.cyan()),
              option_type: InteractiveOptionType::Header,
            }]);
            for item in items.clone() {
              options.append(&mut vec![InteractiveOption {
                text: item.to_string_ext(),
                option_type: InteractiveOptionType::Button,
              }]);
            }
            options.append(&mut vec![
              InteractiveOption {
                text: String::from("Back"),
                option_type: InteractiveOptionType::Special,
              },
              InteractiveOption {
                text: format!("Return to {} Menu", APPNAME),
                option_type: InteractiveOptionType::Special,
              },
            ]);
            current_items = items;
          } else {
            exit(1);
          };
        },
        (_, Some(option), InteractiveOptionType::Special) => {
          if option == *"Back" {
            options = menu.clone();
            current_items = total.clone();
          } else if option == format!("Return to {} Menu", APPNAME) {
            return;
          }
        },
        _ => panic!("UNKOWN OPTION TYPE"),
      }
    }
  }

  fn process_item(&mut self, item: Item) {
    println!("Selected: {}", item.to_string_ext().cyan());
    let mut playlist: Vec<Item> = vec![];
    match item.Type.as_str() {
      "Movie" => {
        playlist.push(item);
      },
      "Episode" => {
        let series = self.resolve_series(item.clone());
        let mut found = false;
        for season in series.seasons {
          if found {
            playlist.extend(season.episodes);
          } else {
            for episode in season.episodes {
              if found {
                playlist.push(episode);
              } else if episode.Id == item.Id {
                found = true;
                playlist.push(episode);
              }
            }
          }
        }
      },
      "Series" | "Season" => {
        let series = self.resolve_series(item);
        playlist = self.choose_from_series(series);
      },
      _ => (),
    }
    if playlist.is_empty() {
      return;
    }

    let headers = self.get_headers();
    let auth_token = &headers.get(2).unwrap().1;
    let server_address = self.get_address();

    let settings = self.get_settings().clone();
    let mut player = Player::new(self.get_config_handle().clone(), settings.clone());

    let mut transcoding_settings = None;
    let mut index = 0;
    let mut stdout = stdout();
    while index < playlist.len() {
      let item = playlist[index].clone();
      let mut next_index = index + 1;
      let mut streamable_item = item.clone();
      if let Ok(playback_info) =
        self.post_playbackinfo(&mut streamable_item, &mut transcoding_settings)
      {
        self.insert_value(
          MediaCenterValues::PlaybackInfo,
          serde_json::to_string(&playback_info).unwrap(),
        );
        self.update_player(&mut player);
        player.set_jellyfin_video(
          streamable_item,
          playback_info,
          server_address.clone(),
          auth_token.to_string(),
          &mut transcoding_settings,
        );
        let ret = player.play();
        if let Some((_, audio, subtitle, ..)) = transcoding_settings.as_mut() {
          *audio = ret.preferred_audio_track;
          *subtitle = ret.preferred_subtitle_track
        }
        'playback_done: loop {
          let mut options: Vec<InteractiveOption> = vec![];
          execute!(stdout, DisableLineWrap).unwrap();
          if !ret.played {
            if let Ok(updated_item) = self.get_item(item.Id.clone()) {
              playlist[index] = updated_item;
            } else {
              print_message(
                PrintMessageType::Error,
                format!(
                  "Failed to get updated information for {}.",
                  item.to_string()
                )
                .as_str(),
              )
            }
            options.append(&mut vec![
              InteractiveOption {
                text: format!("Finish: {}", item.to_string_ext()),
                option_type: InteractiveOptionType::Button,
              },
              InteractiveOption {
                text: format!("Mark as played: {}", item.to_string_ext()),
                option_type: InteractiveOptionType::Button,
              },
            ]);
          }
          while let Some(next_item) = playlist.get(index + 1) {
            // skip every item that has been played already
            // (might want to use unmark in the menu before watching a series again)
            if !next_item.UserData.Played {
              options.push(InteractiveOption {
                text: format!("Continue with: {}", next_item.to_string_ext()),
                option_type: InteractiveOptionType::Button5s,
              });
              break;
            }
            next_index += 1;
          }
          if options.is_empty() {
            print_message(
              PrintMessageType::Warning,
              "Playlist done. Returning to menu.",
            );
            return;
          }
          options.append(&mut vec![
            InteractiveOption {
              text: "Back to Menu".to_string(),
              option_type: InteractiveOptionType::Special,
            },
            InteractiveOption {
              text: "Exit Application".to_string(),
              option_type: InteractiveOptionType::Special,
            },
          ]);
          match interactive_select(options) {
            ((_, _), Some(text), InteractiveOptionType::Button) => {
              if text.starts_with("Finish") {
                transcoding_settings.as_mut().unwrap().0 = true;
                break 'playback_done;
              } else if text.starts_with("Mark") {
                self.item_set_playstate(item.Id.clone(), true);
                continue 'playback_done;
              } else if text.starts_with("Continue") {
                index = next_index;
                break 'playback_done;
              }
            },
            ((_, _), Some(text), InteractiveOptionType::Special) => match text.as_str() {
              "Back to Menu" => {
                execute!(stdout, EnableLineWrap).unwrap();
                return;
              },
              _ => {
                execute!(stdout, EnableLineWrap).unwrap();
                exit(0)
              },
            },
            _ => (),
          }
          execute!(stdout, EnableLineWrap).unwrap();
        }
      }
    }
  }

  fn update_player(&mut self, player: &mut Player);

  fn post_playbackinfo(
    &mut self,
    item: &mut Item,
    previous_settings: &mut Option<(bool, Option<u32>, Option<u32>, String)>,
  ) -> Result<PlaybackInfo, ()> {
    let mut handle = self.get_config_handle().clone();
    let user_id = handle.get_active_user().unwrap().user_id;
    let mut stdout = stdout();
    execute!(stdout, SavePosition).unwrap();

    let mut mediasource_index = 0;
    let mediasource_list: Vec<MediaSourceInfo> = if let Some(mediasources) = &item.MediaSources {
      mediasources.to_vec()
    } else {
      return Err(());
    };

    // This is the only setting which isn't saved across the playlist. Don't really see the point in that tbh.
    if mediasource_list.len() > 1 {
      let mut options: Vec<InteractiveOption> = vec![InteractiveOption {
        text: "\nPlease select from the following files:".to_string(),
        option_type: InteractiveOptionType::Header,
      }];
      for mediasource in mediasource_list.clone() {
        if let Some(path) = mediasource.Path {
          options.push(InteractiveOption {
            text: path.split_terminator('/').last().unwrap().to_string(),
            option_type: InteractiveOptionType::Button,
          });
        }
      }
      let ((index, _), ..) = interactive_select(options);
      mediasource_index = index;
      enable_raw_mode().unwrap();
      execute!(
        stdout,
        RestorePosition,
        MoveToColumn(0),
        Clear(ClearType::FromCursorDown)
      )
      .unwrap();
      disable_raw_mode().unwrap();
    }

    if previous_settings.is_none() && !handle.config.transcoding {
      let url = format!("Users/{}", user_id);
      match self.get(url) {
        Ok(res) => {
          let user = serde_json::from_str::<UserDto>(&res.text().unwrap()).unwrap();
          let mut audio_streams: Vec<MediaStream> = vec![];
          let mut subtitle_streams: Vec<MediaStream> = vec![];
          let audio_language = if let Some(pref) = user.Configuration.AudioLanguagePreference {
            if let Ok(lang) = LanguageCode::from_str(&pref) {
              Some(lang)
            } else {
              None
            }
          } else {
            None
          };
          let subtitle_language = if let Some(pref) = user.Configuration.SubtitleLanguagePreference
          {
            if let Ok(lang) = LanguageCode::from_str(&pref) {
              Some(lang)
            } else {
              None
            }
          } else {
            None
          };
          let mut audio_track = None;
          let mut subtitle_track = None;
          for stream in mediasource_list[mediasource_index].MediaStreams.clone() {
            if stream.Type == *"Audio" {
              audio_streams.push(stream.clone());
            } else if stream.Type == *"Subtitle" {
              subtitle_streams.push(stream.clone());
            }
          }
          if audio_language.is_some() {
            for (index, stream) in audio_streams.iter().enumerate() {
              if let Some(lang) = &stream.Language {
                if let Ok(lang_code) = LanguageCode::from_str(lang) {
                  if audio_language == Some(lang_code) {
                    audio_track = Some(index as u32 + 1);
                    break;
                  }
                }
              }
            }
          }
          if subtitle_language.is_some() {
            for (index, stream) in subtitle_streams.iter().enumerate() {
              if let Some(lang) = &stream.Language {
                if let Ok(lang_code) = LanguageCode::from_str(lang) {
                  if subtitle_language == Some(lang_code) {
                    subtitle_track = Some(index as u32 + 1);
                    break;
                  }
                }
              }
            }
          }
          *previous_settings = Some((false, audio_track, subtitle_track, String::new()));
        },
        Err(err) => {
          print_message(
            PrintMessageType::Error,
            format!("Failed to get user prefences: {}", err.status()).as_str(),
          );
        },
      }
    }

    if handle.config.transcoding {
      let time = (item.UserData.PlaybackPositionTicks as f64) / 10000000.0;
      let formated: String = if time > 60.0 {
        if (time / 60.0) > 60.0 {
          format!(
            "{:02}:{:02}:{:02}",
            ((time / 60.0) / 60.0).trunc(),
            ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(),
            (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
          )
        } else {
          format!(
            "00:{:02}:{:02}",
            (time / 60.0).trunc(),
            (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
          )
        }
      } else {
        time.to_string()
      };
      if !previous_settings
        .clone()
        .unwrap_or((false, Some(0), Some(0), String::new()))
        .0
      {
        print!(
          "\nDo you want to start at: {}?\n  (Y)es | (N)o",
          formated.cyan().bold()
        );
        match getch("YyNn") {
          'N' | 'n' => {
            print!("Please enter a playback position in minutes: ");
            let mut input: String;
            loop {
              input = String::new();
              stdout.flush().expect("Failed to flush stdout");
              stdin().read_line(&mut input).unwrap();
              if input.trim().parse::<f64>().is_err() {
                print!("\nInvalid input, please try again.\n: ");
              } else if input.contains('.') {
                if input
                  .split('.')
                  .collect::<Vec<&str>>()
                  .get(1)
                  .unwrap()
                  .len()
                  > 8
                {
                  print!("\nInvalid input, please lower the amount of decimal places.\n: ");
                } else {
                  break;
                }
              } else {
                break;
              }
            }
            item.UserData.PlaybackPositionTicks =
              (input.trim().parse::<f64>().unwrap() * 60.0 * 10000000.0)
                .to_string()
                .parse::<u64>()
                .unwrap();
          },
          _ => (),
        }
      }

      enable_raw_mode().unwrap();
      execute!(stdout, RestorePosition, Clear(ClearType::FromCursorDown)).unwrap();
      disable_raw_mode().unwrap();

      let mut mbps: String = String::new();
      if let Some((_, _, _, speed)) = previous_settings {
        mbps = (*speed.clone()).to_string();
      } else {
        print!("\nPlease enter your connection speed in mbps: ");
        loop {
          stdout.flush().expect("Failed to flush stdout");
          stdin().read_line(&mut mbps).unwrap();
          if !mbps.trim().is_numeric() {
            print!("\nInvalid input! Enter something like \"25\" equal to ~3MB/s.\n: ")
          } else {
            break;
          }
        }
      }

      enable_raw_mode().unwrap();
      execute!(
        stdout,
        RestorePosition,
        MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
        EnterAlternateScreen
      )
      .unwrap();
      disable_raw_mode().unwrap();

      let mut audio_track_index: u32 = 0;
      let mut subtitle_track_index: u32 = 0;
      println!();
      let media_source = &mediasource_list[mediasource_index];
      if !media_source.SupportsTranscoding {
        print_message(
          PrintMessageType::Error,
          format!(
            "MediaSource \"{}\" does not support transcoding. Trying next ...",
            media_source.Id
          )
          .as_str(),
        );
        exit(1);
      }
      let mut audio_tracks: Vec<MediaStream> = vec![];
      let mut subtitle_tracks: Vec<MediaStream> = vec![];
      for media_stream in media_source.MediaStreams.clone() {
        match media_stream.Type.as_str() {
          "Audio" => audio_tracks.push(media_stream),
          "Subtitle" => subtitle_tracks.push(media_stream),
          _ => (),
        }
      }
      if audio_tracks.len() > 1 {
        let mut skip = false;
        if let Some((_, Some(selection), _, _)) = previous_settings {
          for track in audio_tracks.clone() {
            if track.Index == *selection {
              skip = true;
              audio_track_index = *selection;
              break;
            }
          }
        }
        if !skip {
          let mut options: Vec<InteractiveOption> = vec![InteractiveOption {
            text: "Please choose which audio track to use:".to_string(),
            option_type: InteractiveOptionType::Header,
          }];
          for track in audio_tracks.clone() {
            options.push(InteractiveOption {
              text: track.to_string(),
              option_type: InteractiveOptionType::Button,
            });
          }
          if let ((ind, _), _, InteractiveOptionType::Button) = interactive_select(options) {
            audio_track_index = audio_tracks[ind].Index;
          }
        }
      }
      if subtitle_tracks.len() > 1 {
        let mut skip = false;
        if let Some((_, _, Some(selection), _)) = previous_settings {
          for track in subtitle_tracks.clone() {
            if track.Index == *selection {
              skip = true;
              subtitle_track_index = *selection;
              break;
            }
          }
        }
        if !skip {
          let mut options: Vec<InteractiveOption> = vec![InteractiveOption {
            text: "Please choose which subtitle track to use:".to_string(),
            option_type: InteractiveOptionType::Header,
          }];
          for track in subtitle_tracks.clone() {
            options.push(InteractiveOption {
              text: track.to_string(),
              option_type: InteractiveOptionType::Button,
            });
          }
          if let ((ind, _), _, InteractiveOptionType::Button) = interactive_select(options) {
            subtitle_track_index = subtitle_tracks[ind].Index;
          }
        }
      }

      *previous_settings = Some((
        false,
        Some(audio_track_index),
        Some(subtitle_track_index),
        mbps.clone(),
      ));

      enable_raw_mode().unwrap();
      execute!(
        stdout,
        RestorePosition,
        MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
        LeaveAlternateScreen
      )
      .unwrap();
      disable_raw_mode().unwrap();

      let bitrate = mbps.trim().parse::<u64>().unwrap() * 1000000;

      let session_capabilities: SessionCapabilities = SessionCapabilities {
        UserId: user_id.clone(),
        StartTimeTicks: item.UserData.PlaybackPositionTicks,
        MediaSourceId: mediasource_list[mediasource_index].Id.clone(),
        AudioStreamIndex: audio_track_index,
        SubtitleStreamIndex: subtitle_track_index,
        MaxStaticBitrate: bitrate,
        MaxStreamingBitrate: bitrate,
        EnableDirectPlay: true,
        EnableDirectStream: true,
        EnableTranscoding: true,
        AllowVideoStreamCopy: true,
        AllowAudioStreamCopy: true,
        DeviceProfile: DeviceProfile {
          Name: APPNAME.to_string(),
          Id: handle.get_device_id().clone(),
          MaxStaticMusicBitrate: 999999999,
          MaxStreamingBitrate: bitrate,
          SupportedMediaTypes: "Video".to_string(),
          TranscodingProfiles: [
            TranscodingProfile {
              Type: "Video".to_string(),
              Container: "mkv".to_string(),
              VideoCodec: "hevc".to_string(),
              TranscodeSeekInfo: "Auto".to_string(),
              Context: "Streaming".to_string(),
            },
            TranscodingProfile {
              Type: "Video".to_string(),
              Container: "mkv".to_string(),
              VideoCodec: "avc".to_string(),
              TranscodeSeekInfo: "Auto".to_string(),
              Context: "Streaming".to_string(),
            },
            TranscodingProfile {
              Type: "Video".to_string(),
              Container: "mkv".to_string(),
              VideoCodec: "av1".to_string(),
              TranscodeSeekInfo: "Auto".to_string(),
              Context: "Streaming".to_string(),
            },
          ]
          .to_vec(),
          SubtitleProfiles: [
            SubtitleProfile {
              Format: "subrip".to_string(),
              Method: "Embed".to_string(),
            },
            SubtitleProfile {
              Format: "srt".to_string(),
              Method: "Embed".to_string(),
            },
            SubtitleProfile {
              Format: "ass".to_string(),
              Method: "Embed".to_string(),
            },
            SubtitleProfile {
              Format: "ssa".to_string(),
              Method: "Embed".to_string(),
            },
            SubtitleProfile {
              Format: "pgssub".to_string(),
              Method: "Embed".to_string(),
            },
            SubtitleProfile {
              Format: "sub".to_string(),
              Method: "Embed".to_string(),
            },
            SubtitleProfile {
              Format: "dvdsub".to_string(),
              Method: "Embed".to_string(),
            },
            SubtitleProfile {
              Format: "pgs".to_string(),
              Method: "Embed".to_string(),
            },
          ]
          .to_vec(),
        },
      };

      let url = format!("Items/{}/PlaybackInfo?UserId={}", item.Id, user_id);
      match self.post(url, serde_json::to_string(&session_capabilities).unwrap()) {
        Ok(res) => {
          let search_text: &String = &res.text().unwrap();
          Ok(serde_json::from_str::<PlaybackInfo>(search_text).unwrap())
        },
        Err(err) => {
          print_message(
            PrintMessageType::Error,
            format!("Failed to post playback information: {}", err).as_str(),
          );
          Err(())
        },
      }
    } else {
      let url = format!("Items/{}/PlaybackInfo?UserId={}", item.Id, user_id);
      match self.get(url) {
        Ok(res) => {
          let search_text: &String = &res.text().unwrap();
          Ok(serde_json::from_str::<PlaybackInfo>(search_text).unwrap())
        },
        Err(err) => {
          print_message(
            PrintMessageType::Error,
            format!(
              "Failed to post playback information: {}",
              err.text().unwrap()
            )
            .as_str(),
          );
          Err(())
        },
      }
    }
  }

  fn series_set_playstate(&mut self, series: Series, indexes: Vec<usize>, played: bool) {
    let mut index = 0;
    for season in series.seasons {
      for episode in season.episodes {
        if indexes.contains(&index) {
          self.item_set_playstate(episode.Id, played);
        }
        index += 1;
      }
    }
  }

  fn item_set_playstate(&mut self, item_id: String, played: bool) {
    let status_str = if played { "Played" } else { "Un-Played" };
    let current_time = chrono::Local::now();
    let format_string = "%Y%m%d%H%M%S";
    let formatted_time = current_time.format(format_string);
    let user = self.get_config_handle().get_active_user().unwrap();
    let url = format!(
      "Users/{}/PlayedItems/{}?DatePlayed={}",
      user.user_id, item_id, formatted_time
    );
    let req = if played {
      self.post(url, String::new())
    } else {
      self.delete(url, String::new())
    };
    if let Err(err) = req {
      print_message(
        PrintMessageType::Error,
        format!("Failed to mark item as {}: {}", status_str, err).as_str(),
      );
    }
  }

  fn choose_from_series(&mut self, mut series: Series) -> Vec<Item> {
    loop {
      let mut selection: usize;
      match jelly_series_select(self.generate_series_structure(&series), series.clone()) {
        (SeriesOptions::Back, _) => {
          return vec![];
        },
        (SeriesOptions::Play, Some(index)) => {
          selection = index[0];
        },
        (SeriesOptions::Played, Some(indexes)) => {
          self.series_set_playstate(series.clone(), indexes, true);
          series = self.resolve_series(series.seasons[0].episodes[0].clone());
          continue;
        },
        (SeriesOptions::UnPlayed, Some(indexes)) => {
          self.series_set_playstate(series.clone(), indexes, false);
          series = self.resolve_series(series.seasons[0].episodes[0].clone());
          continue;
        },
        _ => panic!("What?!"),
      }
      let mut items: Vec<Item> = vec![];
      let mut record = false;
      for season in series.seasons.clone() {
        if record {
          items.extend(season.episodes);
        } else if season.episodes.len() > selection {
          for episode in season.episodes {
            if record {
              items.push(episode.clone());
            } else if selection == 0 {
              record = true;
              items.push(episode.clone());
            } else {
              selection -= 1;
            }
          }
        } else {
          selection -= season.episodes.len();
        }
      }
      return items;
    }
  }

  fn generate_series_structure(&mut self, series: &Series) -> Vec<String> {
    let mut trans_item = series.seasons[0].item.clone();
    trans_item.Type = String::from("Series");
    trans_item.Name = trans_item.clone().SeriesName.unwrap_or("???".to_string());

    let full_size = {
      let mut size = 0;
      series.seasons.iter().for_each(|s| size += s.episodes.len());
      size
    };
    let zero_pad_amount = (full_size as f64).log10().floor() as usize + 1;
    let mut just_text: Vec<String> = vec![format!(" {}", trans_item.to_string().bold())];
    let mut index = 0;
    for (season_index, season) in series.seasons.iter().enumerate() {
      let mut line: String = String::new();
      if season_index == series.seasons.len() - 1 {
        line.push_str("  └─ ");
      } else {
        line.push_str("  ├─ ");
      }
      line.push_str(format!("{}", season.item.Name.clone().bold()).as_str());
      just_text.push(line.clone());
      line.clear();
      for (episode_index, episode) in season.clone().episodes.iter().enumerate() {
        let prefix: &str = if season_index == series.seasons.len() - 1 {
          "       "
        } else {
          "  │    "
        };
        if episode_index == season.episodes.len() - 1 {
          line.push_str(format!("{}└── ", prefix).as_str());
        } else {
          line.push_str(format!("{}├── ", prefix).as_str());
        }
        line.push_str(format!("[{:0zero_pad_amount$}] ", index).as_str());
        let terminal_size = terminal::size().unwrap().0 as usize;
        if 13 + zero_pad_amount + index.to_string().len() + episode.to_string_ext().len()
          > terminal_size
        {
          line.push_str(format!("{}...", episode.to_string_ext()).as_str());
          just_text.push(line.clone());
          line.clear();
        } else {
          line.push_str(episode.to_string_ext().as_str());
          just_text.push(line.clone());
          line.clear();
        }
        index += 1;
      }
    }
    just_text
  }

  fn resolve_series(&mut self, item: Item) -> Series {
    if item.Type != "Season" && item.Type != "Episode" && item.Type != "Series" {
      panic!("This object cannot be part of a series.");
    }
    let user = self.get_config_handle().get_active_user().unwrap();

    let mut series = Series {
      item_id: item.SeriesId.clone().unwrap_or(item.Id),
      seasons: vec![],
    };

    if let Ok(items) = self.get_items(
      format!("Users/{}/Items?ParentId={}", user.user_id, series.item_id),
      false,
    ) {
      for season in items {
        series.seasons.append(&mut vec![Season {
          item: season,
          episodes: vec![],
        }]);
      }
    } else {
      exit(1);
    };

    if series.seasons[0].item.Name == *"Specials" {
      let specials = series.seasons.remove(0);
      series.seasons.push(specials);
    }

    let mut episode_ids: Vec<String> = vec![];
    for (season_index, season) in series.seasons.clone().iter().enumerate() {
      if let Ok(items) = self.get_items(
        format!("Users/{}/Items?ParentId={}", user.user_id, season.item.Id),
        false,
      ) {
        for episode in items {
          if !episode_ids.contains(&episode.Id) {
            episode_ids.push(episode.Id.clone());
            series.seasons[season_index]
              .episodes
              .append(&mut vec![episode]);
          }
        }

        if series.seasons[season_index].episodes.is_empty() {
          series.seasons.remove(season_index);
        }
      } else {
        exit(1);
      };
    }

    series
  }

  fn get_item(&mut self, item_id: String) -> Result<Item, ()> {
    let url = format!(
      "Users/{}/Items/{}?Fields=PremiereDate,MediaSources,Status,ProductionYear&collapseBoxSetItems=False&IsMissing=False",
      self.get_config_handle().get_active_user().unwrap().user_id,
      item_id
    );
    match self.get(url.clone()) {
      Ok(result) => {
        if let Ok(json) = serde_json::from_str::<Value>(&result.text().unwrap()) {
          if let Ok(item_list) = serde_json::from_value::<Item>(json) {
            return Ok(item_list);
          } else {
            print_message(
              PrintMessageType::Error,
              "Failed to serialize the json response into an item.",
            );
          }
        } else {
          print_message(
            PrintMessageType::Error,
            "Failed to convert response into json.",
          );
        }
      },
      Err(e) => {
        print_message(
          PrintMessageType::Error,
          format!("Failed to get item at \"{}\"\n{}\n", url, e.text().unwrap()).as_str(),
        );
      },
    }
    Err(())
  }

  fn get_items(&mut self, mut url: String, raw: bool) -> Result<Vec<Item>, ()> {
    if !url.contains('?') {
      url.push('?')
    } else if !url.ends_with('&') {
      url.push('&')
    }
    let modded_url = format!(
      "{}Fields=PremiereDate,MediaSources,Status,ProductionYear&collapseBoxSetItems=False&IsMissing=False",
      url
    );
    match self.get(modded_url.clone()) {
      Ok(result) => {
        if let Ok(mut json) = serde_json::from_str::<Value>(&result.text().unwrap()) {
          if !raw {
            json = json.get("Items").unwrap().clone();
          }
          if let Ok(item_list) = serde_json::from_value::<Vec<Item>>(json) {
            return Ok(item_list);
          } else {
            print_message(
              PrintMessageType::Error,
              "Failed to serialize the json response into an item list.",
            );
          }
        } else {
          print_message(
            PrintMessageType::Error,
            "Failed to convert response into json.",
          );
        }
      },
      Err(e) => {
        print_message(
          PrintMessageType::Error,
          format!(
            "Failed to get item list at \"{}\"\n{}\n",
            url,
            e.text().unwrap()
          )
          .as_str(),
        );
      },
    }
    Err(())
  }

  async fn stop_playback(
    &mut self,
    item_id: String,
    playbackpositionticks: u64,
    total_runtime: u64,
    time_pos: f64,
  ) -> bool {
    let playback_info = self.get_playback_info();
    let mut time_position = (time_pos * 10000000.0).round() as u64;
    let mut time_as_secs = time_pos;
    let session_id = self.get_session_id().expect("This shouldn't be a None!");
    let user = self.get_config_handle().get_active_user().unwrap();

    if self.get_config_handle().config.transcoding {
      time_position += playbackpositionticks * 10000000;
      time_as_secs += playbackpositionticks as f64
    };

    let finished_obj: PlaybackStopInfo;
    let success_message: String;
    let difference = (((total_runtime * 10000000) as f64) - time_position as f64)
      / ((total_runtime * 10000000) as f64);
    if difference < 0.15 {
      // watched more than 75%
      let url = format!("Users/{}/PlayedItems/{}", user.user_id, item_id);
      match self.async_post(url, String::new()).await {
        Ok(_) => {
          print_message(PrintMessageType::Success, "Marked item as [Played].");
        },
        Err(err) => {
          print_message(
            PrintMessageType::Error,
            format!("Failed to report PlaySession as stopped: {}", err).as_str(),
          );
        },
      }
      return true;
    } else if difference < 0.85 {
      // watched more than 15%
      finished_obj = PlaybackStopInfo {
        ItemId: item_id,
        PlaySessionId: playback_info.PlaySessionId.to_string(),
        SessionId: session_id,
        MediaSourceId: playback_info.MediaSources[0].Id.to_string(),
        PositionTicks: time_position.to_string(),
        Failed: false,
      };
      let formatted: String = if time_as_secs > 60.0 {
        if (time_as_secs / 60.0) > 60.0 {
          format!(
            "{:02}:{:02}:{:02}",
            ((time_as_secs / 60.0) / 60.0).trunc(),
            ((((time_as_secs / 60.0) / 60.0) - ((time_as_secs / 60.0) / 60.).trunc()) * 60.0)
              .trunc(),
            (((time_as_secs / 60.0) - (time_as_secs / 60.0).trunc()) * 60.0).trunc()
          )
        } else {
          format!(
            "00:{:02}:{:02}",
            (time_as_secs / 60.0).trunc(),
            (((time_as_secs / 60.0) - (time_as_secs / 60.0).trunc()) * 60.0).trunc()
          )
        }
      } else {
        time_as_secs.to_string()
      };
      success_message = format!(
        "Playback progress ({}) has been sent to your server.",
        formatted
      )
    } else {
      finished_obj = PlaybackStopInfo {
        ItemId: item_id,
        PlaySessionId: playback_info.PlaySessionId.to_string(),
        SessionId: session_id,
        MediaSourceId: playback_info.MediaSources[0].Id.to_string(),
        PositionTicks: (playbackpositionticks as f64).to_string(),
        Failed: false,
      };
      success_message = "Playback progress of this item has not been changed.".to_string();
    }
    let url = "Sessions/Playing/Stopped".to_string();
    match self
      .async_post(url, serde_json::to_string(&finished_obj).unwrap())
      .await
    {
      Ok(_) => {
        print_message(PrintMessageType::Success, &success_message);
        false
      },
      Err(err) => {
        print_message(
          PrintMessageType::Error,
          format!("Failed to log playback progress to your server: {}", err).as_str(),
        );
        false
      },
    }
  }

  fn get_playback_info(&mut self) -> PlaybackInfo;

  async fn report_playback(
    &mut self,
    item_id: String,
    playbackpositionticks: u64,
    mut time_pos: f64,
    audio_track: u32,
    sub_track: u32,
    paused: bool,
    muted: bool,
    volume_level: u32,
    _socket: &mut UnboundedSender<String>,
  ) {
    let playback_info = self.get_playback_info();
    let session_id = self.get_session_id().expect("This shouldn't be None!");
    let event_name: EventName = if paused {
      EventName::Pause
    } else if muted {
      EventName::VolumeChange
    } else {
      EventName::TimeUpdate
    };
    let playmethod: PlayMethod;
    (playmethod, time_pos) = if self.get_config_handle().config.transcoding {
      (
        PlayMethod::Transcode,
        time_pos * 10000000.0 + (playbackpositionticks * 10000000) as f64,
      )
    } else {
      (PlayMethod::DirectPlay, time_pos * 10000000.0)
    };
    let update_object = PlaybackProgressInfo {
      CanSeek: true,
      ItemId: item_id,
      SessionId: session_id,
      MediaSourceId: playback_info.MediaSources[0].Id.to_string(),
      AudioStreamIndex: audio_track,
      SubtitleStreamIndex: sub_track,
      IsPaused: paused,
      IsMuted: muted,
      PositionTicks: time_pos.round() as u64,
      VolumeLevel: volume_level,
      PlaySessionId: playback_info.PlaySessionId.to_string(),
      PlayMethod: playmethod,
      EventName: event_name,
    };

    let url = "Sessions/Playing/Progress".to_string();
    if let Err(err) = self
      .async_post(url, serde_json::to_string(&update_object).unwrap())
      .await
    {
      print_message(
        PrintMessageType::Error,
        format!("Failed to report PlaySession as started: {}", err).as_str(),
      );
    }
  }

  async fn start_playback(&mut self, item_id: String, playbackpositionticks: u64) {
    let playback_info = self.get_playback_info();
    let session_id = self.get_session_id().expect("This shouldn't be a None!");
    let playmethod = if self.get_config_handle().config.transcoding {
      PlayMethod::Transcode
    } else {
      PlayMethod::DirectPlay
    };
    let audio_index = if let Some(transcoding_url) = &playback_info.MediaSources[0].TranscodingUrl {
      if transcoding_url.contains("AudioStreamIndex") {
        let reg = Regex::new(r#"(?:AudioStreamIndex=)(\d+)"#).unwrap();
        let num = reg
          .captures(transcoding_url)
          .unwrap()
          .get(1)
          .unwrap()
          .as_str();
        num.parse::<u32>().unwrap()
      } else {
        0
      }
    } else {
      0
    };
    let subtitle_index =
      if let Some(transcoding_url) = &playback_info.MediaSources[0].TranscodingUrl {
        if transcoding_url.contains("SubtitleStreamIndex") {
          let reg = Regex::new(r#"(?:SubtitleStreamIndex=)(\d+)"#).unwrap();
          let num = reg
            .captures(transcoding_url)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
          num.parse::<u32>().unwrap()
        } else {
          0
        }
      } else {
        0
      };

    let playing_object = PlayRequest {
      CanSeek: true,
      ItemId: item_id,
      SessionId: session_id.to_string(),
      MediaSourceId: playback_info.MediaSources[0].Id.to_string(),
      AudioStreamIndex: audio_index,
      SubtitleStreamIndex: subtitle_index,
      IsPaused: false,
      IsMuted: false,
      PlaybackStartTimeTicks: playbackpositionticks * 10000000,
      PlaySessionId: playback_info.PlaySessionId.to_string(),
      PlayMethod: playmethod,
      RepeatMode: RepeatMode::RepeatNone,
    };

    let url = "Sessions/Playing".to_string();
    if let Err(err) = self
      .async_post(url, serde_json::to_string(&playing_object).unwrap())
      .await
    {
      print_message(
        PrintMessageType::Error,
        format!("Failed to start playsession!: {}", err).as_str(),
      );
    }
  }

  fn get_address(&mut self) -> String {
    if let Some(address) = self.get_config_handle().get_address() {
      address
    } else {
      match self.get_config_handle().config.media_center_type {
        MediaCenterType::Plex => panic!("Plex: this function shouldn't be called before login()."),
        _ => {
          let handle = self.get_config_handle();
          let config = &mut handle.config;
          if let Some(server_info) = broadcast_search(config.media_center_type) {
            config.server_name = server_info.Name;
            handle.insert_specific_value(Objective::Address, server_info.Address);
          } else {
            handle.ask_for_setting(Objective::ServerName);
            handle.ask_for_setting(Objective::Address);
          }
          handle.get_address().unwrap()
        },
      }
    }
  }

  fn create_user_credentials(&mut self) -> UserCredentials {
    let config = &self.get_config_handle().config;
    print!(
      "Please enter your {} username",
      config.media_center_type.to_string()
    );
    let username = take_string_input(vec![]);
    print!(
      "Please enter your {} password: ",
      config.media_center_type.to_string()
    );
    let password = hidden_string_input(Some('*'));
    println!();
    UserCredentials { username, password }
  }

  fn re_authenticate(&mut self) {
    if let Some(user) = self.get_config_handle().get_active_user() {
      print!(
        "Logging in with {} on {} ",
        user.username.cyan(),
        self.get_config_handle().config.server_name.clone().cyan()
      );
      loop {
        let config = self.get_config_handle();
        let url = format!("Sessions?DeviceId={}", config.get_device_id());
        self.write_headers();
        match self.get(url) {
          Ok(response) => {
            let json_response = serde_json::from_str::<Value>(&response.text().unwrap()).unwrap();
            if let Some(id) = json_response[0].get("Id") {
              println!("{}\n", "🗸".green());
              self.insert_value(
                MediaCenterValues::SessionID,
                id.as_str().unwrap().to_string(),
              );
              if let Some(support) = json_response[0].get("SupportedCommands") {
                if !support.to_string().contains("PlayState") {
                  // yea that should be sufficient
                  self.report_session_capabilities().unwrap();
                }
              }
              return;
            } else if self.report_session_capabilities().is_ok() {
              continue;
            } else {
              println!("{}", "𐄂".red());
              print_message(
                PrintMessageType::Error,
                "Creating a new session failed. Please login again.",
              );
              self.get_config_handle().remove_user(user.access_token);
            }
          },
          Err(e) => {
            println!("{}", "𐄂".red());
            if e.status() == StatusCode::UNAUTHORIZED {
              print_message(
                PrintMessageType::Error,
                format!("{}: This session expired. Please login again.", e.status()).as_str(),
              );
              self.get_config_handle().remove_user(user.access_token);
            } else {
              print_message(
                PrintMessageType::Error,
                format!("{}: {}", e.status(), e.text().unwrap()).as_str(),
              );
            }
          },
        }
        break;
      }
    }
    self.login();
  }

  fn write_headers(&mut self) {
    let handle = self.get_config_handle();
    let config = &mut handle.config;
    match config.media_center_type {
      MediaCenterType::Plex => panic!("not sure if this is even needed"),
      _ => {
        if let Some(user) = handle.get_active_user() {
          let authorization_header: (String, String) = (
            String::from("Authorization"),
            format!(
              "Emby UserId={}, Client=Emby Theater, Device={}, DeviceId={}, Version={}, Token={}",
              user.user_id,
              APPNAME,
              handle.get_device_id(),
              VERSION,
              user.access_token
            ),
          );
          self.insert_value(
            MediaCenterValues::Header,
            serde_json::to_string(&authorization_header).unwrap(),
          );
          let request_header: (String, String) =
            (format!("{}/{}", APPNAME, VERSION), user.access_token);
          self.insert_value(
            MediaCenterValues::Header,
            serde_json::to_string(&request_header).unwrap(),
          );
        } else {
          panic!("Trying to generate a new header without any user existent?!");
        }
      },
    }
  }

  fn login(&mut self) {
    loop {
      let url = "Users/AuthenticateByName".to_string();
      let server_name = self.get_config_handle().config.server_name.clone();
      let creds = self.create_user_credentials();
      let body = format!(
        "{{\"Username\":\"{}\",\"pw\":\"{}\"}}",
        creds.username, creds.password
      );
      print!(
        "Logging in with {} on {} ",
        creds.username.clone().cyan(),
        server_name.clone().cyan()
      );
      match self.post(url.clone(), body.clone()) {
        Ok(res) => {
          println!("{}", "🗸".green());
          let json_response = serde_json::from_str::<Value>(&res.text().unwrap()).unwrap();
          let session_obj = json_response.get("SessionInfo").unwrap();
          let user = UserConfig {
            access_token: json_response["AccessToken"].as_str().unwrap().to_string(),
            username: session_obj["UserName"].as_str().unwrap().to_string(),
            user_id: session_obj["UserId"].as_str().unwrap().to_string(),
          };
          let device_id = session_obj["DeviceId"].as_str().unwrap().to_string();
          let config = self.get_config_handle();
          config.insert_specific_value(Objective::DeviceID, device_id);
          config.insert_specific_value(Objective::User, serde_json::to_string(&user).unwrap());
          config.set_active_user(user.access_token);
          config.save();
          self.write_headers();
          let session_id = session_obj["Id"].as_str().unwrap().to_string();
          self.insert_value(MediaCenterValues::SessionID, session_id);
          break;
        },
        Err(e) => {
          println!("{}", "𐄂".red());
          match e.as_str() {
            "Failed to send request" => {
              print_message(PrintMessageType::Error, "Failed to send login request.");
              let config = self.get_config_handle();
              config.ask_for_setting(Objective::ServerName);
              config.ask_for_setting(Objective::Address);
            },
            _ => {
              print_message(PrintMessageType::Error, "Login failed! Please try again.");
            },
          }
        },
      }
    }
    self.report_session_capabilities().unwrap();
  }

  fn get_session_id(&mut self) -> Option<String>;

  fn report_session_capabilities(&mut self) -> Result<(), ()> {
    let config = &self.get_config_handle().config;
    if config.media_center_type == MediaCenterType::Plex {
      panic!("What in the?!");
    }

    let capabilities = Capabilities {
      PlayableMediaTypes: "Video".to_string(),
      SupportsMediaControl: true,
      SupportedCommands: vec![
        String::from("MoveUp"),
        String::from("MoveDown"),
        String::from("MoveLeft"),
        String::from("MoveRight"),
        String::from("Select"),
        String::from("Back"),
        String::from("ToggleFullscreen"),
        String::from("GoHome"),
        String::from("GoToSettings"),
        String::from("TakeScreenshot"),
        String::from("VolumeUp"),
        String::from("VolumeDown"),
        String::from("ToggleMute"),
        String::from("SetAudioStreamIndex"),
        String::from("SetSubtitleStreamIndex"),
        String::from("Mute"),
        String::from("Unmute"),
        String::from("SetVolume"),
        String::from("DisplayContent"),
        String::from("Play"),
        String::from("Playstate"),
        String::from("PlayNext"),
        String::from("PlayMediaSource"),
      ],
    };

    let url: String;
    if let Some(session_id) = self.get_session_id() {
      url = format!("Sessions/Capabilities/Full?Id={}", session_id);
    } else {
      url = "Sessions/Capabilities/Full".to_string();
    }

    match self.post(url, serde_json::to_string(&capabilities).unwrap()) {
      Ok(_) => Ok(()),
      Err(err) => {
        print_message(
          PrintMessageType::Error,
          format!("Failed to post remote control support: {}.", err).as_str(),
        );
        Err(())
      },
    }
  }

  fn get(&mut self, url: String) -> Result<Response, Response> {
    let url = format!("{}{}", self.get_address(), url);
    let headers = self.get_headers();
    let authorization_2 = if let Some(header) = headers.get(1) {
      header
    } else {
      panic!("Authorization header missing!! Make sure you are running write_headers().");
    };
    let request_headers = if let Some(header) = headers.get(2) {
      header
    } else {
      panic!("Request header missing!! Make sure you are running write_headers().");
    };
    let client = Client::new();
    let request = client
      .get(url)
      .timeout(Duration::from_secs(15))
      .header(authorization_2.clone().0, authorization_2.clone().1)
      .header(String::from("X-Application"), request_headers.clone().0)
      .header(String::from("X-Emby-Token"), request_headers.clone().1)
      .header("Content-Type", "application/json")
      .send();

    let response = if let Err(res) = request {
      print_message(PrintMessageType::Error, res.to_string().as_str());
      exit(1);
    } else {
      request.unwrap()
    };

    match response.status() {
      StatusCode::OK => Ok(response),
      _ => Err(response),
    }
  }

  fn delete(&mut self, url: String, body: String) -> Result<Response, String> {
    let url = format!("{}{}", self.get_address(), url);
    let headers = self.get_headers();
    let client = Client::new();
    let mut builder = client.delete(url).timeout(Duration::from_secs(15));
    if headers.len() == 1 {
      let authorization_1 = headers.get(0).unwrap();
      builder = builder.header(authorization_1.clone().0, authorization_1.clone().1);
    } else {
      let authorization_2 = headers.get(1).unwrap();
      let request_headers = headers.get(2).unwrap();
      builder = builder.header(authorization_2.clone().0, authorization_2.clone().1);
      builder = builder.header(String::from("X-Application"), request_headers.clone().0);
      builder = builder.header(String::from("X-Emby-Token"), request_headers.clone().1);
    }
    let request = builder
      .header("Content-Type", "application/json")
      .body(body)
      .send();

    let response = if let Err(res) = request {
      print_message(PrintMessageType::Error, res.to_string().as_str());
      exit(1);
    } else {
      request.unwrap()
    };

    match response.status() {
      StatusCode::OK => Ok(response),
      _ => Err(response.text().unwrap()),
    }
  }

  fn post(&mut self, url: String, body: String) -> Result<Response, String> {
    let url = format!("{}{}", self.get_address(), url);
    let headers = self.get_headers();
    let client = Client::new();
    let mut builder = client.post(url).timeout(Duration::from_secs(15));
    if headers.len() == 1 {
      let authorization_1 = headers.get(0).unwrap();
      builder = builder.header(authorization_1.clone().0, authorization_1.clone().1);
    } else {
      let authorization_2 = headers.get(1).unwrap();
      let request_headers = headers.get(2).unwrap();
      builder = builder.header(authorization_2.clone().0, authorization_2.clone().1);
      builder = builder.header(String::from("X-Application"), request_headers.clone().0);
      builder = builder.header(String::from("X-Emby-Token"), request_headers.clone().1);
    }
    let request = builder
      .header("Content-Type", "application/json")
      .body(body)
      .send();

    let response = if let Err(res) = request {
      return Err(res.to_string());
    } else {
      request.unwrap()
    };

    match response.status() {
      StatusCode::OK | StatusCode::NO_CONTENT => Ok(response),
      _ => Err(response.text().unwrap()),
    }
  }

  // Since reqwest::blocking::client isn't allowed in an asynchronous context >~<
  async fn async_post(&mut self, url: String, body: String) -> Result<reqwest::Response, String> {
    let url = format!("{}{}", self.get_address(), url);
    let headers = self.get_headers();
    let client = reqwest::Client::new();
    let mut builder = client.post(url).timeout(Duration::from_secs(15));
    if headers.len() == 1 {
      let authorization_1 = headers.get(0).unwrap();
      builder = builder.header(authorization_1.clone().0, authorization_1.clone().1);
    } else {
      let authorization_2 = headers.get(1).unwrap();
      let request_headers = headers.get(2).unwrap();
      builder = builder.header(authorization_2.clone().0, authorization_2.clone().1);
      builder = builder.header(String::from("X-Application"), request_headers.clone().0);
      builder = builder.header(String::from("X-Emby-Token"), request_headers.clone().1);
    }
    let request = builder
      .header("Content-Type", "application/json")
      .body(body)
      .send()
      .await;

    let response = if let Err(res) = request {
      return Err(res.to_string());
    } else {
      request.unwrap()
    };

    match response.status() {
      StatusCode::OK | StatusCode::NO_CONTENT => Ok(response),
      _ => Err(response.text().await.unwrap()),
    }
  }

  fn insert_value(&mut self, value_type: MediaCenterValues, value: String);
}

pub fn broadcast_search(media_center_type: MediaCenterType) -> Option<UDPAnswer> {
  let address: Arc<Mutex<Option<UDPAnswer>>> = Arc::new(Mutex::new(None));
  let who_is = if media_center_type == MediaCenterType::Jellyfin {
    "who is JellyfinServer?"
  } else {
    "who is EmbyServer?"
  };

  print!(
    "Searching for local media-centers (5s timeout).\nPress any key to interrupt and for manual input."
  );
  let handle = thread::spawn({
    let address_clone = Arc::clone(&address);
    move || {
      let res = broadcast(who_is);
      if let Ok(answer) = res {
        let mut address = address_clone.lock().unwrap();
        *address = Some(answer);
      }
    }
  });

  let mut stdout = stdout();
  enable_raw_mode().unwrap();
  execute!(stdout, EnableBlinking, Show).unwrap();

  loop {
    if handle.is_finished() {
      disable_raw_mode().unwrap();
      println!();
      break;
    }
    if poll(Duration::from_millis(100)).unwrap() {
      if let Ok(Event::Key(KeyEvent {
        code,
        modifiers,
        state: _,
        kind: _,
      })) = read()
      {
        if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
          disable_raw_mode().unwrap();
          println!("^C");
          exit(1);
        } else {
          disable_raw_mode().unwrap();
          println!();
          break;
        }
      }
    }
  }

  let obj = address.lock().unwrap().clone();
  if let Some(answer) = obj {
    println!();
    print!(
      "Is this IP-Adress/Domain the correct one: {}\n (Y)es / (N)o",
      answer.Address
    );
    match getch("YyNn") {
      'Y' | 'y' => {
        println!();
        Some(answer)
      },
      _ => {
        println!();
        None
      },
    }
  } else {
    println!("No local media-instances found.\n");
    None
  }
}

fn broadcast(message: &str) -> Result<UDPAnswer, ()> {
  let socket: UdpSocket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to network socket.");
  socket
    .set_read_timeout(Some(Duration::new(5, 0)))
    .expect("Failed to set timeout.");
  socket
    .set_broadcast(true)
    .expect("Failed to start broadcast.");
  socket
    .send_to(&String::from(message).into_bytes(), "255.255.255.255:7359")
    .expect("Failed to send broadcast.");
  let mut buf = [0; 4096];
  if socket.recv_from(&mut buf).is_ok() {
    let parsed = from_utf8(&buf)
      .expect("Failed to read buffer into &str")
      .trim_matches(char::from(0));
    let udp_answer: UDPAnswer = serde_json::from_str(parsed).unwrap();
    Ok(udp_answer)
  } else {
    Err(())
  }
}
