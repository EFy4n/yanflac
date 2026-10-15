# yanflac

a terminal music player written in Rust. plays local FLAC, MP3, OGG and WAV files.
shows tags, no GUI needed.

*(ongoing project)*

## features

- reads tags: artist, album, album artist, genre, title, track number, length
- plays a single audio file

## controls

vim-like keys. `#` means an optional number typed before the command.
if no number is given, it counts as 1.

```
playback:
 space       pause / resume
 + or =      volume up
 -           volume down
 q           quit

seeking:
 #sh         back # seconds      example: 10sh = back 10 seconds
 #sl         forward # seconds   example: 30sl = forward 30 seconds
 #mh         back # minutes      example: 2mh  = back 2 minutes
 #ml         forward # minutes   example: ml   = forward 1 minute

tracks:
 h           previous track
 l           next track
 #t          play track number # example: 4t   = play track 4
 o           cycle loop mode: off, track, album

typing commands:
 0-9         start or extend a count
 esc         cancel the command being typed
```

while you type a command, it shows in the corner, just like in vim.

**note:** `h` and `l` on their own change tracks. after `s` or `m`,
they mean back and forward inside the track.

**coming soon:** keybindings will move to a lua config file,
so they can be changed without recompiling.

## built with

- [lofty](https://crates.io/crates/lofty): tag reading
- [rodio](https://crates.io/crates/rodio): audio playback
