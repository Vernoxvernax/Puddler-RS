## **BREAKING CHANGE:**

... yeah well, cause everything has been rewritten.

Just documenting all changes of this commit would take me days and even then I'd forget about certain things.
Well here is a short summary of the most notable ones:

* ~~Cleaner Code~~
* Support for Plex Streaming
* Emby/Jellyfin: Support for simple Pause and Stop commands (comming from emby app/web)
* Actually usable interface
  * Pretty much every input is handled through `crossterm` with some neat eyecandy such as colors and highlights with bold
  * And of course, almost all of the lines printed by puddler will be deleted afterwards.
    * While using puddler myself, I often got a little annoying at how verbose everything was.
    * Even though I knew exactly what item to play, just pressing enter, I ended up with a couple dozent of unnecessary lines at the end of my viewing experience.
    * Well not anymore.
  * Note that all menus are created and managed through `crossterm` and they are pretty untested as of right now.
* The config files are now also more cleaner. (I hope)

Additional Information:
* Authentication for plex has only been tested with an admin account. I don't have plex pass and am not planning to get one.

### **Please create an issue if you find anything that seems weird, bug or not.**

## **0.6.1**
* Removed getch crate and rewrote it's functionality using the awesome crossterm crate with additional timeout features.
* Fixed issue where episodes would be skipped if they would already be marked as played.
* Fixed issue where episodes would be skipped when using the `f` option in the quick-menu.
* Increase some request timeouts to 10 seconds.
* Avoid discord problems.
* Some code refactoring.
* Bump dependencies.

## **0.6.0**
* **BREAKING CHANGE:**
  * Puddler now has it's dedicated configuration folder on Windows. Additionally, it's folder-name is finally lowercase.
  * How to use your old config files:
    * Windows:
      * go into `%APPDATA%\Roaming\VernoxVernax`, and move `Puddler` up one directory (into `Roaming`).
      * Rename the `Puddler` folder to `puddler` and delete the empty folder: `VernoxVernax`.
    * Linux:
      * `mv ~/.config/Puddler ~/.config/puddler`
    * **You might also want to check out the first item in Puddler's settings.**
* Small little change to re-enable "continue-watching" section for Emby-beta instances ([#5](https://github.com/Vernoxvernax/Puddler-RS/issues/5)).
* Fix bug to allow empty input (sometimes).
* Print connection status on only one line.
* Load external subtitles automatically.
* Add limit of 15 items to "continue-watching" (this should be a quick-menu).
* New dev branch to keep a reasonably stable code base on main [#7](https://github.com/Vernoxvernax/Puddler-RS/issues/7)
* Automated builds and artifact generation [#9](https://github.com/Vernoxvernax/Puddler-RS/issues/9) and [#7](https://github.com/Vernoxvernax/Puddler-RS/issues/7)
* Ask the user to finish/play-item-again, if the item has not been marked as 100% played. Additionally, an option to just mark it as played. [#8](https://github.com/Vernoxvernax/Puddler-RS/issues/8)
* Make discord operate multithreaded. [#8](https://github.com/Vernoxvernax/Puddler-RS/issues/8)
* Switch to an active discord-presence library: [jewlexx/discord-presence](https://github.com/jewlexx/discord-presence).
* Switch to an active libmpv abstraction: [sirno/libmpv-sirno](https://github.com/sirno/libmpv-rs).
* Fix semi-automatic config repairs. (this sounds cooler than it actually is)
* Bump other dependencies.
* Lots of code-refactoring.

## **0.5.13**
* YAY. We got our first few issues!
* A lot more user-friendly error messages in case of connection issues
* Fixed special playback
  * When choosing a special from the "Continue Watching" menu:

```
Library:
  Specials:
    SP1
    SP2
  Season 1:
    E01 [WATCHED]
    SP1
    E02

>> After finishing SP1 (which was selected from the "C.W." menu), the script will continue with E02, instead of SP2.
```

  * Additionally, the `You've chosen ...` message will now include a little indicator (`Embedded`) at the end, if the special is the one from the specials season or the *embedded* one.
  * ###### Yes, there has already been commits to implement exactly this feature. Unfortunately, some of Emby's implementations are neither documented, nor do they make sense. Please understand, or submit issues.
* [#1] Added configuration setting to use the default configuration folder, or the one specified in `--mpv-config-dir`.
* [#1] Added the `--glsl-shaders` CLI options.
  * Useful if you want to load additional shaders without touching MPV's configs.
* [#1] Pressing just `[ENTER]` will now automatically choose the first unwatched item.
* New `--debug` CLI option to log all of MPV's messages to `./mpv.log`.
* [#2] Added `mark` and `unmark` *commands* in series mode, to change the watched state of items.
  * Use formats like these `mark 1-10`, `mark 1,2,3`, `mark 8`
  * Note that this command is only available in series mode, in order to keep the searching functional.
  * The watched state will only be changed on the server. Restart Puddler to see the changes.
  * Keep an eye on the message prompt to see if these are available or not.
* [#2] Decreased limit to mark items was watched, from 90% to 80%.
* Plenty of other bug-fixes

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
