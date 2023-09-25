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
