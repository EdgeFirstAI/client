# EdgeFirst Client Makefile
# Provides common development tasks and pre-commit automation

.PHONY: help format lint test build clean pre-commit pre-release sbom check-license version-check

# Default target
help:
	@echo "EdgeFirst Client - Development Targets"
	@echo ""
	@echo "Common tasks:"
	@echo "  make format      - Format all code (Rust + Python)"
	@echo "  make lint        - Run all linters"
	@echo "  make test        - Run all tests (Rust + Python)"
	@echo "  make build       - Build all crates"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make pre-commit  - Run pre-commit checks (format + lint + build + test)"
	@echo "  make pre-release - Full pre-release validation"
	@echo "  make version-check - Check version consistency across all files"
	@echo "  make sbom        - Generate Software Bill of Materials"
	@echo "  make check-license - Check dependency license compliance"
	@echo ""
	@echo "Rust-specific:"
	@echo "  make rust-format - Format Rust code only"
	@echo "  make rust-lint   - Lint Rust code only"
	@echo "  make rust-test   - Run Rust tests only"
	@echo ""
	@echo "Python-specific:"
	@echo "  make py-format   - Format Python code only"
	@echo "  make py-test     - Run Python tests only"
	@echo "  make py-dev      - Install Python bindings in development mode"

# Format all code
format: rust-format py-format
	@echo "✅ All code formatted"

# Format Rust code
rust-format:
	@echo "Formatting Rust code..."
	cargo +nightly fmt --all

# Format Python code
py-format:
	@echo "Formatting Python code..."
	@if [ -d venv ]; then \
		venv/bin/ruff format *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi; \
		venv/bin/ruff check --fix --exit-zero *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi; \
	else \
		ruff format *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi; \
		ruff check --fix --exit-zero *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi; \
	fi

# Lint all code
lint: rust-lint
	@echo "✅ All linting passed"

# Lint Rust code
rust-lint:
	@echo "Running Rust linter (clippy)..."
	cargo clippy --all-features --all-targets --locked

# Run all tests
test: rust-test py-test
	@echo "✅ All tests passed"

# Run Rust tests
rust-test:
	@echo "Running Rust unit tests..."
	cargo test --all-features --locked
	@echo "Running Rust doc tests..."
	cargo test --doc --locked

# Run Python tests
py-test:
	@echo "Running Python tests..."
	@if [ -d venv ]; then \
		venv/bin/python -m unittest discover -s test -p "test*.py"; \
	else \
		python3 -m unittest discover -s test -p "test*.py"; \
	fi

# Build all crates
build:
	@echo "Building all crates..."
	cargo build --all-features --locked
	@echo "✅ Build successful"

# Build release binaries
build-release:
	@echo "Building release binaries..."
	cargo build --release --all-features --locked
	@echo "✅ Release build successful"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf target/
	rm -rf venv/
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete
	find . -type f -name "*.pyo" -delete
	find . -type f -name ".coverage" -delete
	@echo "✅ Clean complete"

# Install Python bindings in development mode
py-dev:
	@echo "Installing Python bindings in development mode..."
	@if [ -d venv ]; then \
		venv/bin/maturin develop -m crates/edgefirst-client-py/Cargo.toml; \
	else \
		maturin develop -m crates/edgefirst-client-py/Cargo.toml; \
	fi
	@echo "✅ Python bindings installed"

# Pre-commit checks (run before committing)
pre-commit: format lint build
	@echo ""
	@echo "============================================"
	@echo "✅ Pre-commit checks passed!"
	@echo "============================================"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Review your changes: git diff"
	@echo "  2. Update CHANGELOG.md if user-visible changes"
	@echo "  3. Update documentation if needed"
	@echo "  4. Run 'make test' if credentials available"
	@echo "  5. Commit your changes"
	@echo ""

# Pre-release validation (comprehensive checks before release)
pre-release: clean format lint build test sbom check-license version-check
	@echo ""
	@echo "Running pre-release validation..."
	@echo ""
	@echo "Checking CHANGELOG.md..."
	@if ! grep -q "## \[Unreleased\]" CHANGELOG.md; then \
		echo "❌ CHANGELOG.md missing [Unreleased] section"; \
		exit 1; \
	fi
	@echo "✅ CHANGELOG.md has [Unreleased] section"
	@echo ""
	@echo "============================================"
	@echo "✅ Pre-release validation passed!"
	@echo "============================================"
	@echo ""
	@echo "Release checklist:"
	@echo "  1. ✅ All tests passing"
	@echo "  2. ✅ Version consistent"
	@echo "  3. ✅ CHANGELOG.md updated"
	@echo "  4. Review CHANGELOG.md [Unreleased] entries"
	@echo "  5. Run: cargo release patch --execute --no-confirm"
	@echo "  6. Verify tag created: git describe"
	@echo "  7. Push release: git push && git push --tags"
	@echo ""

# Check for common issues
check-issues:
	@echo "Checking for common issues..."
	@echo ""
	@echo "TODO/FIXME comments:"
	@grep -rn "TODO\|FIXME\|XXX" crates/ --include="*.rs" || echo "  None found"
	@echo ""
	@echo "Uncommitted changes:"
	@git status --short || echo "  Not a git repository"
	@echo ""

# Coverage report (requires credentials)
coverage:
	@echo "Generating coverage report..."
	@echo "Setting up coverage environment..."
	@if [ -d venv ]; then \
		source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage) && \
		cargo build --all-features --locked && \
		venv/bin/maturin develop -m crates/edgefirst-client-py/Cargo.toml && \
		venv/bin/python -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python && \
		cargo llvm-cov report --lcov --output-path lcov.info; \
	else \
		source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage) && \
		cargo build --all-features --locked && \
		maturin develop -m crates/edgefirst-client-py/Cargo.toml && \
		python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python && \
		cargo llvm-cov report --lcov --output-path lcov.info; \
	fi
	@echo "✅ Coverage report generated: lcov.info, coverage.xml"

# Install development dependencies
install-deps:
	@echo "Installing development dependencies..."
	@echo "Installing Rust tools..."
	@rustup component add rustfmt clippy
	@rustup toolchain install nightly --component rustfmt
	@echo "Installing Python tools..."
	@if [ -d venv ]; then \
		venv/bin/pip install --upgrade pip; \
		venv/bin/pip install -r requirements.txt; \
	else \
		pip3 install --upgrade pip; \
		pip3 install -r requirements.txt; \
	fi
	@echo "✅ Development dependencies installed"

# Generate Software Bill of Materials (SBOM)
sbom:
	@echo "Generating Software Bill of Materials..."
	@.github/scripts/generate_sbom.sh
	@echo "✅ SBOM generated: sbom.json"

# Check dependency license compliance
check-license:
	@echo "Checking dependency license compliance..."
	@if [ ! -f sbom.json ]; then \
		echo "❌ sbom.json not found. Run 'make sbom' first."; \
		exit 1; \
	fi
	@if [ -d venv ]; then \
		venv/bin/python .github/scripts/check_license_policy.py sbom.json; \
	else \
		python3 .github/scripts/check_license_policy.py sbom.json; \
	fi
	@echo "✅ All dependencies pass license policy"

# Check version consistency across all files
version-check:
	@echo "Checking version consistency across all files..."
	@echo ""
	@cargo_version=$$(grep -m 1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/'); \
	cli_md_version=$$(grep '^footer:' CLI.md | sed 's/footer: edgefirst-client //'); \
	lock_cli=$$(grep -A1 'name = "edgefirst-cli"' Cargo.lock | grep version | sed 's/version = "\(.*\)"/\1/'); \
	lock_client=$$(grep -A1 'name = "edgefirst-client"' Cargo.lock | grep version | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	lock_ffi=$$(grep -A1 'name = "edgefirst-client-ffi"' Cargo.lock | grep version | sed 's/version = "\(.*\)"/\1/'); \
	lock_py=$$(grep -A1 'name = "edgefirst-client-py"' Cargo.lock | grep version | sed 's/version = "\(.*\)"/\1/'); \
	changelog_version=$$(grep -m 1 '## \[' CHANGELOG.md | sed 's/## \[\(.*\)\].*/\1/'); \
	errors=0; \
	echo "Version sources:"; \
	echo "  Cargo.toml:                $$cargo_version"; \
	echo "  CLI.md:                    $$cli_md_version"; \
	echo "  Cargo.lock (cli):          $$lock_cli"; \
	echo "  Cargo.lock (client):       $$lock_client"; \
	echo "  Cargo.lock (client-ffi):   $$lock_ffi"; \
	echo "  Cargo.lock (client-py):    $$lock_py"; \
	echo "  CHANGELOG.md (latest):     $$changelog_version"; \
	echo ""; \
	if [ "$$cargo_version" != "$$cli_md_version" ]; then \
		echo "❌ Mismatch: Cargo.toml ($$cargo_version) != CLI.md ($$cli_md_version)"; \
		errors=1; \
	fi; \
	if [ "$$cargo_version" != "$$lock_cli" ]; then \
		echo "❌ Mismatch: Cargo.toml ($$cargo_version) != Cargo.lock edgefirst-cli ($$lock_cli)"; \
		errors=1; \
	fi; \
	if [ "$$cargo_version" != "$$lock_client" ]; then \
		echo "❌ Mismatch: Cargo.toml ($$cargo_version) != Cargo.lock edgefirst-client ($$lock_client)"; \
		errors=1; \
	fi; \
	if [ "$$cargo_version" != "$$lock_ffi" ]; then \
		echo "❌ Mismatch: Cargo.toml ($$cargo_version) != Cargo.lock edgefirst-client-ffi ($$lock_ffi)"; \
		errors=1; \
	fi; \
	if [ "$$cargo_version" != "$$lock_py" ]; then \
		echo "❌ Mismatch: Cargo.toml ($$cargo_version) != Cargo.lock edgefirst-client-py ($$lock_py)"; \
		errors=1; \
	fi; \
	if [ "$$cargo_version" != "$$changelog_version" ] && [ "$$changelog_version" != "Unreleased" ]; then \
		echo "⚠️  Warning: Cargo.toml ($$cargo_version) != CHANGELOG.md latest ($$changelog_version)"; \
	fi; \
	if [ $$errors -eq 0 ]; then \
		echo "✅ All version sources are consistent: $$cargo_version"; \
	else \
		echo ""; \
		echo "To fix version mismatches:"; \
		echo "  1. Update Cargo.toml: version = \"X.Y.Z\""; \
		echo "  2. Update CLI.md: footer: edgefirst-client X.Y.Z"; \
		echo "  3. Run: cargo check --workspace (updates Cargo.lock)"; \
		echo "  4. Update CHANGELOG.md with [X.Y.Z] entry"; \
		exit 1; \
	fi

