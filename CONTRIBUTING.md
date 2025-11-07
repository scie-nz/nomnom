# Contributing to Nomnom

Thank you for your interest in contributing to Nomnom! This document provides guidelines for contributing to the project.

## Development Setup

### Prerequisites

- Rust 1.70+ (with cargo)
- Python 3.8+ (optional, for Python bridge feature)

### Getting Started

1. Clone the repository:
```bash
git clone https://github.com/scie-nz/nomnom.git
cd nomnom
```

2. Build the project:
```bash
cargo build
```

3. Run tests:
```bash
cargo test
```

4. Format code:
```bash
cargo fmt
```

## Development Workflow

### Code Style

- **Rust**: Follow standard Rust conventions. Run `cargo fmt` before committing.
- **Comments**: Use clear, concise comments. Explain "why", not "what".
- **Documentation**: Update relevant documentation for new features.

### Testing

- All new features must include tests
- Run `cargo test` to ensure all tests pass
- Add integration tests for complex features

### Commit Messages

Use clear, descriptive commit messages:
- Start with a verb in present tense (Add, Fix, Update, Remove)
- Keep the first line under 72 characters
- Provide additional context in the body if needed

Example:
```
Add CSV field extraction transform

Implements a new transform for extracting fields from CSV rows
by column index. Includes support for quoted fields and custom
delimiters.
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests and linting (`make test && make lint`)
5. Commit your changes
6. Push to your fork
7. Open a Pull Request

### PR Guidelines

- Provide a clear description of the changes
- Reference any related issues
- Ensure all CI checks pass
- Request review from maintainers

## Areas for Contribution

### High Priority

- Additional transform implementations
- More data format examples (XML, Parquet, Avro)
- Performance optimizations
- Documentation improvements

### Medium Priority

- Additional serialization formats
- Enhanced error messages
- CLI tool improvements

### Good First Issues

Check the GitHub issues labeled "good first issue" for beginner-friendly tasks.

## Questions?

Feel free to open an issue for:
- Bug reports
- Feature requests
- Documentation clarifications
- General questions

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the technical merits of contributions

## License

By contributing to Nomnom, you agree that your contributions will be licensed under the MIT License.
