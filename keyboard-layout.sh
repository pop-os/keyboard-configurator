#!/usr/bin/env bash

set -ex

cargo build --release --example keyboard_layout
target/release/examples/keyboard_layout "$@"
