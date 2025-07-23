# Shorts Cutter

[![Rust](https://img.shields.io/badge/rust-1.84+-orange.svg)](https://www.rust-lang.org)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

> 🇷🇺 [Русская версия README](README_RU.md)

**Shorts Cutter** is a high-performance Rust CLI tool for batch video processing that converts horizontal MP4 videos into vertical short-form videos optimized for social media platforms like TikTok, Instagram Reels, and YouTube Shorts.

## 🎯 Features

- **Batch Processing**: Process multiple MP4 files simultaneously
- **Parallel Execution**: Configurable multi-threaded processing using async workers
- **Smart Video Conversion**: Converts horizontal videos to vertical format (720x1280) with:
  - Blurred background overlay
  - Centered original video
  - Preserved audio track
- **Comprehensive Logging**: Detailed timestamped logs for monitoring and debugging
- **Error Resilience**: Individual file failures don't interrupt batch processing
- **Cross-Platform**: Works on Windows, macOS, and Linux

## 🛠️ Prerequisites

### FFmpeg Installation

This tool requires **FFmpeg** to be installed and available in your system PATH.

#### Windows
```bash
# Using Chocolatey
choco install ffmpeg

# Using Scoop
scoop install ffmpeg

# Or download from: https://ffmpeg.org/download.html#build-windows
```

#### macOS
```bash
# Using Homebrew
brew install ffmpeg

# Using MacPorts
sudo port install ffmpeg
```

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install ffmpeg
```

#### Linux (CentOS/RHEL/Fedora)
```bash
# CentOS/RHEL
sudo yum install ffmpeg

# Fedora
sudo dnf install ffmpeg
```

### Rust Installation

Install Rust from [rustup.rs](https://rustup.rs/) or use your package manager:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## 🚀 Installation

### From Source

1. **Clone the repository:**
   ```bash
   git clone https://github.com/yourusername/shorts-cutter.git
   cd shorts-cutter
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **Run the executable:**
   ```bash
   ./target/release/shorts-cutter --help
   ```

### Development Mode

For development and testing:

```bash
cargo run -- --input ./input --output ./output --threads 4
```

## 📖 Usage

### Basic Usage

```bash
shorts-cutter --input <INPUT_DIR> --output <OUTPUT_DIR> [--threads <THREADS>]
```

### Command Line Arguments

| Argument | Short | Description | Default |
|----------|-------|-------------|---------|
| `--input` | `-i` | Input directory containing MP4 files | Required |
| `--output` | `-o` | Output directory for processed videos | Required |
| `--threads` | `-t` | Number of parallel processing threads | CPU cores |
| `--help` | `-h` | Show help information | - |

### Examples

**Process videos with default thread count:**
```bash
shorts-cutter --input ./my-videos --output ./shorts
```

**Process with specific thread count:**
```bash
shorts-cutter --input ./videos --output ./output --threads 8
```

**Using short argument names:**
```bash
shorts-cutter -i ./input -o ./output -t 4
```

## 🎬 Video Processing Details

### Recommended Workflow
For best results, combine Shorts Cutter with other tools in your video editing pipeline:

1. **Extract highlights** from long-form videos using [LosslessCut](https://github.com/mifi/lossless-cut)
   - Quickly identify and cut interesting moments
   - Create multiple short clips from a single source video
   - Maintain original quality with lossless cutting

2. **Batch convert to shorts** using Shorts Cutter
   - Process all extracted clips simultaneously
   - Apply consistent vertical format and blurred background
   - Generate social media-ready content at scale

This two-step approach allows you to efficiently transform lengthy content into engaging short-form videos optimized for platforms like TikTok, Instagram Reels, and YouTube Shorts.

### Input Requirements
- **Format**: MP4 files only
- **Location**: All MP4 files in the input directory (searched recursively)
- **Validation**: Files are validated before processing

### Output Format
- **Resolution**: 720x1280 (vertical)
- **Background**: Blurred version of the original video (scaled to 2276x1280)
- **Foreground**: Original video centered and scaled to fit vertically
- **Audio**: Original audio track preserved
- **Naming**: `<original-name>-short.mp4`

### FFmpeg Filter Chain
The tool uses the following FFmpeg filter complex:
```bash
[0:v]scale=2276:1280,boxblur=4[bg];[1:v]scale=720:-1[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2[tmp];[tmp]crop=720:1280:(2276-720)/2:0[out]
```

## 📊 Logging and Monitoring

### Log Files
- **Location**: Output directory
- **Format**: `shorts-cutter-YYYYMMDD-HHMMSS.log`
- **Content**: Detailed processing information, timestamps, errors

### Console Output
Real-time processing information including:
- File discovery results
- Processing progress
- Success/failure summaries
- Error details

### Sample Output
```
Configuration:
  Input directory:  /path/to/input
  Output directory: /path/to/output
  Threads:          4
  Log file:         /path/to/output/shorts-cutter-20250723-120000.log

Starting video processing...
Found 15 files to process
Using 4 parallel threads

=== PROCESSING SUMMARY ===
Total files processed: 15
Successful: 12 ✓
Failed: 3 ✗
Total time: 2m 34.567s

Files with errors:
  ✗ corrupted_video.mp4: FFmpeg error: Invalid data found
  ✗ empty_file.mp4: FFmpeg error: No such file or directory
  ✗ unsupported.mp4: FFmpeg error: Unsupported codec

Log details written to file.
Video processing completed.
```

## 🔧 Configuration

### Default Settings
- **Thread Count**: Number of CPU cores
- **FFmpeg Timeout**: 300 seconds per file
- **Supported Extensions**: `.mp4`
- **Output Suffix**: `-short`

### Customization
For advanced customization, modify the constants in `src/config.rs`:

```rust
pub const FFMPEG_FILTER_COMPLEX: &str = "[0:v]scale=2276:1280,boxblur=4[bg];[1:v]scale=720:-1[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2[tmp];[tmp]crop=720:1280:(2276-720)/2:0[out]";
pub const FFMPEG_TIMEOUT: Duration = Duration::from_secs(300);
pub const DEFAULT_INPUT_EXTENSIONS: &[&str] = &["mp4"];
```

## 🛡️ Error Handling

The tool is designed to be resilient:

- **File-level failures**: Individual file errors don't stop batch processing
- **Detailed error reporting**: Specific error messages for troubleshooting
- **Graceful degradation**: Continues processing remaining files after failures
- **Comprehensive logging**: All errors logged with context

### Common Error Scenarios
- **FFmpeg not found**: Ensure FFmpeg is installed and in PATH
- **Invalid input files**: Corrupted or unsupported video files
- **Insufficient disk space**: Check available storage in output directory
- **Permission issues**: Ensure read/write permissions for directories

## 🏗️ Project Structure

```
shorts-cutter/
├── src/
│   ├── main.rs          # Main application entry point
│   ├── cli.rs           # Command-line argument parsing
│   ├── config.rs        # Configuration and constants
│   ├── error.rs         # Error types and handling
│   ├── ffmpeg.rs        # FFmpeg integration
│   ├── logger.rs        # Logging system
│   ├── utils.rs         # File utilities
│   └── worker.rs        # Parallel processing
├── doc/
│   └── SHORTS-CUTTER-DESIGN.md  # Technical specifications
├── Cargo.toml           # Project dependencies
└── README.md            # This file
```

## 📦 Dependencies

### Runtime Dependencies
- **[clap](https://crates.io/crates/clap)** - Command line argument parsing
- **[tokio](https://crates.io/crates/tokio)** - Async runtime and parallelism
- **[tracing](https://crates.io/crates/tracing)** - Structured logging
- **[tracing-subscriber](https://crates.io/crates/tracing-subscriber)** - Log output formatting
- **[thiserror](https://crates.io/crates/thiserror)** - Error handling

### Development Dependencies
- **[tempfile](https://crates.io/crates/tempfile)** - Testing utilities

## 🧪 Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test ffmpeg

# Run with release optimizations
cargo test --release
```

### Test Coverage
- Unit tests for core functionality
- Integration tests for FFmpeg commands
- File system operation tests
- Error handling validation

## 🚀 Performance

### Optimization Tips
1. **Thread Count**: Set `--threads` to match your CPU cores for optimal performance
2. **SSD Storage**: Use SSD storage for input/output directories
3. **Memory**: Ensure sufficient RAM for parallel processing
4. **FFmpeg Version**: Use the latest FFmpeg version for best performance

### Benchmarks
Performance varies based on:
- Input video resolution and bitrate
- System specifications (CPU, storage, memory)
- Thread configuration

Example performance on a modern system (8-core CPU, SSD storage):
- **1080p videos**: ~30-60 seconds per file
- **Batch of 50 videos**: ~15-25 minutes with 8 threads

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines:

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make your changes** and add tests
4. **Run the test suite**: `cargo test`
5. **Commit your changes**: `git commit -m 'Add amazing feature'`
6. **Push to the branch**: `git push origin feature/amazing-feature`
7. **Open a Pull Request**

### Development Setup

```bash
# Clone your fork
git clone https://github.com/yourusername/shorts-cutter.git
cd shorts-cutter

# Create a development build
cargo build

# Run tests
cargo test

# Run with sample data
mkdir test-input test-output
# Add sample MP4 files to test-input/
cargo run -- --input test-input --output test-output --threads 2
```

## 📜 License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/shorts-cutter/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/shorts-cutter/discussions)
- **Documentation**: See `doc/SHORTS-CUTTER-DESIGN.md` for technical details

## 📈 Roadmap

### Upcoming Features
- [ ] Support for additional input formats (AVI, MOV, MKV)
- [ ] Customizable output resolutions
- [ ] GPU acceleration support
- [ ] Progress bars and real-time status
- [ ] Configuration file support
- [ ] Web interface for batch management

### Version History
- **v0.1.0** - Initial release with core functionality
  - MP4 batch processing
  - Parallel execution
  - Comprehensive logging
  - Cross-platform support

---

## 🏷️ Keywords

`rust` `cli` `ffmpeg` `video-processing` `batch-processing` `shorts` `tiktok` `instagram-reels` `youtube-shorts` `async` `parallel-processing` `video-conversion`

---

**Made with ❤️ in Rust**
