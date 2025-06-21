.DEFAULT_GOAL := help

.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# -- variables --------------------------------------------------------------------------------------

WARNINGS=RUSTDOCFLAGS="-D warnings"

# -- linting --------------------------------------------------------------------------------------

.PHONY: clippy
clippy: ## Run Clippy with configs
	$(WARNINGS) cargo clippy --workspace --all-targets --all-features


.PHONY: fix
fix: ## Run Fix with configs
	cargo fix --allow-staged --allow-dirty --all-targets --all-features


.PHONY: format
format: ## Run Format using nightly toolchain
	cargo +nightly fmt --all


.PHONY: format-check
format-check: ## Run Format using nightly toolchain but only in check mode
	cargo +nightly fmt --all --check


.PHONY: lint
lint: format fix clippy ## Run all linting tasks at once (Clippy, fixing, formatting)

# --- docs ----------------------------------------------------------------------------------------

.PHONY: doc
doc: ## Generates & checks documentation
	cargo doc --keep-going --release


.PHONY: book
book: ## Builds the book & serves documentation site
	mdbook serve --open docs