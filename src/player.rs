extern crate mpv;
use crate::discord;
use crate::discord::DiscordClient;
use mpv::MpvHandler;
use crate::APPNAME;
use crate::Items;
use crate::mediaserver_information::HeadDict;
use crate::settings::Settings;
use crate::progress_report::finished_playback;
use crate::progress_report::update_progress;
use crate::progress_report::started_playing;
use std::{thread, time};
use std::time::SystemTime;


pub fn play(stream_url: String, item: &Items, media_server_name: String, head_dict: &HeadDict, settings: &Settings) {
    let resume_progress = item.UserData.PlaybackPositionTicks / 10000000;
    println!("Using libmpv."); // for those not aware #flex
    let mut mpv_handle: mpv::MpvHandlerBuilder = mpv::MpvHandlerBuilder::new().expect("Couldn't create MPV builder.");
    mpv_handle.set_option("osc", true).unwrap();
    mpv_handle.set_option("input-default-bindings", true).unwrap();
    mpv_handle.set_option("input-vo-keyboard", true).unwrap();
    let mut mpv: MpvHandler = mpv_handle.build().expect("Failed to create specified mpv configuration.");
    if settings.fullscreen {
        mpv.set_property("fullscreen", "yes").expect("Couldn't configure fullscreen.");
    }
    mpv.set_property("title", format!("{} - Streaming: {} ({})", APPNAME, item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]).as_str()).expect("Couldn't configure title.");
    mpv.set_property("user-agent", APPNAME).expect("Couldn't configure user-agent.");
    if item.Type == "Movie" {
        mpv.set_property("force-media-title", format!("{} ({}) | {}", item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], media_server_name).as_str()).expect("Couldn't configure force-media-title.");
    } else {
        mpv.set_property("force-media-title", format!("{} ({}) - {} - {} | {}", item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], item.SeasonName.as_ref().unwrap(), item.Name, media_server_name).as_str()).expect("Couldn't configure force-media-title.");
    }
    mpv.command(&["loadfile", &stream_url as &str]).expect("Failed to stream the file :/");
    let mut discord: DiscordClient = discord::mpv_link(settings.discord_presence);
    let mut old_pos: f64 = -15.0;
    let mut paused: bool = false;
    let total_runtime: f64 = item.RunTimeTicks.unwrap() as f64 / 10000000.0;
    let mut pause_update_counter: u8 = 0; // Since the mpv api for some reason sends two "Pause" events at once
    let (&playsession_id, &mediasource_id);
    (playsession_id, mediasource_id) = started_playing(head_dict, item);
    'main: loop {
        while let Some(event) = mpv.wait_event(0.0) {
            match event {
                mpv::Event::FileLoaded => {
                    if resume_progress != 0 {
                        mpv.command(&["seek", format!("{}", &resume_progress).as_str()]).expect("Failed to seek")
                    }
                }
                mpv::Event::Shutdown => {
                    finished_playback(head_dict, item, old_pos * 10000000.0, &playsession_id, &mediasource_id, false);
                    break 'main;
                }
                mpv::Event::EndFile(_t) => {
                    finished_playback(head_dict, item, old_pos * 10000000.0, &playsession_id, &mediasource_id, true);
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
                                    update_progress(head_dict, item, nice * 10000000.0, paused, &playsession_id, &mediasource_id);
                                    old_pos = nice;
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
                                    update_progress(head_dict, item, nice * 10000000.0, paused, &playsession_id, &mediasource_id);
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
                    update_progress(head_dict, item, nice * 10000000.0, paused, &playsession_id, &mediasource_id);
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
