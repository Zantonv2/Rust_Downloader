# 🎵 Rust Music Downloader

A powerful, cross-platform music downloader built with Rust and featuring a modern GUI. Download music from various sources with automatic metadata embedding, cover art, and lyrics support.

## ✨ Features

- **Multi-Source Support**: YouTube, Spotify, SoundCloud, and more
- **Modern GUI**: Built with Iced framework for a native look and feel
- **Smart Metadata**: Automatic title, artist, album, and cover art embedding
- **SponsorBlock Integration**: Remove unwanted segments from videos
- **Multiple Formats**: Support for MP3, M4A, FLAC, and more
- **Lyrics Support**: Automatic lyrics downloading and embedding
- **Batch Processing**: Download entire playlists or CSV lists
- **Proxy Support**: Built-in proxy configuration
- **Cross-Platform**: Works on Windows, macOS, and Linux

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- FFmpeg (for audio conversion)
- yt-dlp (for YouTube downloads)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/rust-music-downloader.git
cd rust-music-downloader
```

2. Install dependencies:
```bash
cargo build --release
```

3. Run the application:
```bash
cargo run --release
```

## ⚙️ Configuration

The application will create a `settings.json` file on first run. You can configure:

- **API Keys**: Spotify, Last.fm, Genius, Musixmatch
- **Download Settings**: Format, quality, output directory
- **SponsorBlock**: Configure which segments to remove
- **Proxy Settings**: HTTP/SOCKS proxy support
- **UI Preferences**: Theme, window size, concurrent downloads

### Required API Keys

- **Spotify**: Get from [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
- **Last.fm** (optional): For additional metadata
- **Genius** (optional): For lyrics
- **Musixmatch** (optional): For lyrics

## 🎯 Usage

### GUI Mode (Default)
```bash
cargo run
```

### CLI Mode
```bash
# Download a single track
cargo run -- --url "https://www.youtube.com/watch?v=VIDEO_ID"

# Download from Spotify playlist
cargo run -- --spotify-playlist "PLAYLIST_ID"

# Import from CSV
cargo run -- --csv "tracks.csv"
```

## 🏗️ Architecture

```
src/
├── main.rs              # Application entry point
├── cli.rs               # Command-line interface
├── config.rs            # Configuration structures
├── settings.rs          # Settings management
├── errors.rs            # Error handling
├── downloader/          # Download strategies
│   ├── youtube.rs       # YouTube downloads
│   ├── spotify.rs       # Spotify integration
│   ├── soundcloud.rs    # SoundCloud support
│   └── metadata.rs      # Metadata embedding
├── lyrics/              # Lyrics handling
├── ui/                  # GUI components
└── utils/               # Utility functions
```

## 📦 Dependencies

- **Iced**: Modern GUI framework
- **Tokio**: Async runtime
- **Reqwest**: HTTP client
- **Serde**: Serialization
- **Lofty**: Audio metadata
- **Symphonia**: Audio processing

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

This tool is for educational purposes only. Please respect copyright laws and terms of service of the platforms you download from. Only download content you have the right to access.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📞 Support

If you encounter any issues or have questions, please open an issue on GitHub.

---

**Note**: This project is not affiliated with Spotify, YouTube, or any other music platform.
