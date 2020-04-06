.PHONY: clean

connor-fitzgerald-tm.tar.xz: src/main.rs Cargo.lock Cargo.toml
	cargo build --release
	cp target/release/connor-fitzgerald-tm connor-fitzgerald-tm
	tar cJf connor-fitzgerald-tm.tar.xz src connor-fitzgerald-tm Makefile Cargo.lock Cargo.toml

clean: 
	rm connor-fitzgerald-tm connor-fitzgerald-tm.tar.xz
