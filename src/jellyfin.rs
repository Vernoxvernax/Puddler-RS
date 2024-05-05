use crate::{media_center::{MediaCenter, MediaCenterValues, PlaybackInfo}, media_config::Config, mpv::Player, puddler_settings::PuddlerSettings, APPNAME, VERSION};

#[derive(Clone)]
pub struct JellyfinServer {
  config: Config,
  headers: Vec<(String, String)>,
  session_id: Option<String>,
  settings: PuddlerSettings,
  playback_info: Option<PlaybackInfo>
}

impl MediaCenter for JellyfinServer {
  fn new(mut config: Config, settings: PuddlerSettings) -> Self {
    JellyfinServer {
      config: config.clone(),
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
    &mut self.config
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

  fn get_session_id(&mut self) -> Option<String> {
    self.session_id.as_ref().map(|session_id| session_id.to_string())
  }

  fn get_playback_info(&mut self) -> PlaybackInfo {
    self.playback_info.clone().unwrap()
  }

  fn update_player(&mut self, player: &mut Player) {
    player.set_media_center(Box::new(self.clone()));
  }
}
