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
use crate::is_numeric;
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
	StartTimeTicks: i64,
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
	fn select_ind(tracks: Vec<MediaStream>, kind: &str) -> usize {
		match tracks.len() {
			n if n > 1 => {
				tracks[Select::with_theme(&ColorfulTheme::default())
				.with_prompt("Please select which ".to_owned() + kind + " track you want to use:")
				.default(0)
				.items(&tracks[..])
				.interact()
				.unwrap()].Index
			},
			1 => {
				println!("The following {} track will be used:\n{}", kind, tracks.first().unwrap().to_string().green());
				tracks[0].Index
			},
			_ => {
				println!("This file doesn't have any {kind} track.");
				0
			}
		}
	}
	let mut subtitle_tracks: Vec<MediaStream> = [].to_vec();
	let mut audio_tracks: Vec<MediaStream> = [].to_vec();
	let mediaStreams: &Vec<MediaStream> = &item.MediaSources.as_ref().unwrap().first().unwrap().MediaStreams;
	for track in mediaStreams.iter() {
		match &track.Type as &str {
			"Audio" => audio_tracks.append(&mut [track.clone()].to_vec()),
			"Subtitle" => subtitle_tracks.append(&mut [track.clone()].to_vec()),
			_ => ()
		}
	};
	println!();
	(select_ind(audio_tracks, "audio"), select_ind(subtitle_tracks, "subtitle"))
}


pub fn play(settings: &Settings, head_dict: &HeadDict, Item: &Items) {
	let item: &mut Items = &mut Item.clone();
	item.UserData.PlaybackPositionTicks = {
		if item.UserData.PlaybackPositionTicks == 0 && ! settings.transcoding {
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
			print!("Do you want to continue at time: {formated}?\n  (Y)es | (N)o (start from a different position)");
			match getch("YyNnOo") {
				'N' | 'n' => {
					print!("Please enter a playback position in minutes: ");
					let mut input: String;
					loop {
						input = String::new();
						io::stdout().flush().expect("Failed to flush stdout");
						io::stdin().read_line(&mut input).unwrap();
						if input.trim().parse::<f64>().is_err() {
							print!("\nInvalid input, please try again.\n: ");
						} else if input.contains('.') {
							if input.split('.').collect::<Vec<&str>>().get(1).unwrap().len() > 8 {
								print!("\nInvalid input, please lower the amount of decimal places.\n: ");
							} else {
								break
							}
						} else {
							break
						}
					}
					(input.trim().parse::<f64>().unwrap() * 60.0 * 10000000.0).to_string().parse::<i64>().unwrap()
				},
				_ => {
					item.UserData.PlaybackPositionTicks
				}
			}

		}
	};
	
	let playback_info: PlaybackInfo = if settings.transcoding {
		let (audioIndex, subIndex) = choose_trackIndexx(item);
		
		print!("\nPlease enter your connection speed in mbps: ");
		let mut mbps: String = String::new();
		loop {
			io::stdout().flush().expect("Failed to flush stdout");
			io::stdin().read_line(&mut mbps).unwrap();
			if ! is_numeric(mbps.trim()) {
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
					},
					TranscodingProfile {
						Type: "Video".to_string(),
						Container: "mkv".to_string(),
						VideoCodec: "avc".to_string(),
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
		let playback_info_res: Result<http::Response<isahc::Body>, String> = post_puddler(format!("{}{}/Items/{}/PlaybackInfo?UserId={}", head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.config_file.user_id), &head_dict.auth_header, serde_json::to_string_pretty(&sess).unwrap());
		let playback_info: PlaybackInfo = match playback_info_res {
			Ok(mut t) => {
				let search_text: &String = &t.text().unwrap();
				serde_json::from_str(search_text).unwrap()
			}
			Err(e) => panic!("failed to parse get playback info: {e}")
		};
		playback_info
	} else {
		let playback_info_res: Result<http::Response<isahc::Body>, isahc::Error> = puddler_get(format!("{}{}/Items/{}/PlaybackInfo?UserId={}", head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.config_file.user_id), head_dict);
		let playback_info: PlaybackInfo = match playback_info_res {
			Ok(mut t) => {
				let search_text: &String = &t.text().unwrap();
				serde_json::from_str(search_text).unwrap()
			}
			Err(e) => panic!("failed to parse get playback info: {e}")
		};
		playback_info
	};

	started_playing(settings, head_dict, item, &playback_info);

	let resume_progress = item.UserData.PlaybackPositionTicks / 10000000;

	let total_runtime: f64 = if settings.transcoding {
		(item.RunTimeTicks.unwrap() as f64 - item.UserData.PlaybackPositionTicks as f64) / 10000000.0
	} else {
		item.RunTimeTicks.unwrap() as f64 / 10000000.0
	};

	let mut mpv_handle: mpv::MpvHandlerBuilder = mpv::MpvHandlerBuilder::new().expect("Failed to create MPV builder.");
	mpv_handle.set_option("osc", true).unwrap();
	mpv_handle.set_option("input-default-bindings", true).unwrap();
	mpv_handle.set_option("input-vo-keyboard", true).unwrap();

	let mut mpv: MpvHandler = mpv_handle.build().expect("Failed to create specified mpv configuration.");
	
	let stream_url: String = if settings.transcoding {
		format!("{}{}{}", head_dict.config_file.ipaddress, head_dict.media_server, playback_info.MediaSources.get(0).unwrap().TranscodingUrl.as_ref().unwrap())
	} else {
		format!("{}{}/Videos/{}/stream?Container=mkv&Static=true&api_key={}",
    head_dict.config_file.ipaddress, head_dict.media_server, item.Id, head_dict.request_header.token)
	};
	
	if settings.fullscreen {
		mpv.set_property("fullscreen", "yes").expect("Failed to configure fullscreen.");
	}

  if settings.gpu {
    mpv.set_property("hwdec", "auto-safe").expect("Failed to configure hardware-decoding.")
  }
	
	mpv.set_property("user-agent", APPNAME).expect("Failed to configure user-agent.");
	
	if item.Type == "Movie" {
		mpv.set_property("force-media-title", format!("{} ({}) | {}", item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], head_dict.media_server_name).as_str()).expect("Failed to configure force-media-title.");
		mpv.set_property("title", format!("{} - Streaming: {} ({})", APPNAME, item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]).as_str()).expect("Failed to configure title.");
	} else {
		mpv.set_property("force-media-title", format!("{} ({}) - {} - {} | {}", item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], item.SeasonName.as_ref().unwrap(), item.Name, head_dict.media_server_name).as_str()).expect("Failed to configure force-media-title.");
		mpv.set_property("title", format!("{} - Streaming: {} ({}) - {} - {}", APPNAME, item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4], item.SeasonName.as_ref().unwrap(), item.Name).as_str()).expect("Failed to configure title.");
	}

	mpv.command(&["loadfile", &stream_url as &str]).expect("Failed to stream the file :/");

	let mut discord: DiscordClient = discord::mpv_link(settings.discord_presence);
	let mut old_pos: f64 = -15.0;
	let mut last_time_update: f64 = 0.0;
	'main: loop {
		while let Some(event) = mpv.wait_event(0.0) {
			match event {
				mpv::Event::FileLoaded => {
					if resume_progress != 0 && ! settings.transcoding {
						mpv.command(&["seek", format!("{}", &resume_progress).as_str()]).expect("Failed to seek");
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
				_ => {
					// println!("{:#?}", event); // for debugging
				}
			};
		}
		let result: Result<f64, mpv::Error> = mpv.get_property("time-pos");
		if let Ok(nice) = result {
			if nice > old_pos + 15.0 { // this was the most retarded solution, I could think of
				update_progress(settings, head_dict, item, nice * 10000000.0, false, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id);
				if settings.discord_presence {
					if item.Type == "Movie" {
						DiscordClient::update_presence(&mut discord, head_dict,
							"".to_string(),
							format!("{} ({})", &item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
							SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - nice,
						);
					} else {
						DiscordClient::update_presence(&mut discord, head_dict,
							format!("{} ({})", &item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
							format!("{} ({})", item.Name, item.SeasonName.as_ref().unwrap()),
							SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64() + total_runtime - nice,
						);
					}
				}
				old_pos = nice;
			} else if nice == last_time_update {
				update_progress(settings, head_dict, item, nice * 10000000.0, true, &playback_info.PlaySessionId, &playback_info.MediaSources[0].Id);
				if settings.discord_presence {
					if item.Type == "Movie" {
						DiscordClient::pause(&mut discord, head_dict,
							"".to_string(),
							format!("{} ({})", &item.Name, &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
						);
					} else {
						DiscordClient::pause(&mut discord, head_dict,
							format!("{} ({})", &item.SeriesName.as_ref().unwrap(), &item.PremiereDate.as_ref().unwrap_or(&"????".to_string())[0..4]),
							format!("{} ({})", item.Name, item.SeasonName.as_ref().unwrap()),
						);
					}
				}
			}
			last_time_update = nice;
		}
		thread::sleep(time::Duration::from_millis(500));
	}
}
