use std::io;
use std::io::prelude::*;
use std::thread;
use std::time::Duration;
use colored::Colorize;
use libmpv::Mpv;
use libmpv::events::Event;
use serde_derive::Deserialize;
use serde::Serialize;
use isahc::ReadResponseExt;
use crate::getch;
use crate::discord;
use crate::discord::DiscordClient;
use crate::APPNAME;
use crate::Item;
use crate::mediaserver_information::HeadDict;
use crate::mediaserver_information::post_puddler;
use crate::MediaStream;
use crate::puddler_get;
use crate::is_numeric;
use crate::settings::Settings;
use crate::progress_report::PlaybackInfo;
use crate::progress_report::finished_playback;
use crate::progress_report::update_progress;
use crate::progress_report::started_playing;
use std::time::SystemTime;
use dialoguer::{theme::ColorfulTheme, Select};


#[derive(Debug, Serialize, Deserialize)]
struct SessionCapabilities {
  UserId: String,
  StartTimeTicks: i64,
  MediaSourceId: String,
  AudioStreamIndex: usize,
  SubtitleStreamIndex: usize,
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
  MaxStreamingBitrate: u64,
  MaxStaticMusicBitrate: u64,
  TranscodingProfiles: Vec<TranscodingProfile>,
  SubtitleProfiles: Vec<SubtitleProfile>
}


#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirectPlayProfile {
  Type: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TranscodingProfile {
  Type: String,
  TranscodeSeekInfo: String,
  VideoCodec: String,
  Container: String
}


#[derive(Debug, Serialize, Deserialize, Clone)]
struct SubtitleProfile {
  Format: String,
  Method: String
}


pub fn player_new() -> libmpv::Mpv {
  return Mpv::new().expect("Failed to create mpv handle.");
}

pub fn player_set_properties(handler: &libmpv::Mpv, settings: &Settings, media_title: &str, title: &str) {
  if settings.fullscreen {
    handler.set_property("fullscreen", "yes").expect("Failed to configure fullscreen.");
  }

  if settings.gpu {
    handler.set_property("hwdec", "auto-safe").expect("Failed to configure hardware-decoding.")
  }
  
  handler.set_property("user-agent", APPNAME).expect("Failed to configure user-agent.");
  
  handler.set_property("force-media-title", media_title).expect("Failed to configure force-media-title.");
  handler.set_property("title", title).expect("Failed to configure title.");
}


pub fn player_set_options(builder: &libmpv::Mpv, settings: &Settings) {
  if settings.mpv_config_location.is_some() {
    builder.set_property("config-dir", settings.mpv_config_location.clone().unwrap().as_str()).unwrap();
  }

  if settings.load_config {
    builder.set_property("config", true).unwrap();
  }
  
  builder.set_property("input-default-bindings", "yes").unwrap();
  builder.set_property("input-vo-keyboard", "yes").unwrap();
  builder.set_property("osc", true).unwrap();

  if settings.mpv_debug == Some(true) {
    builder.set_property("log-file", "./mpv.log").unwrap();
  }
}


fn choose_trackIndexx(item: &Item) -> (usize, usize) {
  fn select_ind(tracks: Vec<MediaStream>, kind: &str) -> usize {
    match tracks.len() {
      n if n > 1 => {
        tracks[Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Please select which ".to_owned() + kind + " track you want to use:")
        .default(0)
        .items(&tracks[..])
        .interact()
        .unwrap()].Index
      },
      1 => {
        println!("The following {} track will be used:\n{}", kind, tracks.first().unwrap().to_string().green());
        tracks[0].Index
      },
      _ => {
        println!("This file doesn't have any {kind} track.");
        0
      }
    }
  }
  let mut subtitle_tracks: Vec<MediaStream> = [].to_vec();
  let mut audio_tracks: Vec<MediaStream> = [].to_vec();
  let mediaStreams: &Vec<MediaStream> = &item.MediaSources.as_ref().unwrap().first().unwrap().MediaStreams;
  for track in mediaStreams.iter() {
    match &track.Type as &str {
      "Audio" => audio_tracks.append(&mut [track.clone()].to_vec()),
      "Subtitle" => subtitle_tracks.append(&mut [track.clone()].to_vec()),
      _ => ()
    }
  };
  println!();
  (select_ind(audio_tracks, "audio"), select_ind(subtitle_tracks, "subtitle"))
}


pub fn play(settings: &Settings, head_dict: &HeadDict, item: &mut Item) -> bool {
  item.UserData.PlaybackPositionTicks = {
    if item.UserData.PlaybackPositionTicks == 0 && ! settings.transcoding {
      0
    } else {
      let time = (item.UserData.PlaybackPositionTicks as f64) / 10000000.0;
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
      print!("Do you want to continue at: {}?\n  (Y)es | (N)o (start from a different position)", formated.cyan());
      match getch("YyNnOo") {
        'N' | 'n' => {
          print!("Please enter a playback position in minutes: ");
          let mut input: String;
          loop {
            input = String::new();
            io::stdout().flush().expect("Failed to flush stdout");
            io::stdin().read_line(&mut input).unwrap();
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
          (input.trim().parse::<f64>().unwrap() * 60.0 * 10000000.0).to_string().parse::<i64>().unwrap()
        },
        _ => {
          item.UserData.PlaybackPositionTicks
        }
      }

    }
  };
  
  let playback_info: PlaybackInfo = if settings.transcoding {
    let (audioIndex, subIndex) = choose_trackIndexx(item);
    
    print!("\nPlease enter your connection speed in mbps: ");
    let mut mbps: String = String::new();
    loop {
      io::stdout().flush().expect("Failed to flush stdout");
      io::stdin().read_line(&mut mbps).unwrap();
      if ! is_numeric(mbps.trim()) {
        print!("\nInvalid input! Enter something like \"25\" equal to ~3MB/s.\n: ")
      } else {
        break
      }
    };

    let bitrate = mbps.trim().parse::<u64>().unwrap() * 1000000;
    let sess: SessionCapabilities = SessionCapabilities {
      UserId: head_dict.config_file.user_id.clone(),
      StartTimeTicks: item.UserData.PlaybackPositionTicks,
      MediaSourceId: item.MediaSources.as_ref().unwrap()[0].Id.clone(),
      AudioStreamIndex: audioIndex,
      SubtitleStreamIndex: subIndex,
      MaxStaticBitrate: bitrate,
      MaxStreamingBitrate: bitrate,
      EnableDirectPlay: false,
      EnableDirectStream: false,
      EnableTranscoding: true,
      AllowVideoStreamCopy: false,
      AllowAudioStreamCopy: true,
      DeviceProfile: DeviceProfile {
        Name: "mpv".to_string(),
        Id: head_dict.config_file.device_id.clone(),
        MaxStaticMusicBitrate: 999999999,
        MaxStreamingBitrate: bitrate,
        TranscodingProfiles: [
          TranscodingProfile {
            Type: "Video".to_string(),
            Container: "mkv".to_string(),
            VideoCodec: "hevc".to_string(),
            TranscodeSeekInfo: "Auto".to_string(),
          },
          TranscodingProfile {
            Type: "Video".to_string(),
            Container: "mkv".to_string(),
            VideoCodec: "avc".to_string(),
            TranscodeSeekInfo: "Auto".to_string(),
          }
        ].to_vec(),
        SubtitleProfiles: [
          SubtitleProfile {
            Format: "subrip".to_string(),
            Method: "Embed".to_string()
          },
          SubtitleProfile {
            Format: "srt".to_string(),
            Method: "Embed".to_string()
          },
          SubtitleProfile {
            Format: "ass".to_string(),
            Method: "Embed".to_string()
          },
          SubtitleProfile {
            Format: "ssa".to_string(),
            Method: "Embed".to_string()
          },
          SubtitleProfile {
            Format: "pgssub".to_string(),
            Method: "Embed".to_string()
          },
          SubtitleProfile {
            Format: "sub".to_string(),
            Method: "Embed".to_string()
          },
          SubtitleProfile {
            Format: "dvdsub".to_string(),
            Method: "Embed".to_string()
          },
          SubtitleProfile {
            Format: "pgs".to_string(),
            Method: "Embed".to_string()
          }
        ].to_vec()
      }
    };
    let playback_info_res: Result<http::Response<isahc::Body>, String> = post_puddler(format!("{}{}/Items/{}/PlaybackInfo?UserId={}", head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.config_file.user_id), &head_dict.auth_header, serde_json::to_string_pretty(&sess).unwrap());
    let playback_info: PlaybackInfo = match playback_info_res {
      Ok(mut t) => {
        let search_text: &String = &t.text().unwrap();
        serde_json::from_str(search_text).unwrap()
      }
      Err(e) => panic!("failed to parse get playback info: {e}")
    };
    playback_info
  } else {
    let playback_info_res: Result<http::Response<isahc::Body>, isahc::Error> = puddler_get(format!("{}{}/Items/{}/PlaybackInfo?UserId={}", head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.config_file.user_id), head_dict);
    let playback_info: PlaybackInfo = match playback_info_res {
      Ok(mut t) => {
        let search_text: &String = &t.text().unwrap();
        serde_json::from_str(search_text).unwrap()
      }
      Err(e) => panic!("failed to parse get playback info: {e}")
    };
    playback_info
  };

  started_playing(settings, head_dict, item, &playback_info);

  let resume_progress = item.UserData.PlaybackPositionTicks / 10000000;

  let total_runtime: f64 = if settings.transcoding {
    (item.RunTimeTicks.unwrap() as f64 - item.UserData.PlaybackPositionTicks as f64) / 10000000.0
  } else {
    item.RunTimeTicks.unwrap() as f64 / 10000000.0
  };

  let mpv = player_new();
  player_set_options(&mpv, settings);
  
  let stream_url: String = if settings.transcoding {
    format!("{}{}{}", head_dict.config_file.ipaddress, head_dict.media_server, playback_info.MediaSources.get(0).unwrap().TranscodingUrl.as_ref().unwrap())
  } else {
    format!("{}{}/Videos/{}/stream?Container=mkv&Static=true&api_key={}",
    head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.request_header.token)
  };
  
  let media_title: String;
  let title: String;
  if item.Type == "Movie" {
    media_title = format!("{} ({}) | {}", item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], head_dict.media_server_name);
    title = format!("{} - Streaming: {} ({})", APPNAME, item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]);
  } else {
    media_title = format!("{} ({}) - {} - {} | {}", item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], item.SeasonName.as_ref().unwrap(), item.Name, head_dict.media_server_name);
    title = format!("{} - Streaming: {} ({}) - {} - {}", APPNAME, item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], item.SeasonName.as_ref().unwrap(), item.Name);
  }

  player_set_properties(&mpv, settings, media_title.as_str(), title.as_str());

  let mut ctx = mpv.create_event_context();
  ctx.disable_deprecated_events().expect("Failed to disable deprecated events.");

  mpv.command("loadfile", &[&stream_url]).expect("Failed to load file.");

  // Load files provided using the --glsl-shader option.
  if settings.glsl_shader.is_some() {
    for sh in settings.glsl_shader.clone().unwrap() {
      mpv.command("change-list", &["glsl-shaders", "append", sh.as_str()]).expect("Failed to add glsl-shader file");
    }
  }

  let mut discord: DiscordClient = discord::mpv_link();
  if settings.discord_presence {
    discord.start();
  }
  let mut watched_till_end: bool = false;
  let mut old_pos: f64 = -15.0;
  let mut last_time_update: f64 = 0.0;

  'main: loop {
    while let Some(event_res) = ctx.wait_event(0.0) {
      let event = if let Ok(event) = event_res {
        event
      } else {
        eprintln!("No idea why this would happen. Please create an issue. :)");
        break 'main;
      };
      match event {
        Event::FileLoaded => {
          if resume_progress != 0 && ! settings.transcoding {
            mpv.command("seek", &[format!("{}", &resume_progress).as_str()]).expect("Failed to seek");
          }
          load_external_subtitles(settings, head_dict, &mpv, item);
        }
        Event::Shutdown | Event::EndFile(0) => {
          watched_till_end = finished_playback(settings, head_dict, item, old_pos, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id, false);
          if settings.discord_presence {
            discord.stop();
          }
          break 'main;
        }
        Event::Seek | Event::PlaybackRestart => {
          old_pos -= 16.0
        }
        _ => {
          // println!("{:#?}", event); // for debugging
        }
      }
    }
    let result: Result<f64, libmpv::Error> = mpv.get_property("time-pos");
    if let Ok(current_time) = result {
      if current_time > old_pos + 15.0 { // this was the most retarded solution, I could think of
        update_progress(settings, head_dict, item, current_time * 10000000.0, false, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id);
        if settings.discord_presence {
          if item.Type == "Movie" {
            discord.update_presence(head_dict,
              "".to_string(),
              format!("{} ({})", &item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
              SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - current_time,
            );
          } else {
            discord.update_presence(head_dict,
              format!("{} ({})", &item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
              format!("{} ({})", item.Name, item.SeasonName.as_ref().unwrap()),
              SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - current_time,
            );
          }
        }
        old_pos = current_time;
      } else if current_time == last_time_update {
        update_progress(settings, head_dict, item, current_time * 10000000.0, true, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id);
        if settings.discord_presence {
          if item.Type == "Movie" {
            discord.pause(head_dict,
              "".to_string(),
              format!("{} ({})", &item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
            );
          } else {
            discord.pause(head_dict,
              format!("{} ({})", &item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
              format!("{} ({})", item.Name, item.SeasonName.as_ref().unwrap()),
            );
          }
        }
      }
      last_time_update = current_time;
    }
    thread::sleep(Duration::from_millis(500));
  }
  drop(ctx);
  return watched_till_end;
}

fn load_external_subtitles(settings: &Settings, head_dict: &HeadDict, mpv: &libmpv::Mpv, item: &Item) {
  let ipaddress: &String = &head_dict.config_file.ipaddress;
  let item_id: &String = &item.Id;
  let media_server: &String = &head_dict.media_server;
  let mediasrc = item.MediaSources.as_ref().unwrap().get(0).unwrap();
  for (index, stream) in mediasrc.MediaStreams.iter().enumerate() {
    if stream.IsExternal && stream.SupportsExternalStream {
      let extension = if let Some(path) = &stream.Path {
        path.split(".").last().unwrap().to_string()
      } else {
        stream.Codec.as_ref().unwrap().to_owned()
      };
      let mut media_url = format!("{}{}/Videos/{}/{}/Subtitles/{}/Stream.{}?api_key={}", ipaddress, media_server, item_id, mediasrc.Id, index, extension, head_dict.request_header.token);
      if item.UserData.PlaybackPositionTicks != 0 && settings.transcoding {
        media_url += &("&StartPositionTicks=".to_owned() + &item.UserData.PlaybackPositionTicks.to_string());
      }
      let undefined_title = &String::from("Undefined");
      let undefined_lang = &String::from("und");
      let title = format!(r#""{}""#, stream.DisplayTitle.as_ref().unwrap_or(undefined_title));
      let command: [&str; 4] = [&media_url, "auto", title.as_str(), stream.Language.as_ref().unwrap_or(undefined_lang)];
      mpv.command("sub-add", &command).unwrap();
    }
  }
}
