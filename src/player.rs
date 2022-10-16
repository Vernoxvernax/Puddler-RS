extern crate mpv;
use std::io;
use std::io::prelude::*;
use colored::Colorize;
use mpv::MpvHandler;
use serde_derive::{Deserialize};
use serde::Serialize;
use isahc::ReadResponseExt;
use crate::getch;
use crate::discord;
use crate::discord::DiscordClient;
use crate::APPNAME;
use crate::Items;
use crate::mediaserver_information::HeadDict;
use crate::mediaserver_information::post_puddler;
use crate::progress_report::MediaStream;
use crate::puddler_get;
use crate::numbers;
use crate::settings::Settings;
use crate::progress_report::PlaybackInfo;
use crate::progress_report::finished_playback;
use crate::progress_report::update_progress;
use crate::progress_report::started_playing;
use std::{thread, time};
use std::time::SystemTime;
use dialoguer::{theme::ColorfulTheme, Select};


#[derive(Debug, Serialize, Deserialize)]
struct SessionCapabilities {
    UserId: String,
    StartTimeTicks: u64,
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


fn choose_trackIndexx(item: &Items) -> (usize, usize) {
    let mut subtitle_tracks: Vec<MediaStream> = [].to_vec();
    let mut audio_tracks: Vec<MediaStream> = [].to_vec();
    let mediaStreams: &Vec<MediaStream> = &item.MediaSources.as_ref().unwrap().first().unwrap().MediaStreams;
    for track in mediaStreams.into_iter() {
        match &track.Type as &str {
            "Audio" => (
                audio_tracks.append(&mut [track.clone()].to_vec())
            ),
            "Subtitle" => (
                subtitle_tracks.append(&mut [track.clone()].to_vec())
            ),
            _ => ()
        }
    };
    println!("");
    let audioIndex = if audio_tracks.len() > 1 {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Please select which audio track you want to use:")
            .default(0)
            .items(&audio_tracks[..])
            .interact()
            .unwrap()
    } else {
        println!("The following audio track will be used:\n{}", audio_tracks.first().unwrap().to_string().green());
        0
    };
    println!("");
    let subIndex = if subtitle_tracks.len() > 1 {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Please select which subtitle track you want to use:")
            .default(0)
            .items(&subtitle_tracks[..])
            .interact()
            .unwrap()
    } else if subtitle_tracks.len() == 1 {
        println!("The following subtitle track will be used:\n{}", subtitle_tracks.first().unwrap().to_string().green());
        0
    } else {
        println!("This file doesn't have any subtitles.");
        0
    };
    println!("");
    (audio_tracks[audioIndex].Index, subtitle_tracks[subIndex].Index)
}


pub fn play(settings: &Settings, head_dict: &HeadDict, Item: &Items) {
    let item: &mut Items = &mut Item.clone();
    item.UserData.PlaybackPositionTicks = {
        if item.UserData.PlaybackPositionTicks == 0 || ! settings.transcoding {
            0
        } else {
            let time = (item.UserData.PlaybackPositionTicks as f64) / 10000000.0;
            let formated: String = if time > 60.0 {
                if (time / 60.0) > 60.0 {
                    format!("{:02}:{:02}:{:02}", ((time / 60.0) / 60.0).trunc(), ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                } else {
                    format!("00:{:02}:{:02}", (time / 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                }
            } else {
                time.to_string()
            };
            print!("Do you want to continue at time: {}?\n  (Y)es | (N)o (from the beginning) | (O)ther position", formated);
            match getch("YyNnOo") {
                'N' | 'n' => {
                    0
                },
                'O' | 'o' => {
                    print!("Please enter a playback position in minutes: ");
                    io::stdout().flush().expect("Failed to flush stdout");
                    let mut input = String::new();
                    loop {
                        io::stdin().read_line(&mut input).unwrap();
                        if input.trim().parse::<u64>().is_err() {
                            print!("\nInvalid input, please try again.\n: ");
                        } else {
                            break
                        }
                    }
                    input.trim().parse::<u64>().unwrap() * 60 * 10000000
                },
                _ => {
                    item.UserData.PlaybackPositionTicks
                }
            }

        }
    };
    let playback_info: PlaybackInfo = if settings.transcoding {
        let (audioIndex, subIndex) = choose_trackIndexx(item);
        print!("Please enter your internet speed in mbps: ");
        let mut mbps: String = String::new();
        loop {
            io::stdout().flush().expect("Failed to flush stdout");
            io::stdin().read_line(&mut mbps).unwrap();
            if ! numbers(&mbps.trim().to_string()) {
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
        let playback_info_res: Result<http::Response<isahc::Body>, isahc::Error> = post_puddler(format!("{}{}/Items/{}/PlaybackInfo?UserId={}", head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.config_file.user_id), &head_dict.auth_header, serde_json::to_string_pretty(&sess).unwrap());
        let playback_info: PlaybackInfo = match playback_info_res {
            Ok(mut t) => {
                let search_text: &String = &t.text().unwrap();
                serde_json::from_str(search_text).unwrap()
            }
            Err(e) => panic!("failed to parse get playback info: {}", e)
        };
        playback_info
    } else {
        let playback_info_res: Result<http::Response<isahc::Body>, isahc::Error> = puddler_get(format!("{}{}/Items/{}/PlaybackInfo?UserId={}", head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.config_file.user_id), &head_dict);
        let playback_info: PlaybackInfo = match playback_info_res {
            Ok(mut t) => {
                let search_text: &String = &t.text().unwrap();
                serde_json::from_str(search_text).unwrap()
            }
            Err(e) => panic!("failed to parse get playback info: {}", e)
        };
        playback_info
    };
    started_playing(settings, head_dict, item, &playback_info);
    let resume_progress = item.UserData.PlaybackPositionTicks / 10000000;
    let mut mpv_handle: mpv::MpvHandlerBuilder = mpv::MpvHandlerBuilder::new().expect("Couldn't create MPV builder.");
    mpv_handle.set_option("osc", true).unwrap();
    mpv_handle.set_option("input-default-bindings", true).unwrap();
    mpv_handle.set_option("input-vo-keyboard", true).unwrap();
    let mut mpv: MpvHandler = mpv_handle.build().expect("Failed to create specified mpv configuration.");
    let stream_url: String = if settings.transcoding {
        format!("{}{}{}", head_dict.config_file.ipaddress, head_dict.media_server, playback_info.MediaSources.iter().nth(0).unwrap().TranscodingUrl.as_ref().unwrap())
    } else {
        format!("{}{}/Videos/{}/stream?Container=mkv&Static=true&api_key={}", head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.request_header.token)
    };
    if settings.fullscreen {
        mpv.set_property("fullscreen", "yes").expect("Couldn't configure fullscreen.");
    }
    mpv.set_property("title", format!("{} - Streaming: {} ({})", APPNAME, item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]).as_str()).expect("Couldn't configure title.");
    mpv.set_property("user-agent", APPNAME).expect("Couldn't configure user-agent.");
    if item.Type == "Movie" {
        mpv.set_property("force-media-title", format!("{} ({}) | {}", item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], head_dict.media_server_name).as_str()).expect("Couldn't configure force-media-title.");
    } else {
        mpv.set_property("force-media-title", format!("{} ({}) - {} - {} | {}", item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], item.SeasonName.as_ref().unwrap(), item.Name, head_dict.media_server_name).as_str()).expect("Couldn't configure force-media-title.");
    }
    mpv.command(&["loadfile", &stream_url as &str]).expect("Failed to stream the file :/");
    let mut discord: DiscordClient = discord::mpv_link(settings.discord_presence);
    let mut old_pos: f64 = -15.0;
    let mut paused: bool = false;
    let total_runtime: f64 = item.RunTimeTicks.unwrap() as f64 / 10000000.0;
    let mut pause_update_counter: u8 = 0; // Since the mpv api for some reason sends two "Pause" events at once
    'main: loop {
        while let Some(event) = mpv.wait_event(0.0) {
            match event {
                mpv::Event::FileLoaded => {
                    if resume_progress != 0 && ! settings.transcoding {
                        mpv.command(&["seek", format!("{}", &resume_progress).as_str()]).expect("Failed to seek")
                    }
                }
                mpv::Event::Shutdown => {
                    finished_playback(settings, head_dict, item, old_pos * 10000000.0, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id, false);
                    break 'main;
                }
                mpv::Event::EndFile(_t) => {
                    finished_playback(settings, head_dict, item, old_pos * 10000000.0, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id, true);
                    break 'main;
                }
                mpv::Event::Seek | mpv::Event::PlaybackRestart => {
                    old_pos -= 16.0
                }
                mpv::Event::Pause => {
                    paused = true;
                    if pause_update_counter == 0 {
                        let result: Result<f64, mpv::Error> = mpv.get_property("time-pos");
                        match result {
                            Ok(nice) => {
                                    update_progress(settings, head_dict, item, nice * 10000000.0, paused, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id);
                                    old_pos = nice;
                                    if settings.discord_presence {
                                        if item.Type == "Movie" {
                                            DiscordClient::pause(&mut discord, head_dict,
                                                "".to_string(),
                                                format!("Streaming: {} ({})", &item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
                                                nice,
                                            );
                                        } else {
                                            DiscordClient::pause(&mut discord, head_dict,
                                                format!("Streaming: {} ({})", &item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
                                                format!("{} ({})", item.Name, item.SeasonName.as_ref().unwrap()),
                                                nice,
                                            );
                                        }
                                    }
                            }
                            _ => ()
                        }
                        pause_update_counter = 1
                    } else {
                        pause_update_counter = 0
                    }
                }
                mpv::Event::Unpause => {
                    paused = false;
                    if pause_update_counter == 0 {
                        let result: Result<f64, mpv::Error> = mpv.get_property("time-pos");
                        match result {
                            Ok(nice) => {
                                update_progress(settings, head_dict, item, nice * 10000000.0, paused, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id);
                                if settings.discord_presence && ! paused {
                                    if item.Type == "Movie" {
                                        DiscordClient::update_presence(&mut discord, head_dict,
                                            "".to_string(),
                                            format!("Streaming: {} ({})", &item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
                                            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - nice,
                                        );
                                    } else {
                                        DiscordClient::update_presence(&mut discord, head_dict,
                                            format!("Streaming: {} ({})", &item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
                                            format!("{} ({})", item.Name, item.SeasonName.as_ref().unwrap()),
                                            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - nice,
                                        );
                                    }
                                }
                                old_pos = nice;
                            }
                            _ => ()
                        }
                        pause_update_counter = 1
                    } else {
                        pause_update_counter = 0
                    }
                }
                _ => {
                    // println!("{:#?}", event); // for debugging
                }
            };
        }
        let result: Result<f64, mpv::Error> = mpv.get_property("time-pos");
        match result {
            Ok(nice) => {
                if nice > old_pos + 15.0 { // this was the most retarded solution, I could think of
                    update_progress(settings, head_dict, item, nice * 10000000.0, paused, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id);
                    if settings.discord_presence && ! paused {
                        if item.Type == "Movie" {
                            DiscordClient::update_presence(&mut discord, head_dict,
                                "".to_string(),
                                format!("Streaming: {} ({})", &item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
                                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - nice,
                            );
                        } else {
                            DiscordClient::update_presence(&mut discord, head_dict,
                                format!("Streaming: {} ({})", &item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
                                format!("{} ({})", item.Name, item.SeasonName.as_ref().unwrap()),
                                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - nice,
                            );
                        }
                    }
                    old_pos = nice;
                }
            }
            _ => ()
        }
        thread::sleep(time::Duration::from_millis(500));
    }
    // Here, you can add more commands on what should happen after playback.
}
