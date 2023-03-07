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
