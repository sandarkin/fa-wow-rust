.PHONY: run_server run_client build_server build_client

debug ?= false

run_server:
	cargo run --package tcp_wow --bin server

run_client:
	cargo run --package tcp_wow --bin client

build_server:
	cargo build --release --package tcp_wow --bin server

build_client:
	cargo build --release --package tcp_wow --bin client
