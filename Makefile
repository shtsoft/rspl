EXAMPLES = $(patsubst %.rs, %.md, $(wildcard examples/*.rs))

%.md: %.rs
	sed 's/^\/\/&//' $< > $@

all: $(EXAMPLES)
	cargo fmt --all -- --check
	cargo clippy --all --benches --examples --tests --all-features
	cargo doc --no-deps --document-private-items
	cargo test
