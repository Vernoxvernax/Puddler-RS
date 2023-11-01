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
