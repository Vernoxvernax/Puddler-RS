
## **0.5.8**


* new option to resume from manually specified minute (if transcoding is enabled)
* recreated `mpv.lib` to link against new features
* fixed playback report when continuing stream with transcoding turned on
* fixed next episode option after watching a nested special (below)
* changed hotkey for next episode to 'n' instead of 'c'
* updated readme for those who installed visual studio 2022
* fixed a few printing mistakes

___

### First important decision:

Imagine this situation:

- Item A [Played]
- Item B [Played]
- Item C

What should happen if the user specificly rewatches and finishes Item A.
Should Item B be recommened for continuation, or Item C?

This had me thinking for a while, but in the end I decided for the latter (recommending Item C).
: Items that have already been finished, won't be recommended anymore. (unless you change the status in the emby interface of course)

___

### Second important decision:

Imagine the following:

Specials:
- Sp 1
- Sp 2
Season 1:
- Ep 1
- *Sp 1*
- Ep 2

`Sp 1` has been added to season 1 since its content extends story events from `Ep 1` (done by your database).


What is supposed to happen if the user plays & finishes `Sp 1`?
Should the player continue with `Sp 2` or with `Ep 2`?

Once again a pretty hard question. I decided for the latter (`Ep 2`).

___
___

## **0.5.7**
+ video transcode option (by entering mbps when starting a stream)
    + this requires you to choose one audio and one subtitle stream since emby/embyopensource don't support copying multiple of these
    + Hardware encoding recommended but not necessary | The media server will automatically scale the video stream based on the mbps input (server-side)
+ default config file does not need to be recreated if missing keys have been identified
+ config folders are now automatically created if not existent
+ fixed bug on windows for pressing enter in the menu
+ fixed regex to include windows's stupid backslashes
+ finally removed irrelevant "using libmpv" message
+ rewrote a few things
+ killed several small bugs
+ ran clippy

___

## **0.5.6-1**
+ fixed recently introduced bugs
+ 0.5.6 advertised features are now fully working
+ autologin always chooses the first entry (unless default server configuration is set)
+ discovered `git` source option for cargo.toml => you don't need to clone mpv-rs seperately anymore

___

## **0.5.6**

+ finally, full multi-server + multi-user support (config files have to be recreated)
+ added autologin feature since why not
+ updated all dependencies
+ move a little bit of code to the new `config.rs` where it belongs
+ password input will not be shown in the command line anymore

___

## Note:
+ Below is to see what changed since I abandoned the python version.
+ bye

## **0.5.5**

+ ran `cargo clippy --fix`
+ fixed series play (doesn't crash after finishing last episode)
+ decided to push future commits to github since app is kind useable rn

___

## **0.5.4**

+ added main menu which leads to a settings file featuring default values:
    + an emby/jellyfin config can be set as default, skipping the selection on startup
    + discord presence default state (on/off)
    + wether mpv should start in fullscreen
    + all options can be configured within the script itself
+ fixed major problem with the jellyfin api -> NextUp items are finally printed into the menu
+ "ALL" search term will work if there are Items missing the `RunTimeTicks` value.
+ script will now print the time you've ended playback (if not finished)
+ greatly improved series printing (similar to the great `tree` command)
+ premiere year dates will now print next to every item's name -> "Doctor Strange in the Multiverse of Madness **(2022)**"
+ fixed compiling on windows itself
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

~~*Still too lazy to do anything for windows users.*~~

More fixes to come.

___
