# Contributing to Rust Music Downloader

Thank you for your interest in contributing! Here are some guidelines to help you get started.

## 🚀 Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/rust-music-downloader.git`
3. Create a feature branch: `git checkout -b feature/amazing-feature`
4. Make your changes
5. Commit your changes: `git commit -m 'Add amazing feature'`
6. Push to the branch: `git push origin feature/amazing-feature`
7. Open a Pull Request

## 📝 Code Style

- Follow Rust conventions and use `cargo fmt`
- Run `cargo clippy` to check for common issues
- Add documentation for public APIs
- Write tests for new functionality

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## 📋 Pull Request Process

1. Ensure your code compiles without warnings
2. Add tests for new functionality
3. Update documentation if needed
4. Ensure all tests pass
5. Describe your changes in the PR description

## 🐛 Reporting Issues

When reporting issues, please include:

- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Any error messages or logs

## 💡 Feature Requests

We welcome feature requests! Please:

- Check existing issues first
- Provide a clear description
- Explain the use case
- Consider implementation complexity

## 🏗️ Development Setup

1. Install Rust: https://rustup.rs/
2. Install FFmpeg for audio processing
3. Install yt-dlp for YouTube downloads
4. Clone and build: `cargo build`

## 📚 Code Organization

- `src/main.rs` - Application entry point
- `src/cli.rs` - Command-line interface
- `src/downloader/` - Download strategies
- `src/ui/` - GUI components
- `src/lyrics/` - Lyrics handling
- `src/utils/` - Utility functions

## 🤝 Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Follow the golden rule

Thank you for contributing! 🎵
