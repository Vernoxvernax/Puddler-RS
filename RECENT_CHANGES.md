### Changes since **0.7.0**:

* feat(playback): support for user preferred audio/sub tracks
  * This is currently very experimental.
  * Every platform returns different kinds of language codes so I had to add another dependency. (very small though)
  * It's behviour is currently pretty basic. When the preferred audio lang-code is something like `eng`, it will search the tracks of the file for a track that has the same language attribute. If the audio/sub preference is unset, the selection will fall back to the player, so the default track will be selected.
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
