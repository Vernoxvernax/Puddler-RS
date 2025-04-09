## **Changes since 0.7.0**

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
