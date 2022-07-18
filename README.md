# Puddler-RS

Puddler rewritten in Rust.

## Requirements

* fully configured emby or jellyfin server (+account)
* at least some knowledge of ipadresses and networking
* git

___

## Installation

Go to [releases](https://github.com/VernoxVernax/puddler-rs/-/releases) and choose between the binaries there.

### Linux:

* Install `mpv` (Arch-Linux):
```
$ pacman -S mpv
```
* Run it:
```
$ ./puddler
```

### Windows:

Go to [mpv-player](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) at sourceforge.net and get the newest **x86_64** archive. This archive contains `mpv-2.dll` which will need to be placed next to `puddler.exe`.

Then you should be able to run it.

___

## Compiling:

* cargo
* git

___

### Prerequisites:

The mpv crate I'm using for puddler is outdated and had to loose some api bindings.

Clone my fork of `mpv-rs`:
```
$ git clone https://github.com/VernoxVernax/mpv-rs.git
```

#### You can just leave this git-clone next to the `puddler-rs` folder. It's relative path is important!

___

### Linux:

Clone this repo and compile to `/usr/bin/.`:
```
$ git clone https://github.com/VernoxVernax/puddler-rs.git
$ ./install.sh
```

___

### Windows:

Clone this repo and compile `puddler.exe`:
```
$ git clone https://github.com/VernoxVernax/puddler-rs.git
$ cargo build --release
```

Go to [mpv-player](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) at sourceforge.net and get the newest **x86_64** archive. This archive contains `mpv-2.dll` which will need to be placed next to `puddler.exe`.

___


### Linux-to-Windows Cross-Compile:

In case someone struggles with linking to libmpv just like me:

You will need the following tools:
+ Windows (yes, the OS)
+ Arch linux OR knowledge how to do install aur packages on other distros
+ Microsoft Visual Studio 14.0 (vcvarsall.bat)
+ mingw-w64-tools (for `gendef`)
+ 7z

Procedure:

+ Get the newest libmpv-dev from sourceforge [here](https://sourceforge.net/projects/mpv-player-windows/files/libmpv/) (wget command below).


```
$ rustup target add x86_64-pc-windows-gnu
$ git clone https://github.com/VernoxVernax/mpv-rs.git
$ git clone https://github.com/VernoxVernax/puddler-rs.git
$ cd puddler-rs
$ wget https://sourceforge.net/projects/mpv-player-windows/files/libmpv/mpv-dev-x86_64-20220626-git-3a2838c.7z/download -O mpv.7z
$ 7z e -y mpv-dev.7z -ompv-dev
$ cd mpv-dev
$ gendef mpv-2.dll
```

On a windows system, you'll now have to open `cmd` and `cd` in to the `mpv-dev` folder.

```
$ "C:\Progam Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat"
$ lib /def:mpv-2.def /name:mpv-2.dll /out:mpv.lib
```

Back on linux you should now be able to compile it just like this:
```
$ cargo build --target=x86_64-pc-windows-gnu --release
```

Errors upon compiling? Take a look at `./.cargo/config`.

___

Now features everything the python version does, just a little better.