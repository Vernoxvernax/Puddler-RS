use isahc::ReadResponseExt;
use serde::Serialize;
use isahc::Request;
use isahc::prelude::*;
use crate::mediaserver_information::AuthHeader;
use serde_derive::{Deserialize};
use crate::puddler_get;
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

#[derive(Debug, Deserialize, PartialEq, Clone)]
struct MediaInfo {
    MediaSources: Vec<PlayBackInfo>,
    PlaySessionId: String
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
struct PlayBackInfo {
    Id: String,
}


#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct PlayingObject {
    canseek: bool,
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


pub fn started_playing(head_dict: &HeadDict, item: &Items) -> (String, String) {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let item_id: &String = &item.Id;
    let session_id: &String = &head_dict.session_id;
    let media_server: &String = &head_dict.media_server;
    let media_server_name: &String = &head_dict.media_server_name;
    let user_id: &String = &head_dict.config_file.user_id;
    let playback_info_res = puddler_get(format!("{}{}/Items/{}/PlaybackInfo?UserId={}", ipaddress, media_server, item.Id, user_id), head_dict);
    let playback_info: MediaInfo = match playback_info_res {
        Ok(mut t) => {
            let search_text: &String = &t.text().unwrap();
            serde_json::from_str(search_text).unwrap()
        }
        Err(e) => panic!("failed to parse get request: {}", e)
    };
    let playback_id = &playback_info.MediaSources[0].Id;
    let playsession_id = &playback_info.PlaySessionId;
    let playing_object = PlayingObject {
        canseek: true,
        itemid: item_id.to_string(),
        playsessionid: playsession_id.to_string(),
        sessionid: session_id.to_string(),
        mediasourceid: playback_id.to_string(),
        ispaused: false,
        ismuted: false,
        playbackstarttimeticks: item.UserData.PlaybackPositionTicks.to_string(),
        playmethod: "DirectStream".to_string(),
        repeatmode: "RepeatNone".to_string()
    };
    let post_res = no_res_post(format!("{}{}/Sessions/Playing?format=json", ipaddress, media_server), &head_dict.auth_header, serde_json::to_string_pretty(&playing_object).unwrap());
    match post_res {
        Err(error) => {
            println!("Couldn't start playing session on {}. Error: {}", media_server_name, error)
        },
        _ => ()
    };
    (playback_info.PlaySessionId, playback_id.to_string())


}


pub fn update_progress(head_dict: &HeadDict, item: &Items, time_pos: f64, paused: bool, playsession_id: &String, mediasource_id: &String) {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let item_id: &String = &item.Id;
    let media_server: &String = &head_dict.media_server;
    let media_server_name: &String = &head_dict.media_server_name;
    let event_name: String = if paused {
        "Pause".to_string()
    } else {
        "TimeUpdate".to_string()
    };
    let update_obj = PlaybackObject {
        canseek: true,
        itemid: item_id.to_string(),
        playsessionid: playsession_id.to_string(),
        mediasourceid: mediasource_id.to_string(),
        ispaused: paused,
        positionticks: time_pos.round().to_string(),
        playmethod: "DirectStream".to_string(),
        repeastmode: "RepeatNone".to_string(),
        eventname: event_name
    };
    let result = no_res_post(format!("{}{}/Sessions/Playing/Progress", ipaddress, media_server), &head_dict.auth_header, serde_json::to_string_pretty(&update_obj).unwrap());
    match result {
        Err(error) => {
            println!("Couldn't send playback update to {}. Error: {}", media_server_name, error)
        },
        _ => {
        }
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


pub fn finished_playback(head_dict: &HeadDict, item: &Items, time_pos: f64, playsession_id: &String, mediasource_id: &String, eof: bool) {
    let ipaddress: &String = &head_dict.config_file.ipaddress;
    let item_id: &String = &item.Id;
    let session_id: &String = &head_dict.session_id;
    let media_server: &String = &head_dict.media_server;
    // let media_server_name: &String = &head_dict.media_server_name;
    let user_id: &String = &head_dict.config_file.user_id;
    if ! eof {
        let result = no_res_post(format!("{}{}/Users/{}/PlayedItems/{}", ipaddress, media_server, user_id, item_id), &head_dict.auth_header, "".to_string());
        match result {
            Ok(_) => {
                println!("Item has been marked as [PLAYED].")
            }
            _ => {
                println!("Couldn't report item as [PLAYED].")
            }
        };
    } else {
        let difference = ((item.RunTimeTicks.unwrap() as f64) - time_pos) / (item.RunTimeTicks.unwrap() as f64);
        if difference < 0.10 {
            let result = no_res_post(format!("{}{}/Users/{}/PlayedItems/{}", ipaddress, media_server, user_id, item_id), &head_dict.auth_header, "".to_string());
            match result {
                Ok(_) => {
                    println!("Since you've watched more than 90% of the video, it has been marked as [PLAYED].")
                }
                _ => {
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
            let response = no_res_post(format!("{}{}/Sessions/Playing/Stopped", ipaddress, media_server), &head_dict.auth_header, serde_json::to_string_pretty(&finished_obj).unwrap());
            match response {
                Ok(_) => {
                    let time = time_pos as f64 / 10000000.0;
                    let formated: String = if time > 60.0 {
                        if (time / 60.0) > 60.0 {
                            format!("{:02}:{:02}:{:02}", ((time / 60.0) / 60.0).trunc(), ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                        } else {
                            format!("00:{:02}:{:02}", (time / 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                        }
                    } else {
                        time.to_string()
                    };
                    println!("Playback progress ({}) has been sent to your server.", formated)
                }
                _ => {
                    println!("Playback progress couldn't be logged to your server.")
                }
            }
        } else {
            println!("Item has not been marked as [PLAYED].")
        }
    }
}
