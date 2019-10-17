RUSTUP = rustup
TOOLCHAIN = $(shell cat rust-toolchain)
CARGO = $(RUSTUP) run --install $(TOOLCHAIN) cargo --color always

CLIPPY = $(shell which cargo-clippy)
FMT = $(shell which cargo-fmt)
TOMLFMT = $(shell which cargo-tomlfmt)

CARGO_ENV = $(HOME)/.cargo/env

build: build_debug

## Dev environment

$(CLIPPY):
	test -e $@ || $(RUSTUP) component add clippy --toolchain $(TOOLCHAIN)
	CLIPPY = $(shell which cargo-clippy)

$(FMT):
	test -e $@ || $(RUSTUP) component add rustfmt --toolchain $(TOOLCHAIN)
	FMT = $(shell which cargo-fmt)

$(TOMLFMT):
	test -e $@ || $(CARGO) install cargo-tomlfmt
	TOMLFMT = $(shell which cargo-tomlfmt)

dev_env: $(CLIPPY) $(FMT) $(TOMLFMT)

## User install

install:
	@$(CARGO) install --path .

clean:
	@$(CARGO) clean

## Development tasks

all: dev_env format build_debug clippy test doc

format: $(FMT) $(TOMLFMT)
	@$(CARGO) fmt
	@$(CARGO) tomlfmt -p Cargo.toml

build_debug:
	@$(CARGO) build --all --all-targets

clippy: $(CLIPPY)
	@$(CARGO) clippy --all-targets -- -D warnings

test:
	@$(CARGO) test --all

doc:
	@$(CARGO) doc

check_format: $(FMT) $(TOMLFMT)
	@$(CARGO) fmt -- --check
	@$(CARGO) tomlfmt -d -p Cargo.toml
