all: build

install-rust:
ifeq ($(shell which rustc),)
	sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
	sh -c "echo 'source $HOME/.cargo/env' >> ${HOME}/.bashrc"
endif

clean: install-rust
	rm -f server
	cargo clean

build: install-rust
	cargo build --release
	cp target/release/bibifi server
