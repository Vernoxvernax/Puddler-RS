#!/bin/bash
cargo build --release
#sudo cp ./target/release/puddler /usr/bin/.
sudo install -Dm755 target/release/puddler "/usr/bin/puddler"
