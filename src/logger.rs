use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{info, warn, error, debug};
use crate::error::{LoggingError, LoggingResult};

/// Инициализирует систему логирования
pub fn initialize_logging(log_file_path: PathBuf, _console_level: &str, _file_level: &str) -> LoggingResult<()> {
    // Простая версия логирования - пока используем только stdout
    let subscriber = tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|_| LoggingError::InitializationFailed)?;
    
    info!("Logging initialized. Log file: {}", log_file_path.display());
    
    Ok(())
}

/// Структура для отслеживания прогресса обработки файла
pub struct FileProcessingLogger {
    filename: String,
    start_time: Instant,
}

impl FileProcessingLogger {
    /// Создает новый экземпляр и логирует начало обработки файла
    pub fn start_processing(filename: &str) -> Self {
        let start_time = Instant::now();
        info!("START processing {}", filename);
        
        Self {
            filename: filename.to_string(),
            start_time,
        }
    }
    
    /// Логирует выполняемую FFmpeg команду
    pub fn log_ffmpeg_command(&self, command: &str) {
        info!("CMD: {}", command);
        debug!("FFmpeg command for {}: {}", self.filename, command);
    }
    
    /// Логирует успешное завершение обработки
    pub fn log_success(&self, input_path: &PathBuf, output_path: &PathBuf) {
        let duration = self.start_time.elapsed();
        info!(
            "SUCCESS: {} -> {} ({})",
            input_path.file_name().unwrap_or_default().to_string_lossy(),
            output_path.file_name().unwrap_or_default().to_string_lossy(),
            format_duration(duration)
        );
        debug!("File processing completed in {:?}: {} -> {}", 
               duration, input_path.display(), output_path.display());
    }
    
    /// Логирует ошибку обработки файла
    pub fn log_error(&self, input_path: &PathBuf, output_path: &PathBuf, error_message: &str) {
        let duration = self.start_time.elapsed();
        error!(
            "ERROR: {} -> {} ({})",
            input_path.file_name().unwrap_or_default().to_string_lossy(),
            output_path.file_name().unwrap_or_default().to_string_lossy(),
            format_duration(duration)
        );
        error!("ERRMSG: {}", error_message);
        debug!("File processing failed after {:?}: {} -> {}", 
               duration, input_path.display(), output_path.display());
    }
    
    /// Возвращает длительность обработки на текущий момент
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Структура для сбора и отображения финальной статистики
pub struct ProcessingSummary {
    pub total_files: usize,
    pub successful: usize,
    pub failed: usize,
    pub total_duration: Duration,
    pub successful_files: Vec<(PathBuf, PathBuf, Duration)>,
    pub failed_files: Vec<(PathBuf, String)>,
}

impl ProcessingSummary {
    /// Создает новую сводку
    pub fn new() -> Self {
        Self {
            total_files: 0,
            successful: 0,
            failed: 0,
            total_duration: Duration::ZERO,
            successful_files: Vec::new(),
            failed_files: Vec::new(),
        }
    }
    
    /// Добавляет успешно обработанный файл
    pub fn add_success(&mut self, input: PathBuf, output: PathBuf, duration: Duration) {
        self.successful += 1;
        self.successful_files.push((input, output, duration));
    }
    
    /// Добавляет файл с ошибкой
    pub fn add_failure(&mut self, input: PathBuf, error_message: String) {
        self.failed += 1;
        self.failed_files.push((input, error_message));
    }
    
    /// Устанавливает общую длительность обработки
    pub fn set_total_duration(&mut self, duration: Duration) {
        self.total_duration = duration;
        self.total_files = self.successful + self.failed;
    }
    
    /// Выводит финальный отчет в консоль и лог
    pub fn print_final_report(&self) {
        info!("=== PROCESSING COMPLETED ===");
        info!("Total files: {}", self.total_files);
        info!("Successful: {}", self.successful);
        info!("Failed: {}", self.failed);
        info!("Total time: {}", format_duration(self.total_duration));
        
        if !self.successful_files.is_empty() {
            info!("Successfully processed files:");
            for (input, output, duration) in &self.successful_files {
                info!("  ✓ {} -> {} ({})", 
                     input.file_name().unwrap_or_default().to_string_lossy(),
                     output.file_name().unwrap_or_default().to_string_lossy(),
                     format_duration(*duration));
            }
        }
        
        if !self.failed_files.is_empty() {
            warn!("Files with errors:");
            for (input, error) in &self.failed_files {
                error!("  ✗ {}: {}", 
                      input.file_name().unwrap_or_default().to_string_lossy(),
                      error);
            }
        }
        
        // Также выводим в консоль для пользователя
        println!("\n=== PROCESSING SUMMARY ===");
        println!("Total files processed: {}", self.total_files);
        println!("Successful: {} ✓", self.successful);
        println!("Failed: {} ✗", self.failed);
        println!("Total time: {}", format_duration(self.total_duration));
        
        if self.failed > 0 {
            println!("\nFiles with errors:");
            for (input, error) in &self.failed_files {
                println!("  ✗ {}: {}", 
                        input.file_name().unwrap_or_default().to_string_lossy(),
                        error);
            }
        }
        
        println!("Log details written to file.");
    }
    
    /// Возвращает соответствующий код выхода программы
    pub fn exit_code(&self) -> i32 {
        match (self.successful, self.failed) {
            (0, 0) => crate::config::exit_codes::CRITICAL_ERROR, // Не найдено файлов
            (_, 0) => crate::config::exit_codes::SUCCESS,        // Все успешно
            (0, _) => crate::config::exit_codes::CRITICAL_ERROR, // Все с ошибками
            (_, _) => crate::config::exit_codes::PARTIAL_SUCCESS, // Частичный успех
        }
    }
}

impl Default for ProcessingSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Логирует информацию о запуске приложения
pub fn log_startup_info(input_dir: &PathBuf, output_dir: &PathBuf, thread_count: usize) {
    info!("=== SHORTS CUTTER STARTED ===");
    info!("Input directory: {}", input_dir.display());
    info!("Output directory: {}", output_dir.display());
    info!("Thread count: {}", thread_count);
    info!("FFmpeg filter: {}", crate::config::FFMPEG_FILTER_COMPLEX);
}

/// Логирует информацию о найденных файлах
pub fn log_files_found(file_count: usize) {
    if file_count == 0 {
        warn!("{}", crate::config::messages::NO_FILES_FOUND);
    } else {
        info!("Found {} .mp4 files for processing", file_count);
    }
}

/// Логирует сигнал завершения
pub fn log_shutdown_signal() {
    warn!("{}", crate::config::messages::GRACEFUL_SHUTDOWN);
}

/// Форматирует duration в человекочитаемый вид
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = duration.subsec_millis();
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if seconds > 0 {
        format!("{}.{:03}s", seconds, millis)
    } else {
        format!("{}ms", millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(5)), "5.000s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }
    
    #[test]
    fn test_processing_summary() {
        let mut summary = ProcessingSummary::new();
        
        summary.add_success(
            PathBuf::from("test1.mp4"), 
            PathBuf::from("test1-short.mp4"),
            Duration::from_secs(10)
        );
        
        summary.add_failure(
            PathBuf::from("test2.mp4"),
            "FFmpeg error".to_string()
        );
        
        summary.set_total_duration(Duration::from_secs(30));
        
        assert_eq!(summary.successful, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.total_files, 2);
        assert_eq!(summary.exit_code(), crate::config::exit_codes::PARTIAL_SUCCESS);
    }
}
