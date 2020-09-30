#!/usr/bin/env bash

set -ex

cargo build --release --example keyboard_layout
sudo target/release/examples/keyboard_layout
