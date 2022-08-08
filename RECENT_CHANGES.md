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