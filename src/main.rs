mod cli;
mod config;
mod error;
mod logger;
mod utils;

use cli::CliArgs;
use error::Result;
use std::process;

#[tokio::main]
async fn main() {
    let exit_code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
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
    
    // TODO: Здесь будет создание worker pool и запуск обработки
    // Пока просто имитируем успешную обработку
    let mut summary = logger::ProcessingSummary::new();
    
    for task in tasks {
        println!("Would process: {} -> {}", task.input_filename(), task.output_filename());
        // Имитируем успешную обработку
        summary.add_success(task.input, task.output, std::time::Duration::from_secs(5));
    }
    
    summary.set_total_duration(std::time::Duration::from_secs(30));
    summary.print_final_report();
    
    Ok(summary.exit_code())
}
