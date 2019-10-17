RUSTUP = rustup
TOOLCHAIN = $(shell cat rust-toolchain)
CARGO = $(RUSTUP) run --install $(TOOLCHAIN) cargo --color always

CLIPPY_LOC = $(shell which cargo-clippy)
FMT_LOC = $(shell which cargo-fmt)
TOMLFMT_LOC = $(shell which cargo-tomlfmt)

CARGO_ENV = $(HOME)/.cargo/env

build: build_debug

## Dev environment

$(CLIPPY_LOC):
	test -e $@ || $(RUSTUP) component add clippy --toolchain $(TOOLCHAIN)

$(FMT_LOC):
	test -e $@ || $(RUSTUP) component add rustfmt --toolchain $(TOOLCHAIN)

$(TOMLFMT_LOC):
	test -e $@ || $(CARGO) install cargo-tomlfmt

dev_env: $(CLIPPY_LOC) $(FMT_LOC) $(TOMLFMT_LOC)

## User install

install:
	@$(CARGO) install --path .

clean:
	@$(CARGO) clean

## Development tasks

all: dev_env format build_debug clippy test doc

format: $(FMT_LOC) $(TOMLFMT_LOC)
	@$(CARGO) fmt
	@$(CARGO) tomlfmt -p Cargo.toml

build_debug:
	@$(CARGO) build --all --all-targets

clippy: $(CLIPPY_LOC)
	@$(CARGO) clippy --all-targets -- -D warnings

test:
	@$(CARGO) test --all

doc:
	@$(CARGO) doc

check_format: $(FMT_LOC) $(TOMLFMT_LOC)
	@$(CARGO) fmt -- --check
	@$(CARGO) tomlfmt -d -p Cargo.toml
