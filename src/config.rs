use std::time::Duration;

/// Центральная конфигурация приложения
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Фильтр для FFmpeg (захардкожен)
    pub ffmpeg_filter_complex: String,
    
    /// Поддерживаемые расширения входных файлов
    pub supported_extensions: Vec<String>,
    
    /// Суффикс для выходных файлов
    pub output_suffix: String,
    
    /// Максимальное время выполнения FFmpeg для одного файла
    pub ffmpeg_timeout: Duration,
    
    /// Максимальное количество потоков
    pub max_threads: usize,
    
    /// Паттерн имени лог-файла
    pub log_filename_pattern: String,
    
    /// Уровень логирования для консоли
    pub console_log_level: String,
    
    /// Уровень логирования для файла
    pub file_log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ffmpeg_filter_complex: FFMPEG_FILTER_COMPLEX.to_string(),
            supported_extensions: vec!["mp4".to_string()],
            output_suffix: OUTPUT_SUFFIX.to_string(),
            ffmpeg_timeout: FFMPEG_TIMEOUT,
            max_threads: MAX_THREADS,
            log_filename_pattern: LOG_FILENAME_PATTERN.to_string(),
            console_log_level: "info".to_string(),
            file_log_level: "debug".to_string(),
        }
    }
}

impl AppConfig {
    /// Создает новую конфигурацию с значениями по умолчанию
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Валидирует конфигурацию
    pub fn validate(&self) -> crate::error::ConfigResult<()> {
        if self.supported_extensions.is_empty() {
            return Err(crate::error::ConfigError::invalid_arg(
                "No supported file extensions configured"
            ));
        }
        
        if self.output_suffix.is_empty() {
            return Err(crate::error::ConfigError::invalid_arg(
                "Output suffix cannot be empty"
            ));
        }
        
        if self.ffmpeg_filter_complex.is_empty() {
            return Err(crate::error::ConfigError::invalid_arg(
                "FFmpeg filter complex cannot be empty"
            ));
        }
        
        Ok(())
    }
    
    /// Определяет количество потоков по умолчанию (количество CPU ядер)
    pub fn default_thread_count() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }
    
    /// Генерирует имя лог-файла на основе текущего времени
    pub fn generate_log_filename() -> String {
        let now = chrono::Local::now();
        now.format("shorts-cutter-%Y%m%d-%H%M%S.log").to_string()
    }
}

// Константы приложения

/// Жестко заданная FFmpeg команда для создания вертикальных шортсов
pub const FFMPEG_FILTER_COMPLEX: &str = 
    "[0:v]scale=2276:1280,boxblur=4[bg];[1:v]scale=720:-1[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2[tmp];[tmp]crop=720:1280:(2276-720)/2:0[out]";

/// Суффикс для выходных файлов
pub const OUTPUT_SUFFIX: &str = "-short";

/// Максимальное время выполнения FFmpeg для одного файла (10 минут)
pub const FFMPEG_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Максимально допустимое количество потоков
pub const MAX_THREADS: usize = 32;

/// Паттерн имени лог-файла
pub const LOG_FILENAME_PATTERN: &str = "shorts-cutter-%Y%m%d-%H%M%S.log";

/// Имя исполняемого файла FFmpeg
pub const FFMPEG_EXECUTABLE: &str = "ffmpeg";

/// Аргументы FFmpeg для проверки версии
pub const FFMPEG_VERSION_ARGS: &[&str] = &["-version"];

/// Буферный размер для чтения stdout/stderr FFmpeg
pub const FFMPEG_BUFFER_SIZE: usize = 8192;

/// Расширения файлов для поиска (в нижнем регистре)
pub const DEFAULT_INPUT_EXTENSIONS: &[&str] = &["mp4"];

/// Коды возврата приложения
pub mod exit_codes {
    /// Успешное завершение
    pub const SUCCESS: i32 = 0;
    
    /// Критическая ошибка (неправильные аргументы, FFmpeg не найден и т.д.)
    pub const CRITICAL_ERROR: i32 = 1;
    
    /// Частичный успех (некоторые файлы обработаны с ошибками)
    pub const PARTIAL_SUCCESS: i32 = 2;
}

/// Сообщения для пользователя
pub mod messages {
    pub const FFMPEG_NOT_FOUND: &str = 
        "FFmpeg not found in PATH. Please install FFmpeg and ensure it's available in your system PATH.";
    
    pub const PROCESSING_STARTED: &str = "Starting video processing...";
    
    pub const PROCESSING_COMPLETED: &str = "Video processing completed.";
    
    pub const NO_FILES_FOUND: &str = "No .mp4 files found in the input directory.";
    
    pub const GRACEFUL_SHUTDOWN: &str = "Received shutdown signal. Finishing current tasks...";
}
