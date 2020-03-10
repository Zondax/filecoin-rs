deps_wasm:
	cargo install wasm-pack --version 0.8.1

build_wasm: deps_wasm
	rm -rf wasm-signer/pkg/
	wasm-pack build wasm-signer/
	# temporary workaround
	cp package-wasm-signer.json wasm-signer/pkg/package.json
	cp wasm-signer/pkg/wasm-signer.js wasm-signer/pkg/wasm-signer.mjs

link_wasm: build_wasm
	cd examples/wasm && yarn install
	cd wasm-signer/pkg && yarn link
	cd examples/wasm && yarn link "wasm-signer"

test_wasm_unit: deps_wasm
	wasm-pack test --chrome --headless ./fcwasmsigner

test_wasm_integration: link_wasm
	cd examples/wasm && yarn run test:integration

test_wasm: test_wasm_unit test_wasm_integration

deps_rust:
	cargo install cargo-audit
	cargo install cargo-tree
	cargo install cargo-license
	cargo install cargo-outdated
	cargo install cargo-watch https
	cargo install sccache
	yarn install
	echo "Remember to add export RUSTC_WRAPPER=sccache to your environment."

deps: deps_wasm deps_rust

checks:
	cargo fmt -- --check
	cargo clippy --all-features
	cargo audit

hooks:
	git config core.hooksPath .githooks

# prepreprocess circleci config so it can be ran locally
# Usage example:
# make ci JOB=test_service
ci:
	circleci config process .circleci/config.yml > .circleci/tmp.yml
	circleci build -c .circleci/tmp.yml --job ${JOB}

docs_dev:
	yarn install
	yarn dev

# Run this to have live refreshing of rust docs
docs_rust_edit: deps
	cargo watch -s 'cargo doc && browser-sync start --ss target/doc -s target/doc --directory --no-open'

docs_build:
	yarn install
	yarn build
