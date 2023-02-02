# Puddler-RS

Emby & Jellyfin command line client, powered by MPV. Written in Rust.

[![Rust](https://github.com/Vernoxvernax/Puddler-RS/actions/workflows/tag_release.yml/badge.svg)](https://github.com/Vernoxvernax/Puddler-RS/actions/workflows/tag_release.yml)
![GitHub issues](https://img.shields.io/github/issues/Vernoxvernax/Puddler-RS)
![GitHub release (the latest by date)](https://img.shields.io/github/v/release/Vernoxvernax/Puddler-RS)
![GitHub](https://img.shields.io/github/license/Vernoxvernax/Puddler-RS)

![GitHub all releases](https://img.shields.io/github/downloads/Vernoxvernax/Puddler-RS/total)

___

## Requirements

* fully configured Emby or Jellyfin server (+account)
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
$ cargo install --path .
```

Cargo will print the output folder of the `.exe` so that you know what folder to add to your PATH.

___

### Linux-to-Windows Cross-Compile:

In case someone struggles with linking to libmpv just like me:

Additionally, you will need the following tools:
+ Windows (yes, the OS)
+ Microsoft Visual Studio 14.0 (vcvarsall.bat)
+ mingw-w64-tools (for `gendef`)
+ 7z

#### **Procedure:**

+ Get the newest libmpv-dev from Sourceforge [here](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) (`wget` command below).
+ Visual Studio (f.e. the community edition) [microsoft.com](https://visualstudio.microsoft.com/vs/features/cplusplus)
    + for the sole purpose of libmpv, you will only need `Desktop Development With C++ Workload`.


```
rustup target add x86_64-pc-windows-gnu
git clone https://github.com/VernoxVernax/Puddler-RS.git
cd Puddler-RS
wget https://sourceforge.net/projects/mpv-player-windows/files/libmpv/mpv-dev-x86_64-v3-20230129-git-86093fc.7z/download -O mpv.7z
7z e -y mpv-dev.7z -ompv-dev
cd mpv-dev
gendef libmpv-2.dll
```

On a Windows system, you'll now have to open `cmd` and `cd` in to the `mpv-dev` folder.

```
"C:\Progam Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat" x86_amd64
lib /def:libmpv-2.def /name:libmpv-2.dll /out:mpv.lib /MACHINE:x64
```
Then copy `mpv.lib` to the `mpv` folder


Back on Linux you should now be able to compile it just like this:
```
cargo build --target=x86_64-pc-windows-gnu --release
```

Errors upon compiling? Take a look at `./.cargo/config`.
