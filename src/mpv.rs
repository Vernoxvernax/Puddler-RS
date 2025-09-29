use futures::{
  SinkExt, StreamExt,
  stream::{SplitSink, SplitStream},
};
use libmpv2::{Mpv, events::Event};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::{
  net::TcpStream,
  sync::mpsc::{self},
};
use tokio_tungstenite::{
  MaybeTlsStream, WebSocketStream, connect_async,
  tungstenite::{Message, Utf8Bytes},
};

use crate::{
  APPNAME,
  discord::DiscordClient,
  input::clear_stdin,
  media_center::ToStringAdv,
  media_center::{Item, MediaCenter, PlaybackInfo},
  media_config::Config,
  media_config::MediaCenterType,
  plex::PlexItem,
  printing::{PrintMessageType, print_message},
  puddler_settings::PuddlerSettings,
};

#[derive(Clone, PartialEq)]
pub enum VideoType {
  Movie,
  Episode,
}

#[derive(Clone, PartialEq)]
pub struct Video {
  title: Vec<String>,
  id: String,
  video_type: VideoType,
  stream_url: String,
  playback_position: u64,
  total_runtime: u64,
  external_media: Option<Vec<[String; 4]>>,
  pub played: bool,
  pub preferred_audio_track: Option<u32>,
  pub preferred_subtitle_track: Option<u32>,
}

pub struct Player {
  media_center_config: Config,
  media_center: Option<Box<dyn MediaCenter>>,
  settings: PuddlerSettings,
  video: Option<Video>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
  pub MessageType: String,
  pub Data: Value,
}

impl Player {
  pub fn new(media_center_config: Config, settings: PuddlerSettings) -> Self {
    Player {
      media_center_config,
      settings,
      video: None,
      media_center: None,
    }
  }

  pub fn set_media_center(&mut self, media_center: Box<dyn MediaCenter>) {
    self.media_center = Some(media_center);
  }

  pub fn set_plex_video(
    &mut self,
    item: PlexItem,
    server_address: String,
    auth: String,
    transcoding_settings: &mut Option<(bool, Option<u32>, Option<u32>, String)>,
  ) {
    let handle = &mut self.media_center_config;
    if handle.config.media_center_type != MediaCenterType::Plex {
      panic!("What da hell?!");
    }

    let stream_url = if handle.config.transcoding {
      format!(
        "{}video/:/transcode/universal/start.mkv?{}&path={}&subtitles=embedded&directPlay=0&directStream=1&session={}&protocol=http&X-Plex-Platform={}&fastSeek=1&offset={}",
        server_address,
        auth,
        urlencoding::encode(format!("/library/metadata/{}", item.ratingKey).as_str()),
        handle.get_device_id(),
        urlencoding::encode("Plex Home Theater"),
        item.viewOffset.unwrap_or(0) / 1000
      )
    } else {
      format!(
        "{}{}?{}&X-Plex-Platform={}",
        server_address,
        item.Media.clone().unwrap()[0].Part[0]
          .key
          .trim_start_matches('/'),
        auth,
        urlencoding::encode("Plex Home Theater")
      )
    };

    let mut commands: Vec<[String; 4]> = vec![];
    if !handle.config.transcoding {
      for stream in item.Media.clone().unwrap()[0].Part[0]
        .Stream
        .clone()
        .unwrap()
      {
        if let Some(key) = &stream.key {
          let media_url = format!("{}{}?{}", server_address, key.trim_start_matches('/'), auth);
          let language = if let Some(language) = stream.language.clone() {
            language
          } else {
            String::from("und")
          };
          let title = if let Some(title) = stream.displayTitle.clone() {
            title
          } else {
            String::from("Undefined")
          };
          let formatted_title = format!(r#""{}""#, title);
          let command: [String; 4] = [media_url, "auto".to_string(), formatted_title, language];
          commands.push(command);
        }
      }
    }

    let preferred_tracks = if let Some(settings) = transcoding_settings {
      (settings.1, settings.2)
    } else {
      (None, None)
    };

    self.video = Some(Video {
      title: item.to_string_split(),
      stream_url,
      id: item.ratingKey,
      video_type: if item.r#type == "movie" {
        VideoType::Movie
      } else {
        VideoType::Episode
      },
      playback_position: item.viewOffset.unwrap_or(0) / 1000,
      total_runtime: item.duration.unwrap_or(0) / 1000,
      external_media: if commands.is_empty() {
        None
      } else {
        Some(commands)
      },
      played: true,
      preferred_audio_track: preferred_tracks.0,
      preferred_subtitle_track: preferred_tracks.1,
    });
  }

  pub fn set_jellyfin_video(
    &mut self,
    item: Item,
    playback_info: PlaybackInfo,
    server_address: String,
    auth_token: String,
    transcoding_settings: &mut Option<(bool, Option<u32>, Option<u32>, String)>,
  ) {
    let handle = &self.media_center_config;
    if handle.config.media_center_type == MediaCenterType::Plex {
      panic!("What da hell?!");
    }

    let media_source = playback_info.MediaSources.first().unwrap();
    let stream_url = if let Some(transcoding_url) = &media_source.TranscodingUrl {
      format!(
        "{}{}",
        server_address,
        transcoding_url.trim_start_matches('/')
      )
    } else {
      format!(
        "{}Videos/{}/stream?Container=mkv&Static=true&api_key={}",
        server_address, media_source.Id, auth_token
      )
    };

    let mut commands: Vec<[String; 4]> = vec![];
    for (index, stream) in media_source.MediaStreams.iter().enumerate() {
      if stream.IsExternal && stream.SupportsExternalStream {
        let extension = if let Some(path) = &stream.Path {
          path.split('.').last().unwrap().to_string()
        } else {
          stream.Codec.as_ref().unwrap().to_owned()
        };
        let mut media_url = format!(
          "{}Videos/{}/{}/Subtitles/{}/Stream.{}?api_key={}",
          server_address, item.Id, media_source.Id, index, extension, auth_token
        );
        if item.UserData.PlaybackPositionTicks != 0 && handle.config.transcoding {
          media_url +=
            &("&StartPositionTicks=".to_owned() + &item.UserData.PlaybackPositionTicks.to_string());
        }
        let language = if let Some(language) = stream.Language.clone() {
          language
        } else {
          String::from("und")
        };
        let title = if let Some(title) = stream.DisplayTitle.clone() {
          title
        } else {
          String::from("Undefined")
        };
        let formatted_title = format!(r#""{}""#, title);
        let command: [String; 4] = [media_url, "auto".to_string(), formatted_title, language];
        commands.push(command);
      }
    }

    let preferred_tracks = if let Some(settings) = transcoding_settings {
      (settings.1, settings.2)
    } else {
      (None, None)
    };

    self.video = Some(Video {
      title: item.to_string_split(),
      stream_url,
      id: item.Id,
      video_type: if item.Type == "Movie" {
        VideoType::Movie
      } else {
        VideoType::Episode
      },
      playback_position: item.UserData.PlaybackPositionTicks / 10000000,
      total_runtime: item.RunTimeTicks.unwrap() / 10000000,
      external_media: if commands.is_empty() {
        None
      } else {
        Some(commands)
      },
      played: true,
      preferred_audio_track: preferred_tracks.0,
      preferred_subtitle_track: preferred_tracks.1,
    });
  }

  #[tokio::main]
  pub async fn play(&mut self) -> Video {
    // any time vars in here are in seconds
    let mut video: Video;
    if let Some(vid) = &self.video {
      video = vid.clone();
    } else {
      panic!("You must've forgotten to set the video.");
    }
    let handle = &mut self.media_center_config;
    let media_center: &mut Box<dyn MediaCenter> = self.media_center.as_mut().unwrap();

    let mut websocket_reader = None;
    let mut _websocket_sender = None;
    if handle.config.media_center_type != MediaCenterType::Plex {
      let http_address = media_center.get_address();
      let protocol = if http_address.contains("https") {
        "wss"
      } else {
        "ws"
      };
      let address: String = http_address
        .trim_start_matches("https")
        .trim_start_matches("http")
        .trim_end_matches('/')
        .to_string();
      let headers = media_center.get_headers();
      let token = &headers.get(2).unwrap().1;
      // the "/socket" is mandatory for jellyfin; emby doesn't care either way. documentation on this is fucking terrible
      let url = format!(
        "{}{}/socket?api_key={}&deviceId={}",
        protocol,
        address,
        token,
        handle.get_device_id()
      );
      if let Ok((socket, _)) = connect_async(url).await {
        let (sender, reader) = socket.split();
        websocket_reader = Some(reader);
        _websocket_sender = Some(sender);
      }
      // if this fails, remote control commands will not be available.
    }

    let config = &handle.config;

    let (input, _websocket_output) = mpsc::unbounded_channel::<String>();
    let (websocket_input, mut output) = mpsc::unbounded_channel();
    let websocket_read_handle = tokio::spawn(async move {
      websocket_read(websocket_reader, websocket_input).await;
    });
    // let websocket_write_handle = tokio::spawn(async move {
    //   websocket_send(websocket_sender, websocket_output).await;
    // });

    let media_title = format!(
      "{} | {}",
      video.title[0],
      config.media_center_type.to_string()
    );
    let mpv_title = format!(
      "{} - Streaming: {} ({})",
      APPNAME,
      video.title[0],
      config.media_center_type.to_string()
    );

    let mut mpv = Mpv::new().expect("Failed to create mpv handle!");

    if let Some(path) = &self.settings.mpv_config_location {
      mpv.set_property("config-dir", path.clone()).unwrap();
      mpv.set_property("config", true).unwrap();
    }

    mpv.set_property("input-default-bindings", "yes").unwrap();
    mpv.set_property("input-vo-keyboard", "yes").unwrap();
    mpv.set_property("osc", true).unwrap();

    if self.settings.mpv_debug_log {
      mpv.set_property("log-file", "./mpv.log").unwrap();
    }

    if self.settings.fullscreen {
      mpv
        .set_property("fullscreen", "yes")
        .expect("Failed to configure fullscreen.");
    }

    if self.settings.gpu {
      mpv
        .set_property("hwdec", "auto-safe")
        .expect("Failed to configure hardware-decoding.")
    }

    mpv
      .set_property("user-agent", APPNAME)
      .expect("Failed to configure user-agent.");
    mpv
      .set_property("force-media-title", media_title)
      .expect("Failed to configure force-media-title.");
    mpv
      .set_property("title", mpv_title)
      .expect("Failed to configure title.");

    mpv
      .disable_deprecated_events()
      .expect("Failed to disable deprecated events.");

    mpv
      .command("loadfile", &[&video.stream_url])
      .expect("Failed to load file.");

    media_center
      .start_playback(video.clone().id, video.playback_position)
      .await;

    for shader in &self.settings.glsl_shaders {
      mpv
        .command("change-list", &["glsl-shaders", "append", shader.as_str()])
        .expect("Failed to add glsl-shader file");
    }

    let mut discord: DiscordClient = DiscordClient::new();
    if self.settings.discord_presence {
      discord.start();
    }

    let total_runtime: f64 = if config.transcoding {
      video.total_runtime as f64 - video.playback_position as f64
    } else {
      video.total_runtime as f64
    };

    let resume_progress = video.playback_position;
    let mut paused = false;
    let mut old_pos: f64 = -15.0;
    let mut last_time_update: f64 = 0.0;
    let mut audio_track: u32 = 0;
    let mut sub_track: u32 = 0;
    let mut volume_level: u32 = 0;
    let mut muted: bool = false;
    let initial_preferences = (video.preferred_audio_track, video.preferred_subtitle_track);
    'main: loop {
      if let Ok(msg) = output.try_recv() {
        let message = msg.to_string();
        if let Ok(json_message) = serde_json::from_str::<WebSocketMessage>(&message.to_string()) {
          if json_message.MessageType == "Playstate" {
            match json_message.Data.get("Command").unwrap().as_str().unwrap() {
              "PlayPause" => {
                if paused {
                  mpv.set_property("pause", false).unwrap();
                  paused = false;
                  old_pos -= 16.0;
                } else {
                  mpv.set_property("pause", true).unwrap();
                  paused = true;
                }
              },
              "Stop" => {
                mpv.command("quit", &["0"]).unwrap();
              },
              _ => (),
            }
          }
        }
      }
      while let Some(event_res) = mpv.wait_event(0.0) {
        let event = if let Ok(event) = event_res {
          event
        } else {
          eprintln!("No idea why this would happen. Please create an issue. :)");
          break 'main;
        };
        match event {
          Event::FileLoaded => {
            if resume_progress != 0 && !config.transcoding {
              mpv
                .command("seek", &[&resume_progress.to_string()])
                .expect("Failed to seek");
            }
            // let's hope loading external subs isn't async ...
            load_external_subtitles(self.video.clone().unwrap(), &mpv);
            if let Some(audio_track_) = initial_preferences.0 {
              mpv
                .set_property("aid", audio_track_ as i64)
                .expect("Failed to set preferred audio track.");
            }
            if let Some(subtitle_track_) = initial_preferences.1 {
              mpv
                .set_property("sid", subtitle_track_ as i64)
                .expect("Failed to set preferred subtitle track.");
            }
          },
          Event::Shutdown | Event::EndFile(_) => {
            video.played = media_center
              .stop_playback(
                video.clone().id,
                video.clone().playback_position,
                video.clone().total_runtime,
                old_pos,
              )
              .await;
            if self.settings.discord_presence {
              discord.stop();
            }
            break 'main;
          },
          Event::Seek | Event::PlaybackRestart => {
            old_pos -= 16.0;
          },
          _ => {
            // println!("{:#?}", event); // for debugging
          },
        }
      }
      let result: Result<f64, libmpv2::Error> = mpv.get_property("time-pos");
      if let Ok(current_time) = result {
        if let Ok(track) = mpv.get_property::<String>("current-tracks/audio/src-id") {
          audio_track = track.parse::<u32>().unwrap();
        }
        if let Ok(track) = mpv.get_property::<String>("current-tracks/sub/src-id") {
          sub_track = track.parse::<u32>().unwrap();
        }
        if let Ok(value) = mpv.get_property::<i64>("volume") {
          volume_level = value as u32;
        }
        if let Ok(value) = mpv.get_property::<bool>("mute") {
          muted = value;
        }
        if let Ok(track) = mpv.get_property::<i64>("current-tracks/audio/id") {
          video.preferred_audio_track = Some(track as u32);
        } else {
          video.preferred_audio_track = Some(0);
        }
        if let Ok(track) = mpv.get_property::<i64>("current-tracks/sub/id") {
          video.preferred_subtitle_track = Some(track as u32);
        } else {
          video.preferred_subtitle_track = Some(0);
        }
        if current_time > old_pos + 15.0 {
          if paused {
            paused = false;
          }
          media_center
            .report_playback(
              video.clone().id,
              video.playback_position,
              current_time,
              audio_track,
              sub_track,
              paused,
              muted,
              volume_level,
            )
            .await;
          if self.settings.discord_presence {
            if video.video_type == VideoType::Movie {
              discord.update_presence(
                config.media_center_type,
                String::new(),
                video.title[0].clone(),
                total_runtime,
                current_time,
              );
            } else {
              discord.update_presence(
                config.media_center_type,
                video.title[1].clone(),
                video.title[2].clone(),
                total_runtime,
                current_time,
              );
            }
          }
          old_pos = current_time;
        } else if current_time == last_time_update {
          if current_time == old_pos {
            continue;
          }
          paused = true;
          media_center
            .report_playback(
              video.clone().id,
              video.playback_position,
              current_time,
              audio_track,
              sub_track,
              paused,
              muted,
              volume_level,
            )
            .await;
          if self.settings.discord_presence {
            if video.video_type == VideoType::Movie {
              discord.pause(
                config.media_center_type,
                String::new(),
                video.title[0].clone(),
              );
            } else {
              discord.pause(
                config.media_center_type,
                video.title[1].clone(),
                video.title[2].clone(),
              );
            }
          }
          old_pos = current_time;
        }
        last_time_update = current_time;
      }
      tokio::time::sleep(Duration::from_millis(500)).await;
    }
    if !input.is_closed() {
      input.send("stop".to_string()).unwrap()
    }
    drop(mpv);
    // websocket_write_handle.abort();
    websocket_read_handle.abort();
    clear_stdin();
    video
  }
}

fn load_external_subtitles(video: Video, mpv: &Mpv) {
  if let Some(commands) = video.external_media {
    for command in commands {
      let [media_url, auto, formatted_title, language] = command;
      mpv
        .command("sub-add", &[&media_url, &auto, &formatted_title, &language])
        .unwrap();
    }
  }
}

async fn websocket_read(
  faucet: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
  web_input: mpsc::UnboundedSender<String>,
) {
  if let Some(mut socket) = faucet {
    // can't get any more creative than that
    while let Some(Ok(msg)) = socket.next().await {
      web_input.send(msg.to_string()).unwrap();
    }
  }
}

async fn _websocket_send(
  sink: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
  mut web_output: mpsc::UnboundedReceiver<String>,
) {
  if let Some(mut socket) = sink {
    while let Some(msg) = web_output.recv().await {
      if msg == "stop" {
        socket.close().await.unwrap();
        return;
      }
      if let Err(err) = socket.send(Message::Text(Utf8Bytes::from(msg))).await {
        print_message(
          PrintMessageType::Error,
          format!("Failed to send message through websocket: {}", err).as_str(),
        )
      };
    }
  }
}
