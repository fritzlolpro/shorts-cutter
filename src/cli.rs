use clap::Parser;
use std::path::PathBuf;
use crate::config::AppConfig;
use crate::error::{ConfigError, ConfigResult};

/// CLI tool for batch video processing using FFmpeg
#[derive(Parser, Debug)]
#[command(
    name = "shorts-cutter",
    version = env!("CARGO_PKG_VERSION"),
    about = "Batch video processing tool for creating vertical shorts",
    long_about = None
)]
pub struct CliArgs {
    /// Input directory containing .mp4 files
    #[arg(short, long, value_name = "DIR")]
    pub input: PathBuf,
    
    /// Output directory for processed files
    #[arg(short, long, value_name = "DIR")]
    pub output: PathBuf,
    
    /// Number of parallel processing threads
    #[arg(
        short, 
        long, 
        value_name = "COUNT",
        help = "Number of parallel threads (default: CPU cores)"
    )]
    pub threads: Option<usize>,
}

impl CliArgs {
    /// Парсит аргументы командной строки
    pub fn parse_args() -> Self {
        Self::parse()
    }
    
    /// Валидирует аргументы и возвращает нормализованную конфигурацию
    pub fn validate_and_normalize(self) -> ConfigResult<ValidatedArgs> {
        // Проверяем существование input директории
        if !self.input.exists() {
            return Err(ConfigError::input_not_found(self.input));
        }
        
        if !self.input.is_dir() {
            return Err(ConfigError::invalid_arg(
                format!("Input path is not a directory: {}", self.input.display())
            ));
        }
        
        // Нормализуем пути
        let input = self.input.canonicalize()
            .map_err(|_| ConfigError::invalid_arg(
                format!("Cannot resolve input path: {}", self.input.display())
            ))?;
        
        let output = if self.output.exists() {
            if !self.output.is_dir() {
                return Err(ConfigError::invalid_arg(
                    format!("Output path exists but is not a directory: {}", self.output.display())
                ));
            }
            self.output.canonicalize()
                .map_err(|_| ConfigError::invalid_arg(
                    format!("Cannot resolve output path: {}", self.output.display())
                ))?
        } else {
            // Создаем output директорию если она не существует
            std::fs::create_dir_all(&self.output)
                .map_err(|_| ConfigError::output_creation_failed(self.output.clone()))?;
            
            self.output.canonicalize()
                .map_err(|_| ConfigError::output_creation_failed(self.output))?
        };
        
        // Валидируем количество потоков
        let threads = match self.threads {
            Some(count) => {
                if count == 0 {
                    return Err(ConfigError::invalid_threads(count, crate::config::MAX_THREADS));
                }
                if count > crate::config::MAX_THREADS {
                    return Err(ConfigError::invalid_threads(count, crate::config::MAX_THREADS));
                }
                count
            }
            None => AppConfig::default_thread_count(),
        };
        
        Ok(ValidatedArgs {
            input,
            output,
            threads,
        })
    }
}

/// Валидированные и нормализованные аргументы CLI
#[derive(Debug, Clone)]
pub struct ValidatedArgs {
    /// Абсолютный путь к input директории
    pub input: PathBuf,
    
    /// Абсолютный путь к output директории  
    pub output: PathBuf,
    
    /// Количество потоков для обработки
    pub threads: usize,
}

impl ValidatedArgs {
    /// Проверяет доступность FFmpeg в системе
    pub async fn check_ffmpeg_availability(&self) -> ConfigResult<()> {
        use tokio::process::Command;
        
        let output = Command::new(crate::config::FFMPEG_EXECUTABLE)
            .args(crate::config::FFMPEG_VERSION_ARGS)
            .output()
            .await
            .map_err(|_| ConfigError::FfmpegNotFound)?;
        
        if !output.status.success() {
            return Err(ConfigError::FfmpegNotFound);
        }
        
        Ok(())
    }
    
    /// Генерирует полный путь к лог-файлу
    pub fn log_file_path(&self) -> PathBuf {
        let log_filename = AppConfig::generate_log_filename();
        self.output.join(log_filename)
    }
    
    /// Выводит информацию о конфигурации
    pub fn print_config_info(&self) {
        println!("Configuration:");
        println!("  Input directory:  {}", self.input.display());
        println!("  Output directory: {}", self.output.display());
        println!("  Threads:          {}", self.threads);
        println!("  Log file:         {}", self.log_file_path().display());
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_thread_count_validation() {
        let temp_input = TempDir::new().unwrap();
        let temp_output = TempDir::new().unwrap();
        
        // Тест с нулевым количеством потоков
        let args = CliArgs {
            input: temp_input.path().to_path_buf(),
            output: temp_output.path().to_path_buf(),
            threads: Some(0),
        };
        
        assert!(args.validate_and_normalize().is_err());
        
        // Тест с слишком большим количеством потоков
        let args = CliArgs {
            input: temp_input.path().to_path_buf(),
            output: temp_output.path().to_path_buf(),
            threads: Some(crate::config::MAX_THREADS + 1),
        };
        
        assert!(args.validate_and_normalize().is_err());
    }
    
    #[test]
    fn test_nonexistent_input_directory() {
        let temp_output = TempDir::new().unwrap();
        let nonexistent_input = PathBuf::from("/nonexistent/path");
        
        let args = CliArgs {
            input: nonexistent_input,
            output: temp_output.path().to_path_buf(),
            threads: Some(1),
        };
        
        assert!(args.validate_and_normalize().is_err());
    }
    
    #[test]
    fn test_valid_configuration() {
        let temp_input = TempDir::new().unwrap();
        let temp_output = TempDir::new().unwrap();
        
        let args = CliArgs {
            input: temp_input.path().to_path_buf(),
            output: temp_output.path().to_path_buf(),
            threads: Some(2),
        };
        
        let result = args.validate_and_normalize();
        assert!(result.is_ok());
        
        let validated = result.unwrap();
        assert_eq!(validated.threads, 2);
        assert!(validated.input.is_absolute());
        assert!(validated.output.is_absolute());
    }
}
