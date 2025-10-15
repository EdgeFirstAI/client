# Contributing to EdgeFirst Client

Thank you for your interest in contributing to EdgeFirst Client! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

## How to Contribute

### Reporting Bugs

Before creating a bug report:
1. Check the [existing issues](https://github.com/EdgeFirstAI/client/issues) to avoid duplicates
2. Gather relevant information (version, OS, error messages, etc.)

Use the bug report template when creating an issue.

### Suggesting Features

Feature requests are welcome! Please:
1. Check if the feature has already been suggested
2. Provide clear use cases
3. Explain how it benefits users

Use the feature request template when creating an issue.

### Contributing Code

#### Development Setup

**Prerequisites:**
- Rust 1.90 or later
- Python 3.8 or later
- Git

**Clone and build:**

```bash
git clone https://github.com/EdgeFirstAI/client.git
cd client
cargo build
```

**Install Python dependencies:**

```bash
pip install -r requirements.txt
pip install maturin
maturin develop -m crates/edgefirst-client-py/Cargo.toml
```

#### Making Changes

1. **Fork the repository** and create a branch from `main`:
   ```bash
   git checkout -b feature/my-feature
   # or
   git checkout -b fix/my-bugfix
   ```

2. **Make your changes** following the coding standards below

3. **Add tests** for your changes:
   - Rust: Add tests in the same file or in `tests/` directory
   - Python: Add tests to test files

4. **Run tests** to ensure everything works:
   ```bash
   # Rust tests
   cargo test
   cargo test --doc
   
   # Python tests
   python -m unittest
   ```

5. **Format your code:**
   ```bash
   cargo fmt --all
   ```

6. **Run clippy:**
   ```bash
   cargo clippy --all-targets --all-features
   ```

7. **Update documentation** if needed:
   - Add/update doc comments in code
   - Update README.md for significant changes
   - Update CHANGELOG.md

8. **Commit your changes** with a clear, descriptive message:
   ```bash
   git commit -m "Add new feature X
   
   - Detailed description of changes
   - Related to #123"
   ```

9. **Push to your fork:**
   ```bash
   git push origin feature/my-feature
   ```

10. **Create a Pull Request** using the PR template

#### Code Review Process

- Maintainers will review your PR
- Address any feedback or requested changes
- Once approved, a maintainer will merge your PR

## Coding Standards

### Rust Code

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Fix all `cargo clippy` warnings
- Add doc comments for public APIs
- Include examples in doc comments when helpful
- Write unit tests for new functionality

### Python Code

- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/)
- Use type hints where possible
- Add docstrings for public functions and classes
- Write unit tests for new functionality

### Documentation

- Use clear, concise language
- Include code examples
- Update CHANGELOG.md for user-facing changes
- Keep README.md up to date

### Git Commits

- Write clear, descriptive commit messages
- Reference issue numbers when applicable
- Keep commits focused and atomic
- Explain the "why" behind changes in commit messages

## Project Structure

```
client/
├── crates/
│   ├── edgefirst-client/       # Core Rust library
│   ├── edgefirst-cli/          # CLI application
│   └── edgefirst-client-py/    # Python bindings
├── .github/
│   └── workflows/              # GitHub Actions
├── testdata/                   # Test data files
├── Cargo.toml                  # Workspace manifest
└── README.md
```

## Testing

### Running Tests

**All tests:**
```bash
cargo test
```

**Specific test:**
```bash
cargo test test_name
```

**With output:**
```bash
cargo test -- --nocapture
```

**Doc tests:**
```bash
cargo test --doc
```

**Python tests:**
```bash
python -m unittest
```

### Test Coverage

Generate coverage report:
```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

## Building

### CLI Binary

```bash
cargo build --release -p edgefirst-cli
```

### Python Wheel

```bash
maturin build --release -m crates/edgefirst-client-py/Cargo.toml
```

## Documentation

### Generate Rust Docs

```bash
cargo doc --open
```

### API Documentation

Rust documentation is automatically published to [docs.rs](https://docs.rs/edgefirst-client) on release.

## Versioning

This project follows [Semantic Versioning](https://semver.org/) (SemVer) with the following format:

- **Stable releases**: `X.Y.Z` (e.g., `1.0.0`, `2.1.5`)
- **Release candidates**: `X.Y.ZrcN` (e.g., `1.0.0rc1`, `2.0.0rc2`)
- **Alpha releases**: `X.Y.ZaN` (e.g., `0.1.0a1`, `1.0.0a2`)
- **Beta releases**: `X.Y.ZbN` (e.g., `0.1.0b1`, `1.0.0b2`)

**Important**: Use the format without separators (e.g., `1.0.0rc1`, not `1.0.0-rc.1`) to ensure compatibility with both:
- Python's PEP 440 standard (for PyPI publishing)
- Rust's Cargo/SemVer standard (for crates.io publishing)

## Release Process

Releases are managed by maintainers:

1. Update version in `Cargo.toml` to the new release version
2. Update `CHANGELOG.md` with release notes
3. Commit changes: `git commit -am "Prepare release 1.0.0"`
4. Push changes: `git push`
5. Create a git tag matching the version: `git tag -a 1.0.0 -m "Release 1.0.0"`
6. Push tag: `git push origin 1.0.0`
7. GitHub Actions will automatically:
   - Verify the tag matches the Cargo.toml version
   - Build binaries for all platforms
   - Build Python wheels
   - Publish to crates.io
   - Publish to PyPI
   - Create GitHub Release with all artifacts

**Important**: The version in `Cargo.toml` must exactly match the git tag, or the release workflow will fail.

## Getting Help

- Open an [issue](https://github.com/EdgeFirstAI/client/issues) for questions
- Check existing documentation
- Review closed issues and PRs

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.

## Recognition

Contributors will be recognized in release notes and the project's contributors list.

Thank you for contributing to EdgeFirst Client!
