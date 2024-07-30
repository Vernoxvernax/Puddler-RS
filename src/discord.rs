use std::{
  sync::{Arc, Mutex},
  thread,
};

use crate::media_config::MediaCenterType;
use discord_presence::Client;

pub struct DiscordClient {
  pub discord_client: Arc<Mutex<discord_presence::Client>>,
}

impl DiscordClient {
  pub fn new() -> Self {
    let client = Client::new(980093587314343957);
    Self {
      discord_client: Arc::new(Mutex::new(client)),
    }
  }

  pub fn start(&mut self) {
    let discord_clone = Arc::clone(&self.discord_client);
    thread::spawn(move || {
      if let Ok(mut discord_client) = discord_clone.lock() {
        discord_client.start();
      }
    });
  }

  pub fn stop(&mut self) {
    let discord_clone = Arc::clone(&self.discord_client);
    thread::spawn(move || {
      if let Ok(mut discord_client) = discord_clone.lock() {
        if discord_client.clear_activity().is_ok() {}
      }
    });
  }

  pub fn update_presence(
    &mut self,
    media_center_type: MediaCenterType,
    details: String,
    state: String,
    time_left: f64,
  ) {
    let media_server_name = media_center_type.to_string().to_lowercase();
    let discord_clone = Arc::clone(&self.discord_client);
    thread::spawn(move || {
      if let Ok(mut discord_client) = discord_clone.lock() {
        if details == *"" {
          let _ = discord_client.set_activity(|a| {
            a.assets(|ass| ass.small_image(media_server_name))
              .timestamps(|time| time.end(time_left.round() as u64))
              .state(&state)
          });
        } else {
          let _ = discord_client.set_activity(|a| {
            a.assets(|ass| ass.small_image(media_server_name))
              .timestamps(|time| time.end(time_left.round() as u64))
              .details(&details)
              .state(&state)
          });
        }
      }
    });
  }

  pub fn pause(&mut self, media_center_type: MediaCenterType, details: String, state: String) {
    let media_server_name = media_center_type.to_string().to_lowercase();
    let discord_clone = Arc::clone(&self.discord_client);
    thread::spawn(move || {
      if let Ok(mut discord_client) = discord_clone.lock() {
        if details == *"" {
          discord_client
            .set_activity(|a| {
              a.assets(|ass| ass.large_image(media_server_name).small_image("pause2"))
                .details(&state)
            })
            .ok();
        } else {
          discord_client
            .set_activity(|a| {
              a.assets(|ass| ass.large_image(media_server_name).small_image("pause2"))
                .details(&details)
                .state(&state)
            })
            .ok();
        }
      }
    });
  }
}
