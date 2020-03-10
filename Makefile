
deps_wasm:
	cargo install wasm-pack --version 0.8.1 --force

build_wasm:
	rm -rf fcwebsigner/pkg/
	wasm-pack build fcwebsigner/
	# temporary workaround
	cp package-fcwebsigner.json fcwebsigner/pkg/package.json
	cp fcwebsigner/pkg/fcwebsigner.js fcwebsigner/pkg/fcwebsigner.mjs

link_wasm:
	cd fcwebsigner/pkg && yarn link
	cd examples/wasm && yarn link "fcwebsigner"

test_wasm_unit:
	wasm-pack test --firefox --chrome --headless ./fcwebsigner

test_wasm_integration:
	cd examples/wasm && yarn run test:integration

deps: deps_wasm
	cargo install cargo-audit
	cargo install cargo-tree
	cargo install cargo-license
	cargo install cargo-outdated
	cargo install sccache
	echo "Remember to add export RUSTC_WRAPPER=sccache to your environment."

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
