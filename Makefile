CARGO = cargo --color always
RUSTUP = rustup
TOOLCHAIN = $(shell cat rust-toolchain)
TOOLCHAIN_LOC = ""

# Platform specific magic
LIBUSB_LOC = ""
LIBUSB_INSTALL = ""
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)
ifeq ($(UNAME_S),Linux)
	LIBUSB_LOC = /usr/lib/x86_64-linux-gnu/libusb.so
	LIBUSB_INSTALL = sudo apt-get install libusb-dev
	TOOLCHAIN_LOC = $(RUSTUP_DIR)/toolchains/$(TOOLCHAIN)-$(UNAME_M)-unknown-linux-gnu
endif
ifeq ($(UNAME_S),Darwin)
	LIBUSB_LOC = /usr/local/lib/libusb-1.0.dylib
	LIBUSB_INSTALL = brew install libusb
	TOOLCHAIN_LOC = $(RUSTUP_DIR)/toolchains/$(TOOLCHAIN)-$(UNAME_M)-apple-darwin
endif

CARGO_DIR = $(HOME)/.cargo
RUSTUP_DIR = $(HOME)/.rustup

CLIPPY_LOC = $(TOOLCHAIN_LOC)/bin/cargo-clippy
FMT_LOC = $(TOOLCHAIN_LOC)/bin/cargo-fmt
TOMLFMT_LOC = $(CARGO_DIR)/bin/cargo-tomlfmt

build: build_debug

## Build Environment
# `test` is used to only run if the target does not exist
# Instead of running if the pre-requisite is older than the target

$(TOOLCHAIN_LOC):
	test -e $@ || $(RUSTUP) toolchain install $(TOOLCHAIN)

$(LIBUSB_LOC):
	test -e $@ || $(LIBUSB_INSTALL)

build_env: $(TOOLCHAIN_LOC) $(LIBUSB_LOC)

## Dev environment

$(CLIPPY_LOC): $(TOOLCHAIN_LOC)
	test -e $@ || $(RUSTUP) component add clippy --toolchain $(TOOLCHAIN)

$(FMT_LOC): $(TOOLCHAIN_LOC)
	test -e $@ || $(RUSTUP) component add rustfmt --toolchain $(TOOLCHAIN)

$(TOMLFMT_LOC): $(TOOLCHAIN_LOC)
	test -e $@ || $(CARGO) install cargo-tomlfmt

dev_env: build_env $(CLIPPY_LOC) $(FMT_LOC) $(TOMLFMT_LOC)

## User install

install: build_env
	@$(CARGO) install --path .

clean:
	@$(CARGO) clean

## Development tasks

all: dev_env format build_debug clippy test doc

format: $(FMT_LOC) $(TOMLFMT_LOC)
	@$(CARGO) fmt
	@$(CARGO) tomlfmt -p Cargo.toml

build_debug: build_env
	@$(CARGO) build --all --all-targets

clippy: $(CLIPPY_LOC)
	@$(CARGO) clippy --all-targets -- -D warnings

test: build_env
	@$(CARGO) test --all

doc: build_env
	@$(CARGO) doc

check_format: $(FMT_LOC) $(TOMLFMT_LOC)
	@$(CARGO) fmt -- --check
	@$(CARGO) tomlfmt -d -p Cargo.toml
