# Puddler-RS

Emby & Jellyfin command line client, powered by mpv. Written in Rust.

[![Rust](https://github.com/Vernoxvernax/Puddler-RS/actions/workflows/tag_release.yml/badge.svg)](https://github.com/Vernoxvernax/Puddler-RS/actions/workflows/tag_release.yml)
![GitHub issues](https://img.shields.io/github/issues/Vernoxvernax/Puddler-RS)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/Vernoxvernax/Puddler-RS)
![GitHub](https://img.shields.io/github/license/Vernoxvernax/Puddler-RS)

![GitHub all releases](https://img.shields.io/github/downloads/Vernoxvernax/Puddler-RS/total)

___

## Requirements

* fully configured emby or jellyfin server (+account)
* at least some knowledge of ipadresses and networking

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
$ ./Puddler
```

### Windows:

Go to [mpv-player](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) at sourceforge.net and get the newest libmpv **x86_64** archive. This contains `mpv-2.dll` which will need to be placed next to `Puddler.exe`.

Then you should be able to run it.

___

### Limitations:

* The api doesn't support transcoding while also delivering font attachments.
    *  Heavily stylied subtitles won't display as intended, and it seems there is currently no feasable way around it

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
then you may add it to your PATH variable:
```
$ export PATH="$HOME/.cargo/bin:$PATH"
```

___

### Windows:

Go to [mpv-player](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) at sourceforge.net and get the newest libmpv **x86_64** archive. This contains `mpv-2.dll` which will need to be placed next to `Puddler.exe`.

Then just compile it:
```
$ cargo install --path .
```

Cargo will print the output folder of the `.exe` so that you know what folder to add to your PATH.

___

### Linux-to-Windows Cross-Compile:

In case someone struggles with linking to libmpv just like me:

Additionally you will need the following tools:
+ Windows (yes, the OS)
+ Microsoft Visual Studio 14.0 (vcvarsall.bat)
+ Arch linux OR knowledge how to find aur packages for your distro
+ mingw-w64-tools (for `gendef`)
+ 7z

#### **Procedure:**

+ Get the newest libmpv-dev from sourceforge [here](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) (wget command below).
+ Visual Studio (f.e. the community edition) [microsoft.com](https://visualstudio.microsoft.com/vs/features/cplusplus)
    + for the sole purpose of libmpv, you will only need `Desktop Development With C++ Workload`.


```
rustup target add x86_64-pc-windows-gnu
git clone https://github.com/VernoxVernax/Puddler-RS.git
cd Puddler-RS
wget https://sourceforge.net/projects/mpv-player-windows/files/libmpv/mpv-dev-x86_64-20220626-git-3a2838c.7z/download -O mpv.7z
7z e -y mpv-dev.7z -ompv-dev
cd mpv-dev
gendef mpv-2.dll
```

On a windows system, you'll now have to open `cmd` and `cd` in to the `mpv-dev` folder.

```
"C:\Progam Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat" x86_amd64
lib /def:mpv-2.def /name:mpv-2.dll /out:mpv.lib /MACHINE:x64
```
then copy `mpv.lib` to the `mpv` folder


Back on linux you should now be able to compile it just like this:
```
cargo build --target=x86_64-pc-windows-gnu --release
```

Errors upon compiling? Take a look at `./.cargo/config`.
