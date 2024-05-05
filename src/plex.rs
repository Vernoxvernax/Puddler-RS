use std::{fmt, io::{stdin, stdout, Write}, process::exit, sync::mpsc, thread::{self, sleep}, time::Duration};
use chrono::{DateTime, Utc};
use isahc::{config::Configurable, http::StatusCode, Body, ReadResponseExt, Request, RequestExt, Response};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crossterm::{cursor::{Hide, MoveToColumn, RestorePosition, SavePosition, Show}, execute, style::Stylize, terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen}};
use tokio::sync::mpsc::UnboundedSender;

use crate::{input::{getch, interactive_select, plex_series_select, take_string_input, InteractiveOption, InteractiveOptionType, SeriesOptions}, media_center::{IsNumeric, MediaCenter, MediaCenterValues, ToStringAdv}, media_config::{Config, Objective, UserConfig}, mpv::Player, printing::{print_message, PrintMessageType}, puddler_settings::PuddlerSettings, APPNAME, VERSION};

const PLEX_CLIENT_PROFILES: &str = "add-direct-play-profile(
type=videoProfile
&protocol=http
&container=mkv
&videoCodec=*
&audioCodec=*
&subtitleCodec=*
)+
add-transcode-target(
type=videoProfile
&context=streaming
&protocol=http
&container=mkv
&videoCodec=h264,hevc,png,apng,bmp,mjpeg,thp,gif,vp8,vp9,dirac,ffv1,ffvhuff,huffyuv,rawvideo,012v,ayuv,r210,v210,v210x,v308,v408,v410,y41p,yuv4,ansi,h263,mpeg1video,mpeg2video,mpeg4
&audioCodec=ape,aac,aac_latm,alac,dca,vorbis,opus,pcm,pcm_alaw,pcm_mulaw,pcm_bluray,pcm_dvd,mp1,mp2,eac3,ac3,flac,mp3
&subtitleCodec=*
&replace=true
)+add-limitation(
scope=videoCodec
&scopeName=*
&type=upperBound
&name=video.bitDepth
&value=10
&replace=true)";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct PlexLibrary {
  MediaContainer: MediaContainer
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct MediaContainer {
  size: u32,
  mixedParents: Option<bool>,
  Metadata: Option<Vec<PlexItem>>,
  Hub: Option<Vec<PlexHub>>
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct PlexHub {
  r#type: String,
  size: u32,
  Metadata: Option<Vec<PlexItem>>
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlexItem {
  pub ratingKey: String,
  pub parentRatingKey: Option<String>,
  pub parentTitle: Option<String>,
  pub grandparentTitle: Option<String>,
  pub parentIndex: Option<u32>,
  pub viewCount: Option<u32>,
  pub guid: String,
  pub r#type: String,
  pub title: String,
  pub year: Option<u32>,
  parentYear: Option<u32>,
  pub duration: Option<u64>,
  pub Media: Option<Vec<PlexMediaFile>>,
  leafCount: Option<u32>,
  viewedLeafCount: Option<u32>,
  index: Option<u32>,
  pub viewOffset: Option<u64>
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlexMediaFile {
  id: u64,
  duration: u64,
  bitrate: u64,
  width: u64,
  height: u64,
  audioChannels: u8,
  audioCodec: String,
  videoCodec: String,
  deletedAt: Option<u64>,
  pub Part: Vec<PlexMediaPart>
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlexMediaPart {
  id: u64,
  pub key: String,
  duration: u64,
  file: String,
  pub Stream: Option<Vec<PlexStream>>
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlexStream {
  pub key: Option<String>,
  id: u32,
  index: Option<u32>,
  streamType: u8,
  languageCode: Option<String>,
  codec: Option<String>,
  default: Option<bool>,
  audioChannelLayout: Option<String>,
  title: Option<String>,
  pub displayTitle: Option<String>,
  pub language: Option<String>
}

impl fmt::Display for PlexStream {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.default == Some(true) {
      write!(f, "Title = \"{}\", Language = \"{}\", Codec = \"{}\" {}",
        self.title.as_ref().unwrap_or(self.displayTitle.as_ref().unwrap_or(&"".to_string())),
        self.language.as_ref().unwrap_or(self.languageCode.as_ref().unwrap_or(&"undefined".to_string())),
        self.codec.as_ref().unwrap_or(&"???".to_string()).to_uppercase(),
        "[Default]".to_string().green()
      )
    } else {
      write!(f, "Title = \"{}\", Language = \"{}\", Codec = \"{}\"",
      self.title.as_ref().unwrap_or(self.displayTitle.as_ref().unwrap_or(&"".to_string())),
      self.language.as_ref().unwrap_or(self.languageCode.as_ref().unwrap_or(&"undefined".to_string())),
      self.codec.as_ref().unwrap_or(&"???".to_string()).to_uppercase())
    }
  }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct PlexTVUser {
  id: u64,
  username: String
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlexCreatePin {
  auth_token: Option<String>,
  client_identifier: String,
  code: String,
  expires_at: DateTime<Utc>,
  id: u32,
  trusted: bool,
  user_id: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlexResources {
  accessToken: Option<String>,
  name: String,
  provides: String,
  publicAddress: String
}

#[derive(Clone)]
pub struct PlexServer {
  config_handle: Config,
  headers: Vec<(String, String)>,
  session_id: Option<String>,
  settings: PuddlerSettings,
  playback_info: Option<PlexItem>
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Series {
  item_id: String,
  pub seasons: Vec<Season>
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Season {
  pub item: PlexItem,
  pub episodes: Vec<PlexItem>
}

impl ToStringAdv for PlexItem {
  fn to_string_split(&self) -> Vec<String> {
    let time = if let Some(production_year) = &self.year {
      format!("({})", production_year)
    } else if let Some(parentYear) = self.parentYear {
      format!("({})", parentYear)
    } else {
      "(???)".to_string()
    };

    let mut name: String;
    match self.r#type.as_str() {
      "season" | "episode" => name = self.parentTitle.clone().unwrap_or(String::from("???")),
      _ => name = self.title.clone()
    }
    if name.contains('(') {
      let re = Regex::new(r" \(\d{4}\)").unwrap();
      name = re.replace_all(&name, "").to_string();
    }

    match self.r#type.as_str() {
      "movie" | "show" => {
        vec![format!("{} {}", name, time)]
      },
      "season" => {
        vec![
          self.to_string(),
          format!("{} {}", name, time),
          self.title.clone()
        ]
      },
      "episode" => {
        vec![
          self.to_string(),
          format!("{} {}", name, time),
          format!("S{:02}E{:02} ({})",
            self.parentIndex.unwrap_or(0),
            self.index.unwrap_or(0),
            self.title
          )
        ]
      },
      _ => vec![format!("{} {} (unknown media type)", self.title, time)]
    }
  }

  fn to_string_full(&self) -> String {
    let basic = self.to_string();
    if let Some(offset) = self.viewOffset {
      if let Some(duration) = self.duration {
        return format!("{} {}%", basic, ((offset as f64 / duration as f64) * 100.0).round());
      }
    }
    basic
  }
}

impl ToString for PlexItem {
  fn to_string(&self) -> String {
    let time = if let Some(production_year) = &self.year {
      format!("({})", production_year)
    } else if let Some(parentYear) = self.parentYear {
      format!("({})", parentYear)
    } else {
      "(???)".to_string()
    };
    let mut played_status = String::new();
    if &self.r#type == "season" || &self.r#type == "show" {
      if let Some(leafCount) = self.leafCount {
        if let Some(viewedLeafCount) = self.viewedLeafCount {
          if !leafCount > viewedLeafCount {
            played_status = format!(" - {}", "(Played)".green());
          }
        }
      }
    } else if let Some(viewCount) = self.viewCount {
      if viewCount > 0  {
        played_status = format!(" - {}", "(Played)".green());
      }
    }

    let mut name: String;
    match self.r#type.as_str() {
      "season" => name = self.parentTitle.clone().unwrap_or(String::from("???")),
      "episode" => name = self.grandparentTitle.clone().unwrap_or(String::from("???")),
      _ => name = self.title.clone()
    }
    if name.contains('(') {
      let re = Regex::new(r" \(\d{4}\)").unwrap();
      name = re.replace_all(&name, "").to_string();
    }
    
    match self.r#type.as_str() {
      "movie" | "show" => {
        format!("{} {}{}", name, time, played_status)
      },
      "season" => {
        format!("{} {} - {}",
          name,
          time,
          self.title.clone()
        )
      },
      "episode" => {
        format!("{} {} - S{:02}E{:02} - {}{}",
          name,
          time,
          self.parentIndex.unwrap_or(0),
          self.index.unwrap_or(0),
          self.title,
          played_status
        )
      },
      _ => format!("{} {} (unknown media type){}", self.title, time, played_status)
    }
  }
}

impl MediaCenter for PlexServer {
  fn new(mut config: Config, settings: PuddlerSettings) -> Self {
    PlexServer {
      config_handle: config.clone(),
      headers: vec![
        (
          String::from("Authorization"),
          format!("Emby UserId=\"\", Client=Emby Theater, Device={}, DeviceId={}, Version={}, Token=\"\"",
          APPNAME, config.get_device_id(), VERSION)
        )
      ],
      session_id: None,
      settings,
      playback_info: None
    }
  }

  fn get_settings(&mut self) -> &mut PuddlerSettings {
    &mut self.settings
  }
  
  fn get_config_handle(&mut self) -> &mut Config {
    &mut self.config_handle
  }
    
  fn get_headers(&mut self) -> Vec<(String, String)> {
    self.headers.clone()
  }

  fn insert_value(&mut self, value_type: MediaCenterValues, value: String) {
    match value_type {
      MediaCenterValues::SessionID => {
        self.session_id = Some(value);
      },
      MediaCenterValues::Header => {
        if self.headers.len() == 3 {
          self.headers = vec![self.headers[0].clone()];
        }
        self.headers.append(&mut vec![serde_json::from_str::<(String, String)>(&value).unwrap()])
      },
      MediaCenterValues::PlaybackInfo => {
        self.playback_info = Some(serde_json::from_str(&value).unwrap());
      }
    }
  }

  fn get_playback_info(&mut self) -> crate::media_center::PlaybackInfo {
    panic!("You might instead want to call: \"get_plex_playback_info()\"");
  }

  fn get_session_id(&mut self) -> Option<String> {
    self.session_id.as_ref().map(|session_id| session_id.to_string())
  }

  fn update_player(&mut self, player: &mut Player) {
    player.set_media_center(Box::new(self.clone()));
  }

  fn re_authenticate(&mut self) {
    let config = self.get_config_handle();
    if config.get_active_user().is_some() && self.check_token_valid() {
      return;
    }
    let access_token = self.create_plex_user();
    self.choose_servers(access_token.clone());
    self.get_username(access_token);
  }

  fn menu(&mut self) {
    let mut stdout = stdout();
    execute!(stdout, Hide).unwrap();
    print!("Loading menu [0/1]");
    stdout.flush().unwrap();

    let mut total: Vec<PlexItem> = vec![];
    let mut options: Vec<InteractiveOption> = vec![];
    if let Ok(mut items) = self.get_items(
      "library/recentlyAdded".to_string(), false
    ) {
      items.drain(20..);
      if !items.is_empty() {
        options.append(&mut vec![
          InteractiveOption {
            text: String::from("Recently Added:"),
            option_type: InteractiveOptionType::Header
          }
        ]);
      }
      for item in items.clone() {
        options.append(&mut vec![InteractiveOption {
          text: item.to_string_full(),
          option_type: InteractiveOptionType::Button
        }]);
      }
      total.extend(items);
    } else {
      exit(1);
    };

    options.append(&mut vec![
      InteractiveOption {
        text: String::from(""),
        option_type: InteractiveOptionType::Header
      },
      InteractiveOption {
        text: String::from("Search"),
        option_type: InteractiveOptionType::TextInput
      },
      InteractiveOption {
        text: format!("Return to {} Menu", APPNAME),
        option_type: InteractiveOptionType::Special
      }
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
            format!("hubs/search?query={}", urlencoding::encode(&search)), true
          ) {
            options.clear();
            options.append(&mut vec![InteractiveOption {
              text: format!("Search-Result for \"{}\":", search.cyan()),
              option_type: InteractiveOptionType::Header
            }]);
            for item in items.clone() {
              options.append(&mut vec![InteractiveOption {
                text: item.to_string_full(),
                option_type: InteractiveOptionType::Button
              }]);
            }
            options.append(&mut vec![
              InteractiveOption {
                text: String::from("Back"),
                option_type: InteractiveOptionType::Special
              },
              InteractiveOption {
                text: format!("Return to {} Menu", APPNAME),
                option_type: InteractiveOptionType::Special
              }
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
        _ => panic!("UNKOWN OPTION TYPE")
      }
    }
  }

  fn get(&mut self, mut url: String) -> Result<Response<Body>, Response<Body>> {
    let user = self.get_config_handle().get_active_user().unwrap();
    if !url.contains('?') {
      url.push('?')
    } else if !url.ends_with('&') {
      url.push('&')
    }
    let url = format!("{}{}X-Plex-Token={}&X-Plex-Client-Identifier={}", self.get_address(), url, user.access_token, self.config_handle.get_device_id());
    let response = Request::get(url.clone())
      .timeout(Duration::from_secs(5))
      .header("Content-Type", "application/json")
      .header("accept", "application/json")
      .body(()).unwrap()
    .send().unwrap();

    match response.status() {
      StatusCode::OK => {
        Ok(response)
      },
      _ => {
        Err(response)
      }
    }
  }

  fn report_playback(&mut self,
    item_id: String,
    playbackpositionticks: u64,
    time_pos: f64,
    _audio_track: u32,
    _sub_track: u32,
    paused: bool,
    _muted: bool,
    _volume_level: u32,
    _socket: &mut UnboundedSender<String>
  ) {
    let playback_info = self.get_plex_playback_info();
    let state: &str = if paused {
      "paused"
    } else {
      "playing"
    };

    let actual_time_position = if self.config_handle.config.transcoding {
      (playbackpositionticks as f64 * 1000.0 + time_pos * 1000.0) as u64
    } else {
      (time_pos * 1000.0) as u64
    };

    let mut url = ":/timeline".to_string();
    url += &format!("?X-Plex-Platform={}", urlencoding::encode("Plex Home Theater")); // bad request if this is missing
    url += &format!("&ratingKey={}", item_id);
    url += &format!("&key={}{}", urlencoding::encode("/library/metadata/"), item_id);
    url += &format!("&state={}", state);
    url += &format!("&time={}", actual_time_position);
    url += &format!("&duration={}", playback_info.Media.unwrap()[0].Part[0].duration);
    url += &format!("&X-Plex-Product={}&X-Plex-Device-Name={}", APPNAME, VERSION);
    url += "&hasMDE=1";

    if let Err(err) = self.get(url) {
      print_message(PrintMessageType::Error, format!("Failed to report PlaySession: {}", err.status()).as_str());
    }
  }

  fn start_playback(&mut self,
    _item_id: String,
    _playbackpositionticks: u64
  ) {
    // yea I don't think this is necessary at all for plex.
  }

  fn stop_playback(&mut self, 
    item_id: String,
    playbackpositionticks: u64,
    total_runtime: u64,
    time_pos: f64
  ) -> bool {
    let playback_info = self.get_plex_playback_info();
    let mut time_position = (time_pos * 1000.0).round() as u64;
    let time_as_secs = time_pos / 1000.0;

    if self.get_config_handle().config.transcoding {
      time_position += playbackpositionticks;
    };

    let mut url = ":/timeline".to_string();
    url += &format!("?X-Plex-Platform={}", urlencoding::encode("Plex Home Theater")); // bad request if this is missing
    url += &format!("&ratingKey={}", item_id);
    url += &format!("&key={}{}", urlencoding::encode("/library/metadata/"), item_id);
    url += &format!("&time={}", time_position);
    url += &format!("&duration={}", playback_info.Media.unwrap()[0].Part[0].duration);
    url += &format!("&X-Plex-Product={}&X-Plex-Device-Name={}", APPNAME, VERSION);
    url += "&state=stopped";
    url += "&hasMDE=1";
    
    if let Err(err) = self.get(url) {
      print_message(PrintMessageType::Error, format!("Failed to report PlaySession as stopped: {}", err.status()).as_str());
    }

    let success_message: String;
    let difference = (((total_runtime  * 1000) as f64) - time_position as f64) / ((total_runtime * 1000) as f64);
    if difference < 0.15 { // watched more than 75%
      self.item_set_playstate(playback_info.ratingKey, true);
      // yeah I guess it could fail but who cares.
      print_message(PrintMessageType::Success, "Marked item as [Played].");
      return true;
    } else if difference < 0.85 { // watched more than 15%
      let formatted: String = if time_as_secs > 60.0 {
        if (time_as_secs / 60.0) > 60.0 {
          format!("{:02}:{:02}:{:02}",
            ((time_as_secs / 60.0) / 60.0).trunc(),
            ((((time_as_secs / 60.0) / 60.0) - ((time_as_secs / 60.0) / 60.).trunc()) * 60.0).trunc(),
            (((time_as_secs / 60.0) - (time_as_secs / 60.0).trunc()) * 60.0).trunc()
          )
        } else {
          format!("00:{:02}:{:02}",
            (time_as_secs / 60.0).trunc(),
            (((time_as_secs / 60.0) - (time_as_secs / 60.0).trunc()) * 60.0).trunc()
          )
        }
      } else {
        time_as_secs.to_string()
      };
      success_message = format!("Playback progress ({}) has been sent to your server.", formatted)
    } else {
      self.item_set_playstate(playback_info.ratingKey, false);
      success_message = "Playback progress of this item has not been changed.".to_string();
    }
    print_message(PrintMessageType::Success, &success_message);
    false
  }

  fn item_set_playstate(&mut self, key: String, played: bool) {
    let status_str = if played {
      "Played"
    } else {
      "Un-Played"
    };
    let mut url = if played {
      String::from(":/scrobble")
    } else {
      String::from(":/unscrobble")
    };
    url += &format!("?identifier=com.plexapp.plugins.library&key={}", key);
    if let Err(err) = self.get(url) {
      print_message(PrintMessageType::Error, format!("Failed to mark item as {}: {}", status_str, err.status()).as_str());
    }
  }
}

impl PlexServer {
  fn series_set_playstate(&mut self, series: Series, indexes: Vec<usize>, played: bool) {
    let mut index = 0;
    for season in series.seasons {
      for episode in season.episodes {
        if indexes.contains(&index) {
          self.item_set_playstate(episode.ratingKey, played);
        }
        index += 1;
      }
    }
  }

  fn get_plex_playback_info(&mut self) -> PlexItem {
    self.playback_info.clone().unwrap()
  }

  fn process_item(&mut self, item: PlexItem) {
    println!("Selected: {}", item.to_string().cyan());
    let mut playlist: Vec<PlexItem> = vec![];
    match item.r#type.as_str() {
      "movie" => {
        playlist.push(item);
      },
      "episode" => {
        let series = self.resolve_series(item.clone());
        let mut found = false;
        for season in series.seasons {
          if found {
            playlist.extend(season.episodes);
          } else {
            for episode in season.episodes {
              if found {
                playlist.push(episode);
              } else if episode.ratingKey == item.ratingKey {
                found = true;
                playlist.push(episode);
              }
            }
          }
        }
      },
      "show" | "season" => {
        let series = self.resolve_series(item);
        playlist = self.choose_from_series(series);
      },
      _ => ()
    }
    if playlist.is_empty() {
      return;
    }
    
    let handle = self.get_config_handle();
    let user = handle.get_active_user().unwrap();
    let device_id = handle.get_device_id();
    let auth = format!("X-Plex-Token={}&X-Plex-Client-Identifier={}", user.access_token, device_id);
    let server_address = self.get_address();
    
    let settings = self.get_settings().clone();
    let mut player = Player::new(self.get_config_handle().clone(), settings.clone());

    let mut transcoding_settings = None;
    let mut player_settings: (Option<u32>, Option<u32>) = (None, None);
    let mut index = 0;
    let mut stdout = stdout();
    while index < playlist.len() {
      if let Ok(mut item) = self.get_item(playlist[index].clone().ratingKey) {
        let mut next_index = index + 1;
        let mut streamable_item = item.clone();
        if self.create_transcoding_info(&mut streamable_item, &mut transcoding_settings).is_ok() {
          self.insert_value(MediaCenterValues::PlaybackInfo, serde_json::to_string(&streamable_item).unwrap());
          self.update_player(&mut player);
          player.set_plex_video(item.clone(), server_address.clone(), auth.clone(), player_settings);
          let ret = player.play();
          'playback_done: loop {
            let mut options: Vec<InteractiveOption> = vec![];
            execute!(stdout, DisableLineWrap).unwrap();
            player_settings.0 = ret.preferred_audio_track;
            player_settings.1 = ret.preferred_subtitle_track;
            if !ret.played {
              if let Ok(updated_item) = self.get_item(item.ratingKey.clone()) {
                item = updated_item;
              } else {
                print_message(PrintMessageType::Error, format!("Failed to get updated information for {}.", item.to_string()).as_str())
              }
              options.append(&mut vec![
                InteractiveOption {
                  text: format!("Finish: {}", item.to_string_full()),
                  option_type: InteractiveOptionType::Button
                },
                InteractiveOption {
                  text: format!("Mark as played: {}", item.to_string_full()),
                  option_type: InteractiveOptionType::Button
                }
              ]);
            }
            while let Some(next_item) = playlist.get(index+1) { 
                    // skip every item that has been played already
                    // (might want to use unmark in the menu before watching a series again)
              if next_item.viewCount.is_none() {
                options.push(InteractiveOption {
                  text: format!("Continue with: {}", next_item.to_string()),
                  option_type: InteractiveOptionType::Button5s
                });
                break;
              }
              next_index += 1;
            }
            options.append(&mut vec![
              InteractiveOption {
                text: "Back to Menu".to_string(),
                option_type: InteractiveOptionType::Special
              },
              InteractiveOption {
                text: "Exit Application".to_string(),
                option_type: InteractiveOptionType::Special
              },
            ]);
            match interactive_select(options) {
              ((_, _), Some(text), InteractiveOptionType::Button) => {
                if text.starts_with("Finish") {
                  transcoding_settings.as_mut().unwrap().0 = true;
                  break 'playback_done;
                } else if text.starts_with("Mark") {
                  self.item_set_playstate(item.ratingKey.clone(), true);
                  continue 'playback_done;
                } else if text.starts_with("Continue") {
                  index = next_index;
                  break 'playback_done;
                }
              },
              ((_, _), Some(text), InteractiveOptionType::Special) => {
                match text.as_str() {
                  "Back to Menu" => {
                    return;
                  },
                  _ => exit(0)
                }
              },
              _ => ()
            }
            execute!(stdout, EnableLineWrap).unwrap();
          }
        }
      }
    }
  }

  fn create_transcoding_info(&mut self, item: &mut PlexItem, previous_settings: &mut Option<(bool, u32, u32, String)>) -> Result<(), ()> {
    let handle = self.get_config_handle();
    let mut stdout = stdout();
    execute!(stdout, SavePosition).unwrap();
    let mut mbps = String::new();
    let mut audio_track_index: u32 = 0;
    let mut subtitle_track_index: u32 = 0;
    
    let metadata_key = &item.ratingKey;
    let media_part_id;
    let mut media_file_index = 0;
    let mut media_file_list: Vec<PlexMediaFile> = if let Some(media_files) = &item.Media {
      media_files.iter().filter(|f| f.deletedAt.is_none()).cloned().collect()
    } else {
      return Err(());
    };

    // This is the only setting which isn't saved across the playlist. Don't really see the point in that tbh.
    if media_file_list.len() > 1 {
      let mut options: Vec<InteractiveOption> = vec![InteractiveOption {
        text: "\nPlease select from the following files:".to_string(),
        option_type: InteractiveOptionType::Header
      }];
      for media_file in media_file_list.clone() {
        options.push(InteractiveOption {
          text: media_file.Part[0].file.split_terminator('/').last().unwrap().to_string(),
          option_type: InteractiveOptionType::Button
        });
      }
      let ((index, _), ..) = interactive_select(options);
      media_file_index = index;
      media_part_id = media_file_list[index].Part[0].id;
      enable_raw_mode().unwrap();
      execute!(stdout, RestorePosition, MoveToColumn(0), Clear(ClearType::FromCursorDown)).unwrap();
      disable_raw_mode().unwrap();
    } else {
      media_part_id = media_file_list[0].Part[0].id;
    }

    if handle.config.transcoding {
      let time = (item.viewOffset.unwrap_or(0) as f64) / 1000.0;
      let formated: String = if time > 60.0 {
        if (time / 60.0) > 60.0 {
          format!("{:02}:{:02}:{:02}",
            ((time / 60.0) / 60.0).trunc(),
            ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(),
            (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
          )
        } else {
          format!("00:{:02}:{:02}", (time / 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
        }
      } else {
        time.to_string()
      };
      if time != 0.0 && !previous_settings.clone().unwrap_or((false, 0, 0, String::new())).0 {
        print!("\nDo you want to continue at: {}?\n  (Y)es | (N)o", formated.cyan().bold());
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
                if input.split('.').collect::<Vec<&str>>().get(1).unwrap().len() > 8 {
                  print!("\nInvalid input, please lower the amount of decimal places.\n: ");
                } else {
                  break
                }
              } else {
                break
              }
            }
            item.viewOffset = Some((input.trim().parse::<f64>().unwrap() * 60.0 * 1000.0).to_string().parse::<u64>().unwrap());
          },
          _ => ()
        }
      }

      enable_raw_mode().unwrap();
      execute!(stdout, RestorePosition, MoveToColumn(0), Clear(ClearType::FromCursorDown)).unwrap();
      disable_raw_mode().unwrap();

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
            break
          }
        };
      }

      enable_raw_mode().unwrap();
      execute!(stdout, RestorePosition, MoveToColumn(0), Clear(ClearType::FromCursorDown), EnterAlternateScreen).unwrap();
      disable_raw_mode().unwrap();
      
      let mut audio_tracks: Vec<PlexStream> = vec![];
      let mut subtitle_tracks: Vec<PlexStream> = vec![];

      if let Some(mediaFiles) = item.clone().Media {
        println!();
        let media = &mediaFiles[media_file_index];
        if let Some(stream) = &media.Part[0].Stream {
          for media_stream in stream {
            match media_stream.streamType {
              2 => audio_tracks.push(media_stream.clone()),
              3 => subtitle_tracks.push(media_stream.clone()),
              _ => ()
            }
          }
        } else {
          panic!("Did item doesn't have any streams?? That's weird.");
        }
        if audio_tracks.len() > 1 {
          let mut skip = false;
          if let Some((_, selection, _, _)) = previous_settings {
            for track in audio_tracks.clone() {
              if track.index == Some(*selection) {
                skip = true;
                audio_track_index = *selection;
                break;
              }
            }
          }
          if !skip {
            let mut options: Vec<InteractiveOption> = vec![
              InteractiveOption {
                text: "Please choose which audio track to use:".to_string(),
                option_type: InteractiveOptionType::Header
              }
            ];
            for track in audio_tracks.clone() {
              options.push(InteractiveOption {
                text: track.to_string(),
                option_type: InteractiveOptionType::Button
              });
            }
            if let ((ind, _), _, InteractiveOptionType::Button) = interactive_select(options) {
              audio_track_index = ind as u32;
            }
          }
        }
        if subtitle_tracks.len() > 1 {
          let mut skip = false;
          if let Some((_, _, selection, _)) = previous_settings {
            for track in subtitle_tracks.clone() {
              if track.index == Some(*selection) {
                skip = true;
                subtitle_track_index = *selection;
                break;
              }
            }
          }
          if !skip {
            let mut options: Vec<InteractiveOption> = vec![
              InteractiveOption {
                text: "Please choose which subtitle track to use:".to_string(),
                option_type: InteractiveOptionType::Header
              }
            ];
            for track in subtitle_tracks.clone() {
              options.push(InteractiveOption {
                text: track.to_string(),
                option_type: InteractiveOptionType::Button
              });
            }
            if let ((ind, _), _, InteractiveOptionType::Button) = interactive_select(options) {
              subtitle_track_index = ind as u32;
            }
          }
        }
      } else {
        panic!("This item has not media information. Forgot the \"MediaSources\" field?");
      }
  
      let mut selected_tracks = format!("library/parts/{}?allParts=1", media_part_id);
      if !audio_tracks.is_empty() {
        selected_tracks += &format!("&audioStreamID={}", audio_tracks[audio_track_index as usize].id);
      }
      if !subtitle_tracks.is_empty() {
        selected_tracks += &format!("&subtitleStreamID={}", subtitle_tracks[subtitle_track_index as usize].id);
      }
  
      if let Err(err) = self.put(selected_tracks) {
        print_message(PrintMessageType::Error, format!("Failed to set audio/subtitle tracks: {}", err.status()).as_str());
        return Err(());
      }

      enable_raw_mode().unwrap();
      execute!(stdout, RestorePosition, MoveToColumn(0), Clear(ClearType::FromCursorDown), LeaveAlternateScreen).unwrap();
      disable_raw_mode().unwrap();
    }


    *previous_settings = Some((false, audio_track_index, subtitle_track_index, mbps.clone()));

    let id_to_keep = media_file_list[media_file_index].id;
    media_file_list.retain(|file| file.id == id_to_keep);
    item.Media = Some(media_file_list);

    let mut decision_url = format!(
      "video/:/transcode/universal/decision?session={}",
      self.get_config_handle().get_device_id()
    );

    if !mbps.is_empty() {
      let bitrate = mbps.trim().parse::<u64>().unwrap() * 1000;
      decision_url += &format!("&maxVideoBitrate={}", bitrate);
      decision_url += "&directPlay=0";
    } else {
      decision_url += "&directPlay=1";
    }

    decision_url += "&directStream=1";
    decision_url += "&copyts=1";
    decision_url += "&fastSeek=1";
    decision_url += "&subtitles=embedded";
    decision_url += "&directStreamAudio=1";
    decision_url += "&protocol=http";
    decision_url += "&hasMDE=1";
    decision_url += "&mediaIndex=0";
    decision_url += "&partIndex=0";
    decision_url += &format!("&X-Plex-Platform={}", urlencoding::encode("Plex Home Theater")); // bad request if this is missing
    decision_url += "&X-Plex-Client-Profile-Extra=";
    decision_url += &urlencoding::encode(&PLEX_CLIENT_PROFILES.replace('\n', ""));

    decision_url += "&path=";
    decision_url += &urlencoding::encode(&format!("/library/metadata/{}", metadata_key));

    if let Err(mut err) = self.get(decision_url) {
      print_message(PrintMessageType::Error, format!("Failed to post playback information: {}", err.text().unwrap()).as_str());
      return Err(());
    }
    Ok(())
  }

  fn put(&mut self, mut url: String) -> Result<Response<Body>, Response<Body>> {
    let user = self.get_config_handle().get_active_user().unwrap();
    if !url.contains('?') {
      url.push('?')
    } else if !url.ends_with('&') {
      url.push('&')
    }
    let url = format!("{}{}X-Plex-Token={}&X-Plex-Client-Identifier={}", self.get_address(), url, user.access_token, self.config_handle.get_device_id());
    let response = Request::put(url)
      .timeout(Duration::from_secs(5))
      .header("Content-Type", "application/json")
      .header("accept", "application/json")
      .body(()).unwrap()
    .send().unwrap();

    match response.status() {
      StatusCode::OK => {
        Ok(response)
      },
      _ => {
        Err(response)
      }
    }
  }

  fn generate_series_structure(&mut self, series: &Series) -> Vec<String> {
    let mut trans_item = series.seasons[0].item.clone();
    trans_item.r#type = String::from("show");
    trans_item.title = trans_item.clone().parentTitle.unwrap_or("???".to_string());

    let full_size = {
      let mut size = 0;
      series.seasons.iter().for_each(|s| {
        size += s.episodes.len()
      });
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
      line.push_str(format!("{}", season.item.title.clone().bold()).as_str());
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
        if 13 + zero_pad_amount + index.to_string().len() + episode.to_string().len() > terminal_size {
          line.push_str(format!("{}...", episode.to_string()).as_str());
          just_text.push(line.clone());
          line.clear();
        } else {
          line.push_str(episode.to_string().as_str());
          just_text.push(line.clone());
          line.clear();
        }
        index += 1;
      }
    }
    just_text
  }

  fn choose_from_series(&mut self, mut series: Series) -> Vec<PlexItem> {
    loop {
      let mut selection: usize;
      match plex_series_select(self.generate_series_structure(&series), series.clone()) {
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
        _ => panic!("What?!")
      }
      let mut items: Vec<PlexItem> = vec![];
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

  fn resolve_series(&mut self, item: PlexItem) -> Series {
    let series_id: String = match item.r#type.as_str() {
      "season" | "episode" => item.parentRatingKey.unwrap(),
      "show" => item.ratingKey,
      _ => panic!("This object cannot be part of a series.")
    };
    // let user = self.get_config_handle().get_active_user().unwrap();

    let mut series = Series {
      item_id: series_id,
      seasons: vec![]
    };

    if let Ok(items) = self.get_items(
      format!("library/metadata/{}/children", series.item_id),
      false
    ) {
      for season in items {
        series.seasons.append(&mut vec![Season {
          item: season,
          episodes: vec![]
        }]);
      }
    } else {
      exit(1);
    };

    // if series.seasons[0].item.Name == String::from("Specials") {
    //   let specials = series.seasons.remove(0);
    //   series.seasons.push(specials);
    // }

    let mut episode_ids: Vec<String> = vec![];
    for (season_index, season) in series.seasons.clone().iter().enumerate() {
      if let Ok(items) = self.get_items(
        format!("library/metadata/{}/children", season.item.ratingKey),
        false
      ) {
        for episode in items {
          if !episode_ids.contains(&episode.ratingKey) {
            episode_ids.push(episode.ratingKey.clone());
            series.seasons[season_index].episodes.append(&mut vec![episode]);
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

  fn get_item(&mut self, ratingKey: String) -> Result<PlexItem, ()> {
    let url = format!("library/metadata/{}", ratingKey);
    match self.get(url.clone()) {
      Ok(mut result) => {
        if let Ok(library) = serde_json::from_str::<PlexLibrary>(&result.text().unwrap()) {
          if let Some(items) = library.MediaContainer.Metadata {
            return Ok(items[0].clone());
          }
        } else {
          print_message(PrintMessageType::Error, "The message returned from the server could not be processed.");
        }
      },
      Err(mut e) => {
        print_message(PrintMessageType::Error, format!("Failed to get item list at \"{}\"\n{}\n", url, e.text().unwrap()).as_str());
      }
    }
    Err(())
  }

  fn get_items(&mut self, url: String, hubs: bool) -> Result<Vec<PlexItem>, ()> {
    match self.get(url.clone()) {
      Ok(mut result) => {
        if let Ok(library) = serde_json::from_str::<PlexLibrary>(&result.text().unwrap()) {
          if hubs {
            let mut all_items: Vec<PlexItem> = vec![];
            if let Some(hubs) = library.MediaContainer.Hub {
              for hub in hubs {
                match hub.r#type.as_str() {
                  "movie" | "show" => {
                    if let Some(items) = hub.Metadata {
                      all_items.extend(items);
                    }
                  },
                  _ => ()
                }
              }
            }
            return Ok(all_items);
          } else if let Some(items) = library.MediaContainer.Metadata {
            return Ok(items);
          }
        } else {
          print_message(PrintMessageType::Error, "The message returned from the server could not be processed.");
        }
      },
      Err(mut e) => {
        print_message(PrintMessageType::Error, format!("Failed to get item list at \"{}\"\n{}\n", url, e.text().unwrap()).as_str());
      }
    }
    Err(())
  }

  fn create_plex_user(&mut self) -> String {
    let (pin_sender, pin_receiver) = mpsc::channel();
    let device_id = self.get_config_handle().get_device_id();
    let queries = format!("?X-Plex-Client-Identifier={}&X-Plex-Device-Name={}", device_id, APPNAME);
    thread::spawn(move || {
      let mut pin: Option<PlexCreatePin> = None;
      loop {
        let req: Result<Response<Body>, Response<Body>>;
        if let Some(ref old_pin) = pin {
          let get_url = format!("pins/{}.json{}", old_pin.id, queries);
          req = plex_tv(RequestType::Get, None, device_id.clone(), get_url);
        } else {
          let post_url = format!("pins.json{}", queries);
          req = plex_tv(RequestType::Post, None, device_id.clone(), post_url.clone());
        }
        let new_pin: PlexCreatePin;
        match req {
          Ok(mut response) => {
            let json = serde_json::from_str::<Value>(&response.text().unwrap()).unwrap();
            if let Ok(res_pin) = serde_json::from_value::<PlexCreatePin>(json.get("pin").unwrap().clone()) {
              new_pin = res_pin;
            } else {
              sleep(Duration::from_secs(5));
              continue;
            }
          },
          Err(_err) => {
            sleep(Duration::from_secs(5));
            continue;
          }
        }
        if Some(new_pin.clone()) != pin {
          pin_sender.send(new_pin.clone()).unwrap();
        }
        if new_pin.auth_token.is_some() {
          break;
        }
        pin = Some(new_pin);
        sleep(Duration::from_secs(5));
      }
    });
    let device_id = self.get_config_handle().get_device_id();
    let mut stdout = stdout();
    let access_token: String;
    loop {
      if let Ok(pins) = pin_receiver.recv() {
        if let Some(ref auth) = pins.auth_token {
          println!(" - {}", "Success".cyan());
          access_token = auth.to_owned();
          break;
        }
        execute!(stdout, MoveToColumn(0), Clear(ClearType::FromCursorDown), Hide).unwrap();
        print!("To link your Plex account visit: {} from a web browser and enter the code: {}", "https://plex.tv/link".cyan().underlined(), pins.code.cyan().bold());
        stdout.flush().unwrap();
      }
    }
    self.get_config_handle().insert_specific_value(Objective::DeviceID, device_id);
    execute!(stdout, Show).unwrap();
    access_token
  }

  fn choose_servers(&mut self, access_token: String) {
    let device_id = self.get_config_handle().get_device_id();
    let url = format!("api/v2/resources?includeHttps=1&includeRelay=1&X-Plex-Features=external-media&X-Plex-Language=en&X-Plex-Token={}", access_token);
    let server_name: String;
    let address: String;
    match plex_tv(RequestType::Get, None, device_id, url) {
      Ok(mut response) => {
        if let Ok(json) = serde_json::from_str::<Vec<PlexResources>>(&response.text().unwrap()) {
          let mut options: Vec<InteractiveOption> = vec![InteractiveOption {
            text: "Please choose which server you want to use (don't forget ports):".to_string(),
            option_type: InteractiveOptionType::Header
          }];
          for device in json.clone() {
            if device.provides == "server" {
              options.push(InteractiveOption {
                text: format!("{} - {}", device.name, device.publicAddress),
                option_type: InteractiveOptionType::Button
              })
            }
          }
          options.push(InteractiveOption {
            text: r#"Enter "{NAME},{ADDRESS}""#.to_string(),
            option_type: InteractiveOptionType::TextInput
          });
          loop {
            match interactive_select(options.clone()) {
              ((index, _), _, InteractiveOptionType::Button) => {
                server_name = json[index].name.clone();
                address = json[index].publicAddress.clone();
                break;
              },
              ((_, _), Some(input), InteractiveOptionType::TextInput) => {
                let split = input.split_terminator(',').collect::<Vec<&str>>();
                if split.len() != 2 {
                  continue;
                } else {
                  server_name = split[0].to_string();
                  address = split[1].to_string();
                  break;
                }
              }
              _ => ()
            }
          }
        } else {
          print_message(PrintMessageType::Error, "Failed to serialize server list of user.");
          exit(1);
        }
      },
      Err(err) => {
        print_message(PrintMessageType::Error, format!("Failed to get server list of user: {}", err.status()).as_str());
        exit(1);
      }
    }
    let handle = self.get_config_handle();
    handle.config.server_name = server_name;
    handle.insert_specific_value(Objective::Address, address);
    // handle.save();
  }

  fn check_token_valid(&mut self) -> bool { // WTF IS THIS. use the local media server to check if the token is valid
    let device_id = self.get_config_handle().get_device_id();
    let user = self.get_config_handle().get_active_user().unwrap();
    let url = "api/v2/user".to_string();
    print!("Logging in with {} on {} ", user.clone().username.cyan(), self.get_config_handle().config.server_name.clone().cyan());
    match plex_tv(RequestType::Get, Some(user), device_id, url) {
      Ok(_) => {
        println!("{}\n", "🗸".green());
        true
      }
      Err(err) => {
        println!("{}", "𐄂".red());
        print_message(PrintMessageType::Error, format!("Failed to login: {}", err.status()).as_str());
        false
      }
    }
  }

  fn get_username(&mut self, access_token: String) {
    let device_id = self.get_config_handle().get_device_id();
    let url = format!("api/v2/user?X-Plex-Token={}", access_token);
    match plex_tv(RequestType::Get, None, device_id, url) {
      Ok(mut req) => {
        if let Ok(json) = serde_json::from_str::<PlexTVUser>(&req.text().unwrap()) {
          let user = UserConfig {
            access_token,
            username: json.username,
            user_id: json.id.to_string()
          };
          let config = self.get_config_handle();
          config.insert_specific_value(Objective::User, serde_json::to_string(&user).unwrap());
          config.set_active_user(user.access_token);
          if config.check_existing_config() {
            loop {
              print_message(PrintMessageType::Error, "A media-center configuration with that file name already exists.\nPlease choose a different file name.");
              let file_name = take_string_input(vec![]);
              config.config.server_name = config.config.server_name.replace(' ', "_");
              let config_path = dirs::config_dir().unwrap();
              let config_file_path = format!("{}/{}/media-center/{}.json", &config_path.display().to_string(), APPNAME.to_lowercase(), file_name);
              config.path = config_file_path;
              if !config.check_existing_config() {
                break;
              }
            }
          }
          config.save();
        } else {
          exit(1);
        }
      },
      Err(err) => {
        print_message(PrintMessageType::Error, format!("Failed to get user information: {}", err.status()).as_str());
      }
    }
  }
}

#[derive(PartialEq)]
enum RequestType {
  Get,
  Post
}

/// Function to access the public api at plex.tv. NOT FOR INDIVIDUAL INSTANCES!
fn plex_tv(request_type: RequestType, user: Option<UserConfig>, device_id: String, url: String) -> Result<Response<Body>, Response<Body>> {
  let mut modded_url = format!("https://plex.tv/{}", url);
  if modded_url.contains('?') {
    modded_url += "&";
  } else {
    modded_url += "?";
  }
  
  modded_url += format!("X-Plex-Client-Identifier={}", device_id).as_str();

  if let Some(user) = user {
    modded_url += format!("&X-Plex-Token={}", user.access_token).as_str()
  }

  let builder = if request_type == RequestType::Get {
    Request::get(modded_url)
  } else {
    Request::post(modded_url)
  };

  let response = builder
    .timeout(Duration::from_secs(5))
    .header("Content-Type", "application/json")
    .header("accept", "application/json")
    .header("User-Agent", APPNAME)
    .body(()).unwrap()
  .send().unwrap();

  match response.status() {
    StatusCode::OK | StatusCode::CREATED => {
      Ok(response)
    },
    _ => {
      Err(response)
    }
  }
}
