use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinHandle;
use tracing::{info, error, debug};

use crate::utils::FileTask;
use crate::ffmpeg::{FfmpegCommand, execute_ffmpeg_command, FfmpegExecutionResult};
use crate::logger::{FileProcessingLogger, ProcessingSummary};
use crate::error::Result;

/// Результат обработки одного файла
#[derive(Debug, Clone)]
pub enum TaskResult {
    Success {
        input: std::path::PathBuf,
        output: std::path::PathBuf,
        duration: Duration,
        ffmpeg_result: FfmpegExecutionResult,
    },
    Failure {
        input: std::path::PathBuf,
        error: String,
        duration: Duration,
    },
}

impl TaskResult {
    /// Возвращает путь к входному файлу
    pub fn input_path(&self) -> &std::path::PathBuf {
        match self {
            TaskResult::Success { input, .. } => input,
            TaskResult::Failure { input, .. } => input,
        }
    }
    
    /// Возвращает длительность обработки
    pub fn duration(&self) -> Duration {
        match self {
            TaskResult::Success { duration, .. } => *duration,
            TaskResult::Failure { duration, .. } => *duration,
        }
    }
    
    /// Проверяет, была ли обработка успешной
    pub fn is_success(&self) -> bool {
        matches!(self, TaskResult::Success { .. })
    }
}

/// Worker pool для параллельной обработки видеофайлов
pub struct WorkerPool {
    semaphore: Arc<Semaphore>,
    max_workers: usize,
}

impl WorkerPool {
    /// Создает новый worker pool с указанным количеством воркеров
    pub fn new(max_workers: usize) -> Self {
        info!("Creating worker pool with {} workers", max_workers);
        
        Self {
            semaphore: Arc::new(Semaphore::new(max_workers)),
            max_workers,
        }
    }
    
    /// Выполняет список задач параллельно и возвращает результаты
    pub async fn execute_tasks(&self, tasks: Vec<FileTask>) -> Result<ProcessingResults> {
        let total_tasks = tasks.len();
        let start_time = Instant::now();
        
        info!("Starting parallel processing of {} tasks", total_tasks);
        
        if tasks.is_empty() {
            return Ok(ProcessingResults::empty());
        }
        
        // Создаем канал для сбора результатов
        let (tx, mut rx) = mpsc::unbounded_channel::<TaskResult>();
        
        // Запускаем все задачи
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        
        for (index, task) in tasks.into_iter().enumerate() {
            let semaphore = Arc::clone(&self.semaphore);
            let tx = tx.clone();
            
            let handle = tokio::spawn(async move {
                // Получаем разрешение от семафора
                let _permit = semaphore.acquire().await.unwrap();
                
                debug!("Starting task {}/{} for: {}", 
                       index + 1, total_tasks, task.input_filename());
                
                let result = process_single_file(task).await;
                
                if let Err(e) = tx.send(result) {
                    error!("Failed to send task result: {}", e);
                }
                
                // Разрешение автоматически освобождается при выходе из области видимости
            });
            
            handles.push(handle);
        }
        
        // Закрываем отправитель, чтобы получатель знал, когда все задачи отправлены
        drop(tx);
        
        // Собираем все результаты
        let mut results = Vec::new();
        while let Some(result) = rx.recv().await {
            results.push(result);
        }
        
        // Ждем завершения всех задач
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task handle join error: {}", e);
            }
        }
        
        let total_duration = start_time.elapsed();
        
        info!("Completed processing {} tasks in {}", 
              total_tasks, format_duration(total_duration));
        
        Ok(ProcessingResults::from_task_results(results, total_duration))
    }
    
    /// Возвращает количество активных воркеров
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
    
    /// Возвращает максимальное количество воркеров
    pub fn max_workers(&self) -> usize {
        self.max_workers
    }
}

/// Результаты обработки всех задач
#[derive(Debug)]
pub struct ProcessingResults {
    pub successful: Vec<TaskResult>,
    pub failed: Vec<TaskResult>,
    pub total_duration: Duration,
}

impl ProcessingResults {
    /// Создает пустой результат
    pub fn empty() -> Self {
        Self {
            successful: Vec::new(),
            failed: Vec::new(),
            total_duration: Duration::ZERO,
        }
    }
    
    /// Создает результаты из списка задач
    pub fn from_task_results(results: Vec<TaskResult>, total_duration: Duration) -> Self {
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        
        for result in results {
            match result {
                TaskResult::Success { .. } => successful.push(result),
                TaskResult::Failure { .. } => failed.push(result),
            }
        }
        
        Self {
            successful,
            failed,
            total_duration,
        }
    }
    
    /// Возвращает общее количество задач
    pub fn total_count(&self) -> usize {
        self.successful.len() + self.failed.len()
    }
    
    /// Возвращает количество успешных задач
    pub fn success_count(&self) -> usize {
        self.successful.len()
    }
    
    /// Возвращает количество неудачных задач
    pub fn failure_count(&self) -> usize {
        self.failed.len()
    }
    
    /// Конвертирует в ProcessingSummary для логирования
    pub fn to_processing_summary(&self) -> ProcessingSummary {
        let mut summary = ProcessingSummary::new();
        
        for result in &self.successful {
            if let TaskResult::Success { input, output, duration, .. } = result {
                summary.add_success(input.clone(), output.clone(), *duration);
            }
        }
        
        for result in &self.failed {
            if let TaskResult::Failure { input, error, .. } = result {
                summary.add_failure(input.clone(), error.clone());
            }
        }
        
        summary.set_total_duration(self.total_duration);
        summary
    }
}

/// Обрабатывает один файл
async fn process_single_file(task: FileTask) -> TaskResult {
    let start_time = Instant::now();
    let filename = task.input_filename();
    
    // Создаем логгер для этого файла
    let logger = FileProcessingLogger::start_processing(&filename);
    
    // Валидируем задачу
    if let Err(e) = task.validate() {
        let duration = start_time.elapsed();
        let error_msg = format!("Task validation failed: {}", e);
        
        logger.log_error(&task.input, &task.output, &error_msg);
        
        return TaskResult::Failure {
            input: task.input,
            error: error_msg,
            duration,
        };
    }
    
    // Валидируем входной файл для FFmpeg
    if let Err(e) = crate::ffmpeg::validate_input_file(&task.input) {
        let duration = start_time.elapsed();
        let error_msg = format!("Input file validation failed: {}", e);
        
        logger.log_error(&task.input, &task.output, &error_msg);
        
        return TaskResult::Failure {
            input: task.input,
            error: error_msg,
            duration,
        };
    }
    
    // Создаем FFmpeg команду
    let ffmpeg_cmd = FfmpegCommand::new(task.input.clone(), task.output.clone());
    logger.log_ffmpeg_command(ffmpeg_cmd.display_string());
    
    // Выполняем FFmpeg команду
    match execute_ffmpeg_command(ffmpeg_cmd).await {
        Ok(ffmpeg_result) => {
            let duration = start_time.elapsed();
            
            if ffmpeg_result.success {
                logger.log_success(&task.input, &task.output);
                
                TaskResult::Success {
                    input: task.input,
                    output: task.output,
                    duration,
                    ffmpeg_result,
                }
            } else {
                let error_msg = format!("FFmpeg execution failed: {}", 
                                       ffmpeg_result.error_details().unwrap_or_else(|| "Unknown error".to_string()));
                logger.log_error(&task.input, &task.output, &error_msg);
                
                TaskResult::Failure {
                    input: task.input,
                    error: error_msg,
                    duration,
                }
            }
        }
        Err(e) => {
            let duration = start_time.elapsed();
            let error_msg = format!("FFmpeg error: {}", e);
            
            logger.log_error(&task.input, &task.output, &error_msg);
            
            TaskResult::Failure {
                input: task.input,
                error: error_msg,
                duration,
            }
        }
    }
}

/// Форматирует Duration в человекочитаемый вид
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Структура для мониторинга прогресса обработки
pub struct ProgressMonitor {
    total_tasks: usize,
    completed_tasks: std::sync::atomic::AtomicUsize,
    start_time: Instant,
}

impl ProgressMonitor {
    /// Создает новый монитор прогресса
    pub fn new(total_tasks: usize) -> Self {
        Self {
            total_tasks,
            completed_tasks: std::sync::atomic::AtomicUsize::new(0),
            start_time: Instant::now(),
        }
    }
    
    /// Увеличивает счетчик завершенных задач
    pub fn increment_completed(&self) -> usize {
        self.completed_tasks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }
    
    /// Возвращает текущий прогресс в процентах
    pub fn progress_percentage(&self) -> f64 {
        let completed = self.completed_tasks.load(std::sync::atomic::Ordering::Relaxed);
        if self.total_tasks == 0 {
            100.0
        } else {
            (completed as f64 / self.total_tasks as f64) * 100.0
        }
    }
    
    /// Возвращает ETA (оценочное время до завершения)
    pub fn estimated_time_remaining(&self) -> Option<Duration> {
        let completed = self.completed_tasks.load(std::sync::atomic::Ordering::Relaxed);
        if completed == 0 {
            return None;
        }
        
        let elapsed = self.start_time.elapsed();
        let avg_time_per_task = elapsed / completed as u32;
        let remaining_tasks = self.total_tasks.saturating_sub(completed);
        
        if remaining_tasks == 0 {
            Some(Duration::ZERO)
        } else {
            Some(avg_time_per_task * remaining_tasks as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    
    #[test]
    fn test_worker_pool_creation() {
        let pool = WorkerPool::new(4);
        assert_eq!(pool.max_workers(), 4);
        assert_eq!(pool.available_permits(), 4);
    }
    
    #[test]
    fn test_processing_results() {
        let successful_result = TaskResult::Success {
            input: PathBuf::from("input.mp4"),
            output: PathBuf::from("output.mp4"),
            duration: Duration::from_secs(10),
            ffmpeg_result: crate::ffmpeg::FfmpegExecutionResult {
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                duration: Duration::from_secs(10),
                command: "ffmpeg...".to_string(),
            },
        };
        
        let failed_result = TaskResult::Failure {
            input: PathBuf::from("input2.mp4"),
            error: "Test error".to_string(),
            duration: Duration::from_secs(5),
        };
        
        let results = ProcessingResults::from_task_results(
            vec![successful_result, failed_result],
            Duration::from_secs(20),
        );
        
        assert_eq!(results.total_count(), 2);
        assert_eq!(results.success_count(), 1);
        assert_eq!(results.failure_count(), 1);
        assert_eq!(results.total_duration, Duration::from_secs(20));
    }
    
    #[test]
    fn test_progress_monitor() {
        let monitor = ProgressMonitor::new(10);
        
        assert_eq!(monitor.progress_percentage(), 0.0);
        
        monitor.increment_completed();
        assert_eq!(monitor.progress_percentage(), 10.0);
        
        for _ in 0..9 {
            monitor.increment_completed();
        }
        assert_eq!(monitor.progress_percentage(), 100.0);
    }
    
    #[test]
    fn test_task_result() {
        let result = TaskResult::Success {
            input: PathBuf::from("test.mp4"),
            output: PathBuf::from("test-short.mp4"),
            duration: Duration::from_secs(5),
            ffmpeg_result: crate::ffmpeg::FfmpegExecutionResult {
                success: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                duration: Duration::from_secs(5),
                command: "ffmpeg...".to_string(),
            },
        };
        
        assert!(result.is_success());
        assert_eq!(result.duration(), Duration::from_secs(5));
        assert_eq!(result.input_path(), &PathBuf::from("test.mp4"));
    }
}
