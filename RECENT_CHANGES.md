* new option to resume from manually specified minute (if transcoding is enabled)
* recreated `mpv.lib` to link against new features
* fixed playback report when continuing stream with transcoding turned on
* fixed next episode option after watching a nested special (below)
* changed hotkey for next episode to 'n' instead of 'c'
* updated readme for those who installed visual studio 2022
* fixed a few printing mistakes

___

### First important decision:

Imagine this situation:

- Item A [Played]
- Item B [Played]
- Item C

What should happen if the user specificly rewatches and finishes Item A.
Should Item B be recommened for continuation, or Item C?

This had me thinking for a while, but in the end I decided for the latter (recommending Item C).
: Items that have already been finished, won't be recommended anymore. (unless you change the status in the emby interface of course)

___

### Second important decision:

Imagine the following:

Specials:
- Sp 1
- Sp 2
Season 1:
- Ep 1
- *Sp 1*
- Ep 2

`Sp 1` has been added to season 1 since its content extends story events from `Ep 1` (done by your database).


What is supposed to happen if the user plays & finishes `Sp 1`?
Should the player continue with `Sp 2` or with `Ep 2`?

Once again a pretty hard question. I decided for the latter (`Ep 2`).

___
___
