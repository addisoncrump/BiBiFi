all: build

install-deps:
ifeq ($(shell env PATH=${HOME}/.cargo/bin:${PATH} which rustc),)
	sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
	env PATH=${HOME}/.cargo/bin:${PATH} cargo fetch
endif

clean:
	rm -f server
	env PATH=${HOME}/.cargo/bin:${PATH} cargo --offline clean

build:
	env PATH=${HOME}/.cargo/bin:${PATH} cargo --offline build --release
	cp target/release/bibifi server
