use std::{thread, sync::{Arc, Mutex}};

use crate::mediaserver_information;
use discord_presence::Client;
use mediaserver_information::HeadDict;


pub struct DiscordClient {
  pub discord_client: Arc<Mutex<discord_presence::Client>>
}


impl DiscordClient {
  pub fn new() -> Self {
    let client = Client::new(980093587314343957);
    Self {
      discord_client: Arc::new(Mutex::new(client))
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
        if let Ok(_) = discord_client.clear_activity() {
          return;
        }
      }
    });
  }

  pub fn update_presence(&mut self, head_dict: &HeadDict, details: String, state: String, time_left: f64) {
    let media_server_name = head_dict.media_server_name.to_lowercase().clone();
    let discord_clone = Arc::clone(&self.discord_client);
    thread::spawn(move || {
      if let Ok(mut discord_client) = discord_clone.lock() {
        if details == *"" {
          let _ = discord_client
            .set_activity(|a| {
              a.assets(|ass| {
                ass.small_image(media_server_name)
              })
              .timestamps(|time| {
                time.end(time_left.round() as u64)
              })
              .state(&state)
          });
        } else {
          let _ = discord_client
            .set_activity(|a| {
              a.assets(|ass| {
                ass.small_image(media_server_name)
              })
              .timestamps(|time| {
                time.end(time_left.round() as u64)
              })
              .details(&details)
              .state(&state)
          });
        }
      }
    });
  }

  pub fn pause(&mut self, head_dict: &HeadDict, details: String, state: String) {
    let media_server_name = head_dict.media_server_name.to_lowercase().clone();
    let discord_clone = Arc::clone(&self.discord_client);
    thread::spawn(move || {
      if let Ok(mut discord_client) = discord_clone.lock() {
        if details == *"" {
          discord_client
            .set_activity(|a| {
              a.assets(|ass| {
                ass.large_image(media_server_name)
                .small_image("pause2")
              })
            .details(&state)
          }).ok();
        } else {
          discord_client
            .set_activity(|a| {
              a.assets(|ass| {
                ass.large_image(media_server_name)
                .small_image("pause2")
              })
            .details(&details)
            .state(&state)
          }).ok();
        }
      }
    });
  }
}
