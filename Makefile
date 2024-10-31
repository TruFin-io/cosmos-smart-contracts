RFLAGS="-C link-arg=-s"

build-staker:
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p injective-staker

test-staker: build-staker build-wasm
	RUSTFLAGS=$(RFLAGS) RUST_TEST_THREADS=1 cargo test -p injective-staker --features test

build: build-staker

build-debug:
	cargo wasm-debug

build-wasm:
	mkdir -p ./contracts/injective-staker/tests/test_artifacts/
	cd ./contracts/injective-staker && cargo wasm && cd ..
	cp ./target/wasm32-unknown-unknown/release/injective_staker.wasm ./contracts/injective-staker/tests/test_artifacts/

build-optimized:
	docker run --platform linux/amd64 --rm -v ./:/code \
  --mount type=volume,source=src_cache,target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.16.0

test: test-staker

schema:
	cd contracts/injective-staker && cargo schema

validate:
	cosmwasm-check ./target/wasm32-unknown-unknown/release/injective_staker.wasm

check-format:
	cargo fmt --check
	cargo clippy --all-features --workspace --tests -- --warn clippy::all --warn clippy::nursery

check-coverage: test
	DYLD_LIBRARY_PATH="`pwd`/target/debug/deps" cargo tarpaulin --all-features --skip-clean --out Html --output-dir coverage-report

clean:
	cargo clean
	rm -rf target/
	rm -rf artifacts/
	rm -rf schema/
	rm -rf ./contracts/injective-staker/tests/test_artifacts/
