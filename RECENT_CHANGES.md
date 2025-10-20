* [#16]: increase timeout of all network requests to 15 seconds and don't just unwrap
* [#17]:
  * Support for special character search (like chinese chars) (not extensively tested)
  * Audio & subtitle preferences are apparently optional. Don't expect these to exist in the json response.
* A lot of `cargo fmt` changes (cleaner code)
* Follow up to Plex API change which led to incomplete item metadata.
* Puddler Menu: show the media-center name instead of "Stream from default Media-Center"
* Media-Center Settings: add the option to change the name
* Discord API change: discord presence now shows a progress bar instead of the remaining time
* Fix for emby api change
* Fix for time formatting when interrupting a plex session
* Chores: switch to rust 2024; bump dependencies; cargo fmt (not check lol)
* Fix(plex): don't try to configure new media-server when being forced to re-authenticate
* Fix: avoid empty lines when the options list is very small (temporary)
* Fix(plex): make toggling the watching status of series items functional
* Fix: the old deprecated rust `.config` file
* Fix(jellyfin): add support for multi episode files (S01E01-02)
* Fix(rpc): keep status type as watching while paused
* Chore: replace unmaintained isahc with reqwest
* Fix(plex): (re)authentication bug
* [#18]: fix: use from_str for json struct instead of json!
* [#19]: fix: rewrite regex to support all domains and ip addresses the user could possibly present
* fix: if missing, automatically add `http` in front of ip addresses and `https` infront of domains
* fix(jellyfin): make the mediasource path optional and skip it when needed
* fix(jellyfin): skip mediasources with a None path
* chore: bump dependencies
* fix(jellyfin): change to avoid having the same year in the name twice (2020-2020)
* fix(input): remove excessive newline breaking the interactive menu in one place
* fix(jellyfin): fix issue where a selected mediasource was not played without transcoding
* fix(plex): fix value not being serialized yet
* fix(plex): remove unneeded but breaking part of the PlexMediaFile struct
* fix(plex): send the get request in the asynchronous context instead of blocking
* feat(jellyfin): change playstate if remaining time is smaller than 5min or if watchtime is longer than 4min
* fix: replace sleep with tokio::time::sleep; remove mpsc var
* chore: some changes for discord-presence=v2.0.0
* fix: switch to libmpv2; build_libmpv feature for windows; mpv.lib is now redundant
* feat: extend playstate changes of #49c01e4 to the plex implementation
* chore: general cleanup; ci, code and git related
