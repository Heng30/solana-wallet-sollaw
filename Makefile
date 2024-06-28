#!/bin/bash

pwd=${shell pwd}
build-evn=SLINT_STYLE=material RUSTFLAGS="--remap-path-prefix $(HOME)=/home --remap-path-prefix $(pwd)=/build"
run-evn=RUST_LOG=error,warn,info,debug,sqlx=off,reqwest=off
version=`git describe --tags --abbrev=0`
apk-build-dir=$(CURDIR)/main

all: build-release

build:
	cd $(apk-build-dir) && $(build-evn) cargo apk build --lib

build-release:
	cd $(apk-build-dir) && $(build-evn) cargo apk build --lib --release
	cp -f target/release/apk/sollet.apk target/sollet-${version}.apk

build-release-mold:
	cd $(apk-build-dir) && $(build-evn) mold --run cargo apk build --lib --release
	cp -f target/release/apk/sollet.apk target/sollet-${version}.apk

run:
	cd $(apk-build-dir) && RUST_BACKTRACE=1 $(run-evn) cargo apk run --lib

run-release:
	cd $(apk-build-dir) && RUST_BACKTRACE=1 $(run-evn) cargo apk run --lib --release

run-release-mold:
	cd $(apk-build-dir) && RUST_BACKTRACE=1 $(run-evn) mold --run cargo apk run --lib --release

install:
	cd $(apk-build-dir) && $(build-evn) $(run-evn) cargo apk run --lib --release

debug:
	$(build-evn) $(run-evn) cargo run --bin sollet-desktop --features=desktop

debug-mold:
	$(build-evn) $(run-evn) mold --run cargo run --bin sollet-desktop --features=desktop

debug-local:
	$(run-evn) ./target/debug/sollet-desktop

build-desktop-debug:
	$(build-evn) $(run-evn) cargo build --bin sollet-desktop --features=desktop

build-desktop-release:
	$(build-evn) $(run-evn) cargo build --release --bin sollet-desktop --features=desktop

install-desktop:
	cp -f target/release/sollet-desktop ~/bin/sollet-desktop

test:
	$(build-evn) $(run-evn) cargo test -- --nocapture

clippy:
	cargo clippy

clean-incremental:
	rm -rf ./target/debug/incremental/*
	rm -rf ./target/aarch64-linux-android/debug/incremental

clean-unused-dependences:
	cargo machete

clean:
	cargo clean

slint-view:
	cd $(apk-build-dir) && slint-viewer --style material --auto-reload -I ui ./ui/appwindow.slint

slint-view-light:
	cd $(apk-build-dir) && slint-viewer --style material-light --auto-reload -I ui ./ui/appwindow.slint

slint-view-dark:
	cd $(apk-build-dir) && slint-viewer --style material-dark --auto-reload -I ui ./ui/appwindow.slint

get-font-name:
	cd $(apk-build-dir) && fc-scan ./ui/fonts/SourceHanSerifCN.ttf | grep fullname && fc-scan ./ui/fonts/Plaster-Regular.ttf | grep fullname
