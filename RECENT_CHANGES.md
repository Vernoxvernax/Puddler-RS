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
* Plenty of other bug-fixes
