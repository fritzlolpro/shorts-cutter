use std::path::PathBuf;
use thiserror::Error;

/// Основной тип ошибки для всего приложения
#[derive(Error, Debug)]
pub enum ShortsCutterError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("File system error: {0}")]
    FileSystem(#[from] FileSystemError),
    
    #[error("FFmpeg error: {0}")]
    Ffmpeg(#[from] FfmpegError),
    
    #[error("Logging error: {0}")]
    Logging(#[from] LoggingError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Ошибки конфигурации и CLI аргументов
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Input directory does not exist: {path}")]
    InputDirectoryNotFound { path: PathBuf },
    
    #[error("Cannot create output directory: {path}")]
    OutputDirectoryCreationFailed { path: PathBuf },
    
    #[error("Invalid thread count: {count} (must be > 0 and <= {max})")]
    InvalidThreadCount { count: usize, max: usize },
    
    #[error("FFmpeg not found in PATH")]
    FfmpegNotFound,
    
    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },
}

/// Ошибки работы с файловой системой
#[derive(Error, Debug)]
pub enum FileSystemError {
    #[error("Cannot read directory: {path}")]
    CannotReadDirectory { path: PathBuf },
    
    #[error("Cannot access file: {path}")]
    CannotAccessFile { path: PathBuf },
    
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("Permission denied for path: {path}")]
    PermissionDenied { path: PathBuf },
    
    #[error("Disk full or insufficient space for output")]
    InsufficientSpace,
}

/// Ошибки выполнения FFmpeg
#[derive(Error, Debug)]
pub enum FfmpegError {
    #[error("FFmpeg execution failed with exit code {code}")]
    ExecutionFailed { 
        code: i32, 
        stderr: String,
        command: String,
    },
    
    #[error("FFmpeg process timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
    
    #[error("Invalid input file format: {path}")]
    InvalidInputFormat { path: PathBuf },
    
    #[error("Cannot spawn FFmpeg process")]
    CannotSpawnProcess,
    
    #[error("FFmpeg stderr parsing failed")]
    StderrParsingFailed,
}

/// Ошибки системы логирования
#[derive(Error, Debug)]
pub enum LoggingError {
    #[error("Cannot create log file: {path}")]
    CannotCreateLogFile { path: PathBuf },
    
    #[error("Cannot write to log file")]
    CannotWriteToLogFile,
    
    #[error("Log file initialization failed")]
    InitializationFailed,
    
    #[error("Log file error")]
    FileError,
}

/// Type aliases для упрощения использования
pub type Result<T> = std::result::Result<T, ShortsCutterError>;
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
pub type FileSystemResult<T> = std::result::Result<T, FileSystemError>;
pub type FfmpegResult<T> = std::result::Result<T, FfmpegError>;
pub type LoggingResult<T> = std::result::Result<T, LoggingError>;

/// Helper функции для создания ошибок с контекстом
impl ConfigError {
    pub fn input_not_found(path: PathBuf) -> Self {
        Self::InputDirectoryNotFound { path }
    }
    
    pub fn output_creation_failed(path: PathBuf) -> Self {
        Self::OutputDirectoryCreationFailed { path }
    }
    
    pub fn invalid_threads(count: usize, max: usize) -> Self {
        Self::InvalidThreadCount { count, max }
    }
    
    pub fn invalid_arg(message: impl Into<String>) -> Self {
        Self::InvalidArgument { message: message.into() }
    }
}

impl FfmpegError {
    pub fn execution_failed(code: i32, stderr: String, command: String) -> Self {
        Self::ExecutionFailed { code, stderr, command }
    }
    
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }
    
    pub fn invalid_format(path: PathBuf) -> Self {
        Self::InvalidInputFormat { path }
    }
}

impl FileSystemError {
    pub fn cannot_read_dir(path: PathBuf) -> Self {
        Self::CannotReadDirectory { path }
    }
    
    pub fn cannot_access(path: PathBuf) -> Self {
        Self::CannotAccessFile { path }
    }
    
    pub fn not_found(path: PathBuf) -> Self {
        Self::FileNotFound { path }
    }
    
    pub fn permission_denied(path: PathBuf) -> Self {
        Self::PermissionDenied { path }
    }
}

impl LoggingError {
    pub fn cannot_create_log(path: PathBuf) -> Self {
        Self::CannotCreateLogFile { path }
    }
}
