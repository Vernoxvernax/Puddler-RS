## **BREAKING CHANGE:**

... yeah well, cause everything has been rewritten.

Just documenting all changes of this commit would take me days and even then I'd forget about certain things.
Well here is a short summary of the most notable ones:

* ~~Cleaner Code~~
* Support for Plex Streaming
* Emby/Jellyfin: Support for simple Pause and Stop commands (comming from emby app/web)
* Actually usable interface
  * Pretty much every input is handled through `crossterm` with some neat eyecandy such as colors and highlights with bold
  * And of course, almost all of the lines printed by puddler will be deleted afterwards.
    * While using puddler myself, I often got a little annoying at how verbose everything was.
    * Even though I knew exactly what item to play, just pressing enter, I ended up with a couple dozent of unnecessary lines at the end of my viewing experience.
    * Well not anymore.
  * Note that all menus are created and managed through `crossterm` and they are pretty untested as of right now.
* The config files are now also more cleaner. (I hope)

### **Please create an issue if you find anything that seems weird, bug or not.**

Additional Information:
* Authentication for plex has only been tested with an admin account. I don't have plex pass and am not planning to get it.
