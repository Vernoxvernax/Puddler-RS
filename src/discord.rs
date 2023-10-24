use discord_presence::Client;

use crate::mediaserver_information;
use mediaserver_information::HeadDict;


pub struct DiscordClient {
  pub client: discord_presence::Client
}


pub fn mpv_link(use_discord: bool) -> DiscordClient {
  if use_discord {
    DiscordClient::default()
  } else {
    DiscordClient { client: Client::new(980093587314343957) } // not starting it though
  }
}


impl Default for DiscordClient {
  fn default() -> Self {
    let mut client = Client::new(980093587314343957);
    client.start().is_finished();
    Self {
      client
    }
  }
}


impl DiscordClient {
  pub fn stop(&mut self) {
    self.client.clear_activity().unwrap();
  }

  pub fn update_presence(&mut self, head_dict: &HeadDict, details: String, state: String, time_left: f64) {
    if details == *"" {
      self.client.set_activity(|a| {
        a.assets(|ass| {
          ass.small_image(&head_dict.media_server_name.to_lowercase())
        })
        .timestamps(|time| {
          time.end(time_left.round() as u64)
        })
        .state(&state)
      }).unwrap();
    } else {
      self.client.set_activity(|a| {
        a.assets(|ass| {
          ass.small_image(&head_dict.media_server_name.to_lowercase())
        })
        .timestamps(|time| {
          time.end(time_left.round() as u64)
        })
        .details(&details)
        .state(&state)
      }).unwrap();
    }
  }

  pub fn pause(&mut self, head_dict: &HeadDict, details: String, state: String) {
    if details == *"" {
      self.client
        .set_activity(|a| {
          a.assets(|ass| {
            ass.large_image(&head_dict.media_server_name.to_lowercase())
            .small_image("pause2")
          })
        .details(&state)
      }).ok();
    } else {
      self.client
        .set_activity(|a| {
          a.assets(|ass| {
            ass.large_image(&head_dict.media_server_name.to_lowercase())
            .small_image("pause2")
          })
        .details(&details)
        .state(&state)
      }).ok();
    }
  }
}
