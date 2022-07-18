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