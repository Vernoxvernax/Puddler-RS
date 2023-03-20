## **0.5.11**

* Don't collapse BoxSets when issuing the "ALL" search term
* Reinstate the discord presence paused state (long overdue)
  * Mostly not that important, but the behavior of it changes a little:
  * Since the actual API-Call got removed from MPV's client API, the script will rely on whether the playback position has changed or not
* Discord-Presence: no more verbose information on the playback-state (like "Paused"). The script will now just remove the timer and put a small pause-icon-indicator on the bottom right of the application icon
* Updated `mpv.lib` just in case
* If you want features, changes, improvements, then please open an issue or contact me. I'm bored.
* Add autoplay option to continue after waiting 5 seconds (you can only exit these through CTRL+C, unless you've watched everything)
* Add hardware decoding option (`auto-safe`)
* Refactored code; removed pointless "Starting mpv..." message

## **0.5.10**

* reinstated Next-Up TV-Show for Jellyfin servers (don't ask me)
* activated the "do you want to continue at" question for all transcoding requests
* turned transcoding question input to float, so you can type something like `3.5` being `3min 30seconds`
* fixed window title in series mode

## **0.5.9**

* emby now prints series and movies in the latest section (limited to 10 entries each)
* fixed printing when a played episode/special is shown at "continue-watching" (yes this can happen)
* specials can now appear in the "play-next" prompt after finishing an episode
* discord presence actually turned off (paused state wasn't blocked)
* the codec field can apparently be undefined -> fallback to "???" added


## **0.5.8**

* new option to resume from manually specified minute (if transcoding is enabled)
* recreated `mpv.lib` to link against new features
* fixed playback report when continuing stream with transcoding turned on
* fixed next episode option after watching a nested special (below)
* changed hotkey for next episode to 'n' instead of 'c'
* updated README for those who installed Visual Studio 2022
* fixed a few printing mistakes

___

### First important decision:

Imagine this situation:

- Item A [Played]
- Item B [Played]
- Item C

What should happen if the user specifically rewatches and finishes Item A.
Should Item B be recommended for continuation, or Item C?

This had me thinking for a while, but in the end I decided for the latter (recommending Item C).
-> Items that have already been finished, won't be recommended anymore. (unless you change the status in the Emby interface of course)

___

### Second important decision:

Imagine the following:

Specials:
- SP 1
- SP 2
Season 1:
- EP 1
- *SP 1*
- EP 2

`SP 1` has been added to season 1 since its content extends story events from `EP 1` (done by your database).


What is supposed to happen if the user plays & finishes `SP 1`?
Should the player continue with `SP 2` or with `EP 2`?

Once again a pretty hard question. I decided for the latter (`EP 2`).

___
___

## **0.5.7**
+ video transcode option (by entering Mbps when starting a stream)
    + this requires you to choose one audio and one subtitle stream since Emby/Embyopensource don't support copying multiple of these.
    + Hardware encoding recommended but not necessary | The media server will automatically scale the video stream based on the Mbps input (server-side)
+ default config file does not need to be recreated if missing keys have been identified
+ config folders are now automatically created if not existent
+ fixed bug on Windows for pressing *enter* in the menu
+ fixed regex to include Windows's stupid backslashes
+ finally removed irrelevant "using libmpv" message
+ rewrote a few things
+ killed several small bugs
+ ran Clippy

___

## **0.5.6-1**
+ fixed recently introduced bugs
+ 0.5.6 advertised features are now fully working
+ autologin always chooses the first entry (unless default server configuration is set)
+ discovered `git` source option for `cargo.toml` -> you don't need to clone mpv-rs separately anymore

___

## **0.5.6**

+ finally, full multiserver + multi-user support (config files have to be recreated)
+ added autologin feature since why not
+ updated all dependencies
+ move a little of code to the new `config.rs` where it belongs
+ password input will not be shown in the command line anymore

___

## Note:
+ Below is to see what changed since I abandoned the python version.
+ Bye

## **0.5.5**

+ ran `cargo clippy --fix`
+ fixed series play (doesn't crash after finishing last episode)
+ decided to push future commits to GitHub since app is kind useable rn

___

## **0.5.4**

+ Added main menu which leads to a settings file featuring default values:
    + an Emby/Jellyfin config can be set as default, skipping the selection on startup
    + discord presence default state (on/off)
    + Whether MPV should start in full screen
    + all options can be configured within the script itself
+ fixed major problem with the Jellyfin API -> Next-Up items are finally printed into the menu
+ "ALL" search term will work if there are Items missing the `RunTimeTicks` value.
+ Script will now print the time you've ended playback (if not finished)
+ greatly improved series printing (similar to the great `tree` command)
+ premiere year dates will now print next to every item's name -> "Doctor Strange in the Multiverse of Madness **(2022)**"
+ fixed compiling on Windows itself
+ removed unused dependencies
+ fixed a few typos
+ removed useless comments
+ improved `install.sh`

___

## **0.5.3**

+ improved next/resume printing for episodes
+ changed "continue with next episode" dialogue
+ release site finally features `.exe` windows binaries (hopefully)
+ compiling instructions

___

## **0.5.2**

+ passwords are no longer stored in plain text (lol)
    + the script will ask you to re-enter your credentials when sessions have been flushed (server update, ...)
+ cleaned up *comments*
+ fixed bugs
+ added THIS file :)

~~*Still too lazy to do anything for Windows users.*~~

More fixes to come.

___
