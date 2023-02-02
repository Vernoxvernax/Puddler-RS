use discord_rich_presence::{DiscordIpc, activity};
use crate::mediaserver_information;
use mediaserver_information::HeadDict;


pub struct DiscordClient {
    pub client: discord_rich_presence::DiscordIpcClient,
    pub connection: bool,
}


pub fn mpv_link(use_discord: bool) -> DiscordClient {
    if use_discord {
        DiscordClient::default()
    } else {
        DiscordClient {
            client: DiscordClient::default().client,
            connection: false
        }
    }
}


impl Default for DiscordClient {
    fn default() -> Self {
        let mut client = discord_rich_presence::DiscordIpcClient::new("980093587314343957").unwrap();
        let connection: bool = client.connect().is_ok();
        Self {
            client,
            connection,
        }
    }
}


impl DiscordClient {
    pub fn update_presence(&mut self, head_dict: &HeadDict, details: String, state: String, time_left: f64) {
        if !self.connection {
            self.connection = self.client.connect().is_ok();
        }
        if self.connection {
            if details == *"" {
                self.client.set_activity(activity::Activity::new()
                                       .assets(activity::Assets::new().small_image(&head_dict.media_server_name.to_lowercase()))
                                       .timestamps(activity::Timestamps::new().end(time_left.round() as i64))
                                       .state(&state)
                                    ).ok();
            } else {
                self.client.set_activity(activity::Activity::new()
                                       .assets(activity::Assets::new().small_image(&head_dict.media_server_name.to_lowercase()))
                                       .timestamps(activity::Timestamps::new().end(time_left.round() as i64))
                                       .details(&details)
                                       .state(&state)
                                    ).ok();
            }
        }
    }
    pub fn pause(&mut self, head_dict: &HeadDict, details: String, state: String) {
        if !self.connection {
            self.connection = self.client.connect().is_ok();
        }
        if self.connection {
            if details == *"" {
                self.client
                    .set_activity(
                        activity::Activity::new()
                            .assets(
                                activity::Assets::new()
                                    .large_image(&head_dict.media_server_name.to_lowercase())
                                    .small_image("pause2")
                                )
                            .details(&state)
                    ).ok();
            } else {
                self.client
                    .set_activity(
                        activity::Activity::new()
                            .assets(
                                activity::Assets::new()
                                    .large_image(&head_dict.media_server_name.to_lowercase())
                                    .small_image("pause2")
                                )
                            .details(&details)
                            .state(&state)
                    ).ok();
            }
        }
    }
}
