## **BREAKING CHANGE:**

... yeah well, cause everything has been rewritten.

Just documenting all changes of this new version would take me days and even then I'd forget about certain things.
Well here is a short summary of the most notable ones:

* ~~Cleaner Code~~
* Support for Plex Streaming
* Emby/Jellyfin: Support for simple Pause and Stop commands (comming from emby app/web)
* Actually usable interface
  * Pretty much every input is handled through `crossterm` with some neat eyecandy such as colors, highlights and bold text
  * And for some reason, almost all of the lines printed by puddler will be deleted afterwards.
    * While using puddler myself, I often got a little annoying at how verbose everything was.
    * Even though I knew exactly what item to play, just pressing enter, I ended up with a couple dozent of unnecessary lines during my viewing experience.
    * Not anymore.
  * Note that all menus are created and managed through `crossterm` and they are pretty untested as of right now.
* The config files are now also much more cleaner. (I hope)

Additional Information:
* Authentication for plex has only been tested with an admin account. I don't have plex pass and am not planning to get one.

___

### **Noteable changes the first commit of 0.7.0**

* fix(printing): cleaner item titles; correctly splitting chars
  * "(Played)" was still in Mpv's titlebar.
  * ContinueMenu: Dont split chars based on the byte index
* fix(jellyfin): avoid duplicates in menu
  * Whenever an episode has been played long enough to show up in Continue Watching, it also shows up in NextUp which lead to duplicates in the menu.
* refactor(playlist): exit playlist mode when there is nothing left to do
* fix(text): fix incorrect description of option (hardware acceleration)
* fix(cli/config): don't override the debug_log option
* fix(playback): fix the "finish" item option
* fix(input): only update the interface when necessary
* fix(jellyfin/emby): fix external subtitles url
* feat(playback): support for user preferred audio/sub tracks
  * This is currently very experimental.
  * Every platform returns different kinds of language codes so I had to add another dependency. (very small though)
  * It's behaviour is currently pretty basic. When the preferred audio lang-code is something like `eng`, it will search the tracks of the file for a track that has the same language attribute. If the audio/sub preference is unset, the selection will fall back to the player, so the default track will be selected.
  * This is implemented for non-transcode streams, because you'd be forced to choose a track when transcoding.
* fix(input): ask again for the server address if the request failed (login)
* fix(input): only take keypresses; this should also fix user input on windows
* fix(config/settings): correctly generate paths to the config files/folders
* fix(transcoding): use the correct mediasource/file when selecting sub/audio tracks
* fix(transcoding): ask for start-time even if there is no progress
* fix(playback_reporting): correctly convert time and actually use the  modified item
* feat(jellyfin): add option to select from multiple mediasources
* fix(interface): ask for subtitle tracks in a seperate window
* fix(transcoding): added missing negation for numeric check (the mbps question is now working as expected)
* fix(mpv): avoid getting strings like "(Played)" into the item title
* fix(jellyfin): fix playback finished menu

### **Please create an issue if you find anything that seems weird, bug or not.**
