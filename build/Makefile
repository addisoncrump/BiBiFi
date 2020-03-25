all: install-rust build

check-rust:


install-rust: check-rust
ifeq ($(shell which rustc),)
	sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
	sh -c "echo 'source $HOME/.cargo/env'"
endif

build:
	cargo build --release
