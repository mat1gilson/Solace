# Contributing to Solace Protocol

We welcome contributions from the community! This document provides guidelines for contributing to the Solace Protocol project.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Create a feature branch** from `main`
4. **Make your changes** and commit them
5. **Push to your fork** and submit a pull request

## Development Setup

### Prerequisites

- Node.js 18+ and npm/yarn
- Rust 1.70+ and Cargo
- Python 3.9+ and pip
- Solana CLI tools

### Local Development

```bash
# Clone the repository
git clone https://github.com/your-username/solace-protocol.git
cd solace-protocol

# Install dependencies
npm install

# Build the project
npm run build

# Run tests
npm test
```

## Code Style

### Rust
- Follow Rust standard formatting (`cargo fmt`)
- Use Clippy for linting (`cargo clippy`)
- Write comprehensive tests for new functionality

### TypeScript
- Use ESLint and Prettier for formatting
- Follow TypeScript best practices
- Write unit tests for all new features

### Python
- Follow PEP 8 style guidelines
- Use Black for formatting
- Use type hints where appropriate
- Write docstrings for all public functions

## Pull Request Process

1. **Update documentation** if needed
2. **Add tests** for new functionality
3. **Ensure all tests pass** locally
4. **Update the README.md** if needed
5. **Submit a clear pull request** with:
   - Description of changes
   - Related issue numbers
   - Screenshots (if UI changes)

## Issue Guidelines

When filing issues:

- **Use clear, descriptive titles**
- **Provide detailed reproduction steps** for bugs
- **Include relevant logs and error messages**
- **Tag appropriately** (bug, feature, documentation, etc.)

## Security

For security vulnerabilities, please email security@solaceprotocol.com instead of filing a public issue.

## License

By contributing, you agree that your contributions will be licensed under the MIT License. 