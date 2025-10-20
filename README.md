# Puddler-RS

Jellyfin & Emby & Plex command line client, powered by MPV. Written in Rust.

[![Rust](https://github.com/Vernoxvernax/Puddler-RS/actions/workflows/release-builds.yml/badge.svg)](https://github.com/Vernoxvernax/Puddler-RS/actions/workflows/release-builds.yml)
![GitHub issues](https://img.shields.io/github/issues/Vernoxvernax/Puddler-RS)
![GitHub release (the latest by date)](https://img.shields.io/github/v/release/Vernoxvernax/Puddler-RS)
![GitHub](https://img.shields.io/github/license/Vernoxvernax/Puddler-RS)

![GitHub all releases](https://img.shields.io/github/downloads/Vernoxvernax/Puddler-RS/total)

___

## Requirements

* fully configured Jellyfin or Emby or Plex server (+account)
* at least some knowledge of IP addresses and networking

___

## Installation

Head over to [releases](https://github.com/VernoxVernax/Puddler-RS/releases) and choose between the binaries there.

### Linux:

*Install `mpv`:*

+ Arch-Linux:
```
$ pacman -S mpv
```
+ Debian:
```
$ apt-get install mpv
```

*Run it:*
```
$ ./puddler
```

### Windows:

Go to [mpv-player](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) at Sourceforge.net and get the newest libmpv **x86_64** archive. This contains `libmpv-2.dll` which will need to be placed next to `puddler.exe` or into PATH.

Then you should be able to run it.

___

## Compiling:

What you'll need:

* cargo
* git

___

### Linux:

Clone this repo and install the binary:
```
$ git clone https://github.com/VernoxVernax/Puddler-RS.git
$ cargo build --release
$ cargo install --path .
```
Then you may add it to your PATH variable:
```
$ export PATH="$HOME/.cargo/bin:$PATH"
```

___

### Windows:

Go to [mpv-player](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) at Sourceforge.net and get the newest libmpv **x86_64** archive. This contains `libmpv-2.dll` which will need to be placed next to `puddler.exe`.

Then just compile it:
```
$ git clone https://github.com/VernoxVernax/Puddler-RS.git
$ cargo build --release
$ cargo install --path .
```

Cargo will print the output folder of the `.exe` so that you know what folder to add to your PATH.

___

### Linux-to-Windows Cross-Compiling:

In case someone struggles with linking to libmpv just like me:

Additionally, you will need the following tools:
+ 7zip

#### **Procedure:**

+ Get the newest libmpv-dev from Sourceforge [here](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) (`wget` command below).

```
rustup target add x86_64-pc-windows-gnu
git clone https://github.com/VernoxVernax/Puddler-RS.git
cd Puddler-RS
wget https://downloads.sourceforge.net/project/mpv-player-windows/libmpv/mpv-dev-x86_64-20250928-git-db2b436.7z -O mpv.7z
7z e -y mpv.7z -o64
export MPV_SOURCE="$(pwd)"
cargo build --target=x86_64-pc-windows-gnu --release
```

Errors upon compiling? Give up!
