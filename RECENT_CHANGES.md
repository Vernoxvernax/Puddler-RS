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
