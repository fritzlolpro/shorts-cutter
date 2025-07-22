use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};
use crate::config::{FFMPEG_EXECUTABLE, FFMPEG_FILTER_COMPLEX, FFMPEG_TIMEOUT};
use crate::error::{FfmpegError, FfmpegResult};

/// Структура для представления FFmpeg команды
#[derive(Debug, Clone)]
pub struct FfmpegCommand {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub command_string: String,
}

impl FfmpegCommand {
    /// Создает новую FFmpeg команду для обработки видео в вертикальный shorts
    pub fn new(input_path: PathBuf, output_path: PathBuf) -> Self {
        let command_string = build_ffmpeg_command_string(&input_path, &output_path);
        
        Self {
            input_path,
            output_path,
            command_string,
        }
    }
    
    /// Возвращает аргументы для выполнения команды
    pub fn args(&self) -> Vec<String> {
        build_ffmpeg_args(&self.input_path, &self.output_path)
    }
    
    /// Возвращает строковое представление команды для логирования
    pub fn display_string(&self) -> &str {
        &self.command_string
    }
}

/// Проверяет доступность FFmpeg в системе
pub async fn check_ffmpeg_availability() -> FfmpegResult<String> {
    debug!("Checking FFmpeg availability...");
    
    let output = Command::new(FFMPEG_EXECUTABLE)
        .args(["-version"])
        .output()
        .await
        .map_err(|_| FfmpegError::CannotSpawnProcess)?;
    
    if !output.status.success() {
        return Err(FfmpegError::CannotSpawnProcess);
    }
    
    let version_info = String::from_utf8_lossy(&output.stdout);
    let version_line = version_info
        .lines()
        .next()
        .unwrap_or("Unknown version")
        .to_string();
    
    info!("FFmpeg found: {}", version_line);
    Ok(version_line)
}

/// Выполняет FFmpeg команду асинхронно
pub async fn execute_ffmpeg_command(cmd: FfmpegCommand) -> FfmpegResult<FfmpegExecutionResult> {
    let start_time = std::time::Instant::now();
    
    debug!("Starting FFmpeg execution for: {}", cmd.input_path.display());
    debug!("Command: {}", cmd.display_string());
    
    // Проверяем существование входного файла
    if !cmd.input_path.exists() {
        return Err(FfmpegError::invalid_format(cmd.input_path));
    }
    
    // Создаем директорию для выходного файла если нужно
    if let Some(parent) = cmd.output_path.parent() {
        if let Err(_) = tokio::fs::create_dir_all(parent).await {
            return Err(FfmpegError::CannotSpawnProcess);
        }
    }
    
    let args = cmd.args();
    
    // Запускаем FFmpeg с захватом stdout и stderr
    let child = Command::new(FFMPEG_EXECUTABLE)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| FfmpegError::CannotSpawnProcess)?;
    
    // Ждем завершения с таймаутом
    let execution_result = match timeout(FFMPEG_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let duration = start_time.elapsed();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if output.status.success() {
                info!("FFmpeg completed successfully for: {} ({})", 
                     cmd.input_path.file_name().unwrap_or_default().to_string_lossy(),
                     format_duration(duration));
                
                FfmpegExecutionResult {
                    success: true,
                    exit_code: output.status.code().unwrap_or(0),
                    stdout,
                    stderr,
                    duration,
                    command: cmd.command_string.clone(),
                }
            } else {
                let exit_code = output.status.code().unwrap_or(-1);
                warn!("FFmpeg failed for: {} (exit code: {})", 
                     cmd.input_path.file_name().unwrap_or_default().to_string_lossy(),
                     exit_code);
                
                return Err(FfmpegError::execution_failed(
                    exit_code,
                    stderr,
                    cmd.command_string,
                ));
            }
        }
        Ok(Err(e)) => {
            warn!("FFmpeg process error for: {} - {}", 
                 cmd.input_path.file_name().unwrap_or_default().to_string_lossy(),
                 e);
            return Err(FfmpegError::CannotSpawnProcess);
        }
        Err(_) => {
            warn!("FFmpeg timeout for: {}", 
                 cmd.input_path.file_name().unwrap_or_default().to_string_lossy());
            
            return Err(FfmpegError::timeout(FFMPEG_TIMEOUT.as_secs()));
        }
    };
    
    Ok(execution_result)
}

/// Результат выполнения FFmpeg команды
#[derive(Debug, Clone)]
pub struct FfmpegExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub command: String,
}

impl FfmpegExecutionResult {
    /// Возвращает краткое описание результата
    pub fn summary(&self) -> String {
        if self.success {
            format!("Success ({})", format_duration(self.duration))
        } else {
            format!("Failed (exit code: {})", self.exit_code)
        }
    }
    
    /// Возвращает подробную информацию об ошибке
    pub fn error_details(&self) -> Option<String> {
        if !self.success && !self.stderr.is_empty() {
            Some(extract_ffmpeg_error(&self.stderr))
        } else {
            None
        }
    }
}

/// Строит аргументы для FFmpeg команды
fn build_ffmpeg_args(input_path: &Path, output_path: &Path) -> Vec<String> {
    let input_str = input_path.to_string_lossy().to_string();
    let output_str = output_path.to_string_lossy().to_string();
    
    vec![
        "-i".to_string(),
        input_str.clone(),
        "-i".to_string(),
        input_str,
        "-filter_complex".to_string(),
        FFMPEG_FILTER_COMPLEX.to_string(),
        "-map".to_string(),
        "[out]".to_string(),
        "-map".to_string(),
        "0:a".to_string(),
        "-y".to_string(), // Перезаписывать выходные файлы без запроса
        output_str,
    ]
}

/// Строит строковое представление FFmpeg команды для логирования
fn build_ffmpeg_command_string(input_path: &Path, output_path: &Path) -> String {
    let args = build_ffmpeg_args(input_path, output_path);
    format!("{} {}", FFMPEG_EXECUTABLE, args.join(" "))
}

/// Извлекает полезную информацию об ошибке из stderr FFmpeg
fn extract_ffmpeg_error(stderr: &str) -> String {
    // Ищем последние строки с ошибками, игнорируя предупреждения
    let error_lines: Vec<&str> = stderr
        .lines()
        .rev()
        .take(10) // Берем последние 10 строк
        .filter(|line| {
            let line_lower = line.to_lowercase();
            line_lower.contains("error") || 
            line_lower.contains("failed") ||
            line_lower.contains("invalid") ||
            line_lower.contains("cannot") ||
            (line_lower.contains("no such file") && line_lower.contains("directory"))
        })
        .collect();
    
    if error_lines.is_empty() {
        // Если не нашли специфичные ошибки, возвращаем последние несколько строк
        stderr
            .lines()
            .rev()
            .take(3)
            .collect::<Vec<&str>>()
            .into_iter()
            .rev()
            .collect::<Vec<&str>>()
            .join(" | ")
    } else {
        error_lines
            .into_iter()
            .rev()
            .collect::<Vec<&str>>()
            .join(" | ")
    }
}

/// Форматирует Duration в человекочитаемый вид
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = duration.subsec_millis();
    
    if minutes > 0 {
        format!("{}m {}.{:03}s", minutes, seconds, millis)
    } else {
        format!("{}.{:03}s", seconds, millis)
    }
}

/// Валидирует входной файл перед обработкой
pub fn validate_input_file(path: &Path) -> FfmpegResult<()> {
    if !path.exists() {
        return Err(FfmpegError::invalid_format(path.to_path_buf()));
    }
    
    if !path.is_file() {
        return Err(FfmpegError::invalid_format(path.to_path_buf()));
    }
    
    // Проверяем расширение файла
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            let ext_lower = ext_str.to_lowercase();
            if !crate::config::DEFAULT_INPUT_EXTENSIONS.contains(&ext_lower.as_str()) {
                return Err(FfmpegError::invalid_format(path.to_path_buf()));
            }
        } else {
            return Err(FfmpegError::invalid_format(path.to_path_buf()));
        }
    } else {
        return Err(FfmpegError::invalid_format(path.to_path_buf()));
    }
    
    Ok(())
}

/// Оценивает примерный размер выходного файла на основе входного
pub async fn estimate_output_size(input_path: &Path) -> FfmpegResult<u64> {
    match tokio::fs::metadata(input_path).await {
        Ok(metadata) => {
            let input_size = metadata.len();
            // Примерная оценка: выходной файл будет примерно 80-120% от размера входного
            // из-за изменения разрешения и компрессии
            Ok((input_size as f64 * 1.0) as u64)
        }
        Err(_) => Err(FfmpegError::invalid_format(input_path.to_path_buf())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    
    #[test]
    fn test_build_ffmpeg_args() {
        let input = PathBuf::from("input.mp4");
        let output = PathBuf::from("output.mp4");
        
        let args = build_ffmpeg_args(&input, &output);
        
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"input.mp4".to_string()));
        assert!(args.contains(&"output.mp4".to_string()));
        assert!(args.contains(&"-filter_complex".to_string()));
        assert!(args.contains(&FFMPEG_FILTER_COMPLEX.to_string()));
    }
    
    #[test]
    fn test_ffmpeg_command_creation() {
        let input = PathBuf::from("test_input.mp4");
        let output = PathBuf::from("test_output.mp4");
        
        let cmd = FfmpegCommand::new(input.clone(), output.clone());
        
        assert_eq!(cmd.input_path, input);
        assert_eq!(cmd.output_path, output);
        assert!(cmd.command_string.contains("ffmpeg"));
        assert!(cmd.command_string.contains("test_input.mp4"));
        assert!(cmd.command_string.contains("test_output.mp4"));
    }
    
    #[test]
    fn test_extract_ffmpeg_error() {
        let stderr_with_error = "
Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'input.mp4':
  Metadata:
    creation_time   : 2023-01-01T00:00:00.000000Z
Error: No such file or directory
av_interleaved_write_frame(): Input/output error
        ";
        
        let error = extract_ffmpeg_error(stderr_with_error);
        assert!(error.contains("No such file"));
    }
    
    #[test]
    fn test_validate_input_file() {
        let temp_dir = TempDir::new().unwrap();
        
        // Создаем валидный файл
        let valid_file = temp_dir.path().join("test.mp4");
        File::create(&valid_file).unwrap();
        
        assert!(validate_input_file(&valid_file).is_ok());
        
        // Тестируем несуществующий файл
        let nonexistent = temp_dir.path().join("nonexistent.mp4");
        assert!(validate_input_file(&nonexistent).is_err());
        
        // Тестируем файл с неправильным расширением
        let wrong_ext = temp_dir.path().join("test.txt");
        File::create(&wrong_ext).unwrap();
        assert!(validate_input_file(&wrong_ext).is_err());
    }
    
    #[test]
    fn test_format_duration() {
        use std::time::Duration;
        
        assert_eq!(format_duration(Duration::from_millis(500)), "0.500s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5.000s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5.000s");
    }
}
