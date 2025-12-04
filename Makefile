.PHONY: build

build:
	cargo build -p router
	sudo setcap 'cap_net_admin+eip' target/debug/router

run: build
	./target/debug/router $(ARGS)

clean:
	cargo clean
