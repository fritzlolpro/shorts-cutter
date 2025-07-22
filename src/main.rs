mod cli;
mod config;
mod error;
mod ffmpeg;
mod logger;
mod utils;
mod worker;

use cli::CliArgs;
use error::Result;
use worker::WorkerPool;
use std::process;
use tracing::{info, error as log_error};

#[tokio::main]
async fn main() {
    let exit_code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            
            // Логируем ошибку если логирование уже инициализировано
            log_error!("Critical error: {}", e);
            
            config::exit_codes::CRITICAL_ERROR
        }
    };
    
    process::exit(exit_code);
}

async fn run() -> Result<i32> {
    // Парсим аргументы командной строки
    let args = CliArgs::parse_args();
    
    // Валидируем и нормализуем аргументы
    let validated_args = args.validate_and_normalize()?;
    
    // Проверяем доступность FFmpeg
    validated_args.check_ffmpeg_availability().await?;
    
    // Инициализируем логирование
    let log_file_path = validated_args.log_file_path();
    logger::initialize_logging(
        log_file_path,
        &config::AppConfig::default().console_log_level,
        &config::AppConfig::default().file_log_level,
    )?;
    
    // Выводим информацию о конфигурации
    validated_args.print_config_info();
    
    // Проверяем версию FFmpeg и логируем
    match ffmpeg::check_ffmpeg_availability().await {
        Ok(version) => info!("Using {}", version),
        Err(e) => {
            eprintln!("FFmpeg check failed: {}", e);
            return Ok(config::exit_codes::CRITICAL_ERROR);
        }
    }
    
    // Логируем информацию о запуске
    logger::log_startup_info(&validated_args.input, &validated_args.output, validated_args.threads);
    
    // Ищем видеофайлы для обработки
    let video_files = utils::find_video_files(&validated_args.input)?;
    logger::log_files_found(video_files.len());
    
    if video_files.is_empty() {
        println!("{}", config::messages::NO_FILES_FOUND);
        return Ok(config::exit_codes::CRITICAL_ERROR);
    }
    
    // Создаем задачи обработки
    let tasks = utils::create_file_tasks(video_files, &validated_args.output);
    
    println!("{}", config::messages::PROCESSING_STARTED);
    println!("Found {} files to process", tasks.len());
    println!("Using {} parallel threads", validated_args.threads);
    println!();
    
    // Создаем worker pool и запускаем обработку
    let worker_pool = WorkerPool::new(validated_args.threads);
    
    info!("Starting parallel processing with {} workers", validated_args.threads);
    
    let processing_results = worker_pool.execute_tasks(tasks).await?;
    
    // Генерируем финальный отчет
    let summary = processing_results.to_processing_summary();
    summary.print_final_report();
    
    println!("
{}", config::messages::PROCESSING_COMPLETED);
    
    Ok(summary.exit_code())
}
