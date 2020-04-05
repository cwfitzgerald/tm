.PHONY: clean

tm.tar.xz: src/main.rs Cargo.lock Cargo.toml
	cargo build --release
	cp target/release/tm tm
	tar cJf tm.tar.xz src tm Makefile Cargo.lock Cargo.toml

clean: 
	rm tm tm.tar.xz
