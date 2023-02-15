use std::fmt;
use colored::Colorize;
use serde::Serialize;
use isahc::Request;
use isahc::prelude::*;
use crate::mediaserver_information::AuthHeader;
use crate::settings::Settings;
use serde_derive::{Deserialize};
extern crate mpv;
use crate::{
    HeadDict,
    Items
};


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct PlaybackObject {
    canseek: bool,
    itemid: String,
    playsessionid: String,
    mediasourceid: String,
    ispaused: bool,
    positionticks: String,
    playmethod: String,
    repeastmode: String,
    eventname: String
}


#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct PlayingObject {
    itemid: String,
    playsessionid: String,
    sessionid: String,
    mediasourceid: String,
    ispaused: bool,
    ismuted: bool,
    playbackstarttimeticks: String,
    playmethod: String,
    repeatmode: String
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct PlaybackInfo {
    pub MediaSources: Vec<MediaSourceInfo>,
    pub PlaySessionId: String
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
    pub IsDefault: bool
}


impl fmt::Display for MediaStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.IsDefault {
            write!(f, "Title = \"{}\", Language = \"{}\", Codec = \"{}\" {}", self.Title.as_ref().unwrap_or(self.DisplayTitle.as_ref().unwrap_or(&"".to_string())), self.DisplayLanguage.as_ref().unwrap_or(self.Language.as_ref().unwrap_or(&"undefined".to_string())), self.Codec.as_ref().unwrap_or(&"???".to_string()).to_uppercase(), "[Default]".to_string().green())
        } else {
            write!(f, "Title = \"{}\", Language = \"{}\", Codec = \"{}\"", self.Title.as_ref().unwrap_or(self.DisplayTitle.as_ref().unwrap_or(&"".to_string())), self.DisplayLanguage.as_ref().unwrap_or(self.Language.as_ref().unwrap_or(&"undefined".to_string())), self.Codec.as_ref().unwrap_or(&"???".to_string()).to_uppercase())
        }
    }
}


pub fn started_playing(settings: &Settings, head_dict: &HeadDict, item: &Items, playback_info: &PlaybackInfo) {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let item_id: &String = &item.Id;
    let session_id: &String = &head_dict.session_id;
    let media_server: &String = &head_dict.media_server;
    let media_server_name: &String = &head_dict.media_server_name;
    let playmethod = if settings.transcoding {
        "Transcode".to_string()
    } else {
        "DirectPlay".to_string()
    };
    let playing_object = PlayingObject {
        itemid: item_id.to_string(),
        playsessionid: playback_info.PlaySessionId.to_string(),
        sessionid: session_id.to_string(),
        mediasourceid: playback_info.MediaSources[0].Id.to_string(),
        ispaused: false,
        ismuted: false,
        playbackstarttimeticks: item.UserData.PlaybackPositionTicks.to_string(),
        playmethod,
        repeatmode: "RepeatNone".to_string()
    };
    let post_res = no_res_post(format!("{ipaddress}{media_server}/Sessions/Playing?format=json"), &head_dict.auth_header, serde_json::to_string_pretty(&playing_object).unwrap());
    if let Err(error) = post_res {
        println!("Couldn't start playing session on {media_server_name}. Error: {error}");
    }
}


pub fn update_progress(settings: &Settings, head_dict: &HeadDict, item: &Items, mut time_pos: f64, paused: bool, playsession_id: &String, mediasource_id: &String) {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let item_id: &String = &item.Id;
    let media_server: &String = &head_dict.media_server;
    let media_server_name: &String = &head_dict.media_server_name;
    let event_name: String = if paused {
        "Pause".to_string()
    } else {
        "TimeUpdate".to_string()
    };
    let playmethod: String;
    (playmethod, time_pos) = if settings.transcoding {
        ("Transcode".to_string(), time_pos + item.UserData.PlaybackPositionTicks as f64)
    } else {
        ("DirectPlay".to_string(), time_pos)
    };
    let update_obj = PlaybackObject {
        canseek: true,
        itemid: item_id.to_string(),
        playsessionid: playsession_id.to_string(),
        mediasourceid: mediasource_id.to_string(),
        ispaused: paused,
        positionticks: time_pos.round().to_string(),
        playmethod,
        repeastmode: "RepeatNone".to_string(),
        eventname: event_name
    };
    let result = no_res_post(format!("{ipaddress}{media_server}/Sessions/Playing/Progress"), &head_dict.auth_header, serde_json::to_string_pretty(&update_obj).unwrap());
    if let Err(error) = result {
        println!("Couldn't send playback update to {media_server_name}. Error: {error}")
    }
}


pub fn no_res_post (url: String, auth_header: &AuthHeader, bod: String) -> Result<(), isahc::Error> {
    Request::post(url)
        .header("Authorization", &auth_header.authorization)
        .header("Content-Type", "application/json")
        .body(bod)?
        .send()?;
    Ok(())
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct FinishedObject {
    itemid: String,
    playsessionid: String,
    sessionid: String,
    mediasourceid: String,
    positionticks: String
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct NoProgressObject {
    itemid: String,
    playsessionid: String,
    sessionid: String,
    mediasourceid: String,
    positionticks: String
}


pub fn finished_playback(settings: &Settings, head_dict: &HeadDict, item: &Items, mut time_pos: f64, playsession_id: &String, mediasource_id: &String, eof: bool) {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let item_id: &String = &item.Id;
    let session_id: &String = &head_dict.session_id;
    let media_server: &String = &head_dict.media_server;
    let user_id: &String = &head_dict.config_file.user_id;
    if settings.transcoding {
        time_pos += item.UserData.PlaybackPositionTicks as f64
    };
    if ! eof {
        let result = no_res_post(format!("{ipaddress}{media_server}/Users/{user_id}/PlayedItems/{item_id}"), &head_dict.auth_header, "".to_string());
        match result {
            Ok(_) => {
                println!("Item has been marked as [PLAYED].")
            }
            Err(_) => {
                println!("Couldn't report item as [PLAYED].")
            }
        };
    } else {
        let difference = ((item.RunTimeTicks.unwrap() as f64) - time_pos) / (item.RunTimeTicks.unwrap() as f64);
        if difference < 0.10 {
            let result = no_res_post(format!("{ipaddress}{media_server}/Users/{user_id}/PlayedItems/{item_id}"), &head_dict.auth_header, "".to_string());
            match result {
                Ok(_) => {
                    println!("Since you've watched more than 90% of the video, it has been marked as [PLAYED].")
                }
                Err(_) => {
                    println!("Couldn't report item as [PLAYED].")
                }
            };
        } else if difference < 0.90 {
            let finished_obj = FinishedObject {
                itemid: item_id.to_string(),
                playsessionid: playsession_id.to_string(),
                sessionid: session_id.to_string(),
                mediasourceid: mediasource_id.to_string(),
                positionticks: time_pos.to_string()
            };
            let response = no_res_post(format!("{ipaddress}{media_server}/Sessions/Playing/Stopped"), &head_dict.auth_header, serde_json::to_string_pretty(&finished_obj).unwrap());
            match response {
                Ok(_) => {
                    let time = time_pos / 10000000.0;
                    let formated: String = if time > 60.0 {
                        if (time / 60.0) > 60.0 {
                            format!("{:02}:{:02}:{:02}", ((time / 60.0) / 60.0).trunc(), ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                        } else {
                            format!("00:{:02}:{:02}", (time / 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                        }
                    } else {
                        time.to_string()
                    };
                    println!("Playback progress ({formated}) has been sent to your server.")
                }
                Err(_) => {
                    println!("Playback progress couldn't be logged to your server.")
                }
            }
        } else {
            let finished_obj = NoProgressObject {
                itemid: item_id.to_string(),
                playsessionid: playsession_id.to_string(),
                sessionid: session_id.to_string(),
                mediasourceid: mediasource_id.to_string(),
                positionticks: (item.UserData.PlaybackPositionTicks as f64).to_string()
            };
            let response = no_res_post(format!("{ipaddress}{media_server}/Sessions/Playing/Stopped"), &head_dict.auth_header, serde_json::to_string_pretty(&finished_obj).unwrap());
            match response {
                Ok(_) => {
                    println!("Item has not been marked as [PLAYED].")
                }
                Err(_) => {
                    println!("Playback progress couldn't be logged to your server.")
                }
            }
        }
    }
}
