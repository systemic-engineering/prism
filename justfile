# prism — focus | project | refract

# Run all tests
test:
    nix develop -c cargo test --workspace --features optics

# Run tests with coverage report
coverage:
    nix develop -c cargo llvm-cov --workspace --features optics

# Coverage gate — fails if below threshold
pre-push:
    nix develop -c cargo llvm-cov --workspace --features optics --fail-under-lines 99

# Build workspace
build:
    nix develop -c cargo build --workspace --features optics

# Clippy lint
lint:
    nix develop -c cargo clippy --workspace --features optics -- -D warnings

# Format check
fmt-check:
    nix develop -c cargo fmt --check

# Format
fmt:
    nix develop -c cargo fmt

# Full check (what CI and pre-push hook run)
check: test lint pre-push
