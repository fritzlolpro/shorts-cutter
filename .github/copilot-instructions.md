# Copilot Instructions for Shorts Cutter

## Project Overview
**Shorts Cutter** is a Rust CLI tool for batch video processing using FFmpeg. It processes `.mp4` files from an input directory, applies a hardcoded FFmpeg filter complex for creating vertical shorts, and outputs processed files with detailed logging.

## Architecture & Key Concepts

### Core Components (to be implemented)
- **CLI Interface**: Uses clap for argument parsing (`--input`, `--output`, `--threads`)
- **File Processing**: Async worker pool using Tokio for parallel FFmpeg execution
- **Logging**: Centralized timestamped logging to `shorts-cutter-YYYYMMDD-HHMMSS.log`
- **Error Handling**: Resilient processing where individual file errors don't stop the batch

### FFmpeg Command Template
The application uses a fixed FFmpeg filter complex for creating vertical shorts:
```bash
ffmpeg -i <infile> -i <infile> -filter_complex "[0:v]scale=2276:1280,boxblur=4[bg];[1:v]scale=720:-1[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2[tmp];[tmp]crop=720:1280:(2276-720)/2:0[out]" -map "[out]" -map 0:a <outfile>
```

### File Naming Convention
- Input: `*.mp4` files from input directory
- Output: `<source_name>-short.mp4` in output directory

## Development Guidelines

### Dependencies to Add
Based on `doc/SHORTS-CUTTER-DESIGN.md`, implement with:
- `clap` for CLI argument parsing
- `tokio` for async runtime and worker pool
- `log` + `env_logger` or `tracing` for structured logging
- `chrono` for timestamp formatting

### Project Structure (planned)
```
src/
├── main.rs          # Entry point, CLI setup, main execution flow
├── cli.rs           # Command line argument definitions
├── processor.rs     # Core video processing logic
├── worker.rs        # Async worker pool implementation  
├── logger.rs        # Logging utilities and file management
└── ffmpeg.rs        # FFmpeg command building and execution
```

### Key Implementation Patterns
1. **Error Resilience**: Individual file failures must not stop batch processing
2. **Structured Logging**: Each operation logs start, command, and result with timestamps
3. **Resource Management**: Configurable thread pool size (default: available cores)
4. **Path Validation**: Ensure FFmpeg availability before processing starts

### Testing Strategy
- Unit tests for CLI parsing and file path handling
- Integration tests with sample video files
- Mock FFmpeg execution for CI environments
- Test error scenarios (missing files, invalid paths, FFmpeg failures)

### Build & Run
```bash
# Development
cargo run -- --input ./test-clips --output ./output --threads 2

# Release build
cargo build --release

# Run tests
cargo test
```

### Critical Implementation Notes
- Validate FFmpeg availability in PATH before starting
- Use async/await pattern for file processing pipeline
- Implement graceful shutdown handling for long-running batches
- Log FFmpeg stderr output for debugging failed conversions
- Ensure output directory exists before processing

Refer to `doc/SHORTS-CUTTER-DESIGN.md` for detailed specifications and examples.
