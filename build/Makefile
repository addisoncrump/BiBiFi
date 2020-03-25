all: build

install-rust:
ifeq ($(shell env PATH=${HOME}/.cargo/bin:${PATH} which rustc),)
	sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
endif

clean: install-rust
	rm -f server
	env PATH=${HOME}/.cargo/bin:${PATH} cargo clean

build: install-rust
	env PATH=${HOME}/.cargo/bin:${PATH} cargo build --release
	cp target/release/bibifi server
