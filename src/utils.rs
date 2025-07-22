use std::path::{Path, PathBuf};
use crate::error::{FileSystemError, FileSystemResult};
use crate::config::DEFAULT_INPUT_EXTENSIONS;
use tracing::debug;

/// Рекурсивно ищет все файлы с поддерживаемыми расширениями в директории
pub fn find_video_files(input_dir: &Path) -> FileSystemResult<Vec<PathBuf>> {
    let mut video_files = Vec::new();
    
    debug!("Searching for video files in: {}", input_dir.display());
    
    find_files_recursive(input_dir, &mut video_files)?;
    
    // Сортируем файлы для предсказуемого порядка обработки
    video_files.sort();
    
    debug!("Found {} video files", video_files.len());
    Ok(video_files)
}

/// Рекурсивная функция поиска файлов
fn find_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> FileSystemResult<()> {
    let entries = std::fs::read_dir(dir)
        .map_err(|_| FileSystemError::cannot_read_dir(dir.to_path_buf()))?;
    
    for entry in entries {
        let entry = entry
            .map_err(|_| FileSystemError::cannot_read_dir(dir.to_path_buf()))?;
        
        let path = entry.path();
        
        if path.is_dir() {
            // Рекурсивно обходим подпапки
            find_files_recursive(&path, files)?;
        } else if path.is_file() && is_supported_video_file(&path) {
            files.push(path);
        }
    }
    
    Ok(())
}

/// Проверяет, является ли файл поддерживаемым видеофайлом
pub fn is_supported_video_file(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            let ext_lower = ext_str.to_lowercase();
            return DEFAULT_INPUT_EXTENSIONS.contains(&ext_lower.as_str());
        }
    }
    false
}

/// Генерирует путь к выходному файлу на основе входного файла
pub fn generate_output_path(input_path: &Path, output_dir: &Path) -> PathBuf {
    let input_filename = input_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    
    let input_extension = input_path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");
    
    let output_filename = format!("{}{}.{}", 
                                 input_filename, 
                                 crate::config::OUTPUT_SUFFIX, 
                                 input_extension);
    
    output_dir.join(output_filename)
}

/// Проверяет, существует ли файл и доступен ли он для чтения
pub fn validate_input_file(path: &Path) -> FileSystemResult<()> {
    if !path.exists() {
        return Err(FileSystemError::not_found(path.to_path_buf()));
    }
    
    if !path.is_file() {
        return Err(FileSystemError::cannot_access(path.to_path_buf()));
    }
    
    // Проверяем права на чтение
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if metadata.permissions().readonly() {
                // На Windows readonly не означает отсутствие прав на чтение
                #[cfg(windows)]
                return Ok(());
                
                #[cfg(not(windows))]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o444 == 0 {
                        return Err(FileSystemError::permission_denied(path.to_path_buf()));
                    }
                }
            }
        }
        Err(_) => {
            return Err(FileSystemError::cannot_access(path.to_path_buf()));
        }
    }
    
    Ok(())
}

/// Проверяет права на запись в директорию
pub fn validate_output_directory(path: &Path) -> FileSystemResult<()> {
    if !path.exists() {
        return Err(FileSystemError::not_found(path.to_path_buf()));
    }
    
    if !path.is_dir() {
        return Err(FileSystemError::cannot_access(path.to_path_buf()));
    }
    
    // Пытаемся создать временный файл для проверки прав на запись
    let temp_file_path = path.join(".shorts_cutter_write_test");
    match std::fs::write(&temp_file_path, b"test") {
        Ok(_) => {
            // Удаляем временный файл
            let _ = std::fs::remove_file(&temp_file_path);
            Ok(())
        }
        Err(_) => Err(FileSystemError::permission_denied(path.to_path_buf())),
    }
}

/// Проверяет, достаточно ли места на диске для выходного файла
pub fn check_disk_space(_output_path: &Path, estimated_size: Option<u64>) -> FileSystemResult<()> {
    // Пытаемся получить информацию о свободном месте
    // Это упрощенная проверка - более точную реализацию можно добавить позже
    if let Some(size) = estimated_size {
        if size > 1024 * 1024 * 1024 * 10 { // 10GB лимит как пример
            return Err(FileSystemError::InsufficientSpace);
        }
    }
    
    Ok(())
}

/// Создает безопасное имя файла, удаляя недопустимые символы
pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Получает размер файла в байтах
pub fn get_file_size(path: &Path) -> FileSystemResult<u64> {
    let metadata = std::fs::metadata(path)
        .map_err(|_| FileSystemError::cannot_access(path.to_path_buf()))?;
    
    Ok(metadata.len())
}

/// Форматирует размер файла в человекочитаемый вид
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size_f = size as f64;
    let mut unit_index = 0;
    
    while size_f >= 1024.0 && unit_index < UNITS.len() - 1 {
        size_f /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size_f, UNITS[unit_index])
    }
}

/// Структура для представления задачи обработки файла
#[derive(Debug, Clone)]
pub struct FileTask {
    pub input: PathBuf,
    pub output: PathBuf,
}

impl FileTask {
    /// Создает новую задачу обработки файла
    pub fn new(input: PathBuf, output: PathBuf) -> Self {
        Self { input, output }
    }
    
    /// Валидирует задачу перед обработкой
    pub fn validate(&self) -> FileSystemResult<()> {
        validate_input_file(&self.input)?;
        
        // Проверяем директорию назначения
        if let Some(parent) = self.output.parent() {
            validate_output_directory(parent)?;
        }
        
        // Проверяем, что выходной файл не совпадает с входным
        if self.input == self.output {
            return Err(FileSystemError::cannot_access(self.output.clone()));
        }
        
        Ok(())
    }
    
    /// Возвращает имя входного файла для отображения
    pub fn input_filename(&self) -> String {
        self.input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
    
    /// Возвращает имя выходного файла для отображения
    pub fn output_filename(&self) -> String {
        self.output
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

/// Создает список задач обработки на основе найденных файлов
pub fn create_file_tasks(input_files: Vec<PathBuf>, output_dir: &Path) -> Vec<FileTask> {
    input_files
        .into_iter()
        .map(|input_path| {
            let output_path = generate_output_path(&input_path, output_dir);
            FileTask::new(input_path, output_path)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[test]
    fn test_is_supported_video_file() {
        assert!(is_supported_video_file(&PathBuf::from("test.mp4")));
        assert!(is_supported_video_file(&PathBuf::from("test.MP4")));
        assert!(!is_supported_video_file(&PathBuf::from("test.avi")));
        assert!(!is_supported_video_file(&PathBuf::from("test.txt")));
        assert!(!is_supported_video_file(&PathBuf::from("test")));
    }
    
    #[test]
    fn test_generate_output_path() {
        let input = PathBuf::from("/input/video.mp4");
        let output_dir = PathBuf::from("/output");
        
        let result = generate_output_path(&input, &output_dir);
        assert_eq!(result, PathBuf::from("/output/video-short.mp4"));
    }
    
    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal_file.mp4"), "normal_file.mp4");
        assert_eq!(sanitize_filename("file<with>bad:chars.mp4"), "file_with_bad_chars.mp4");
        assert_eq!(sanitize_filename("file|with\"quotes.mp4"), "file_with_quotes.mp4");
    }
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024 * 2), "2.0 MB");
        assert_eq!(format_file_size(1024_u64.pow(3) * 5), "5.0 GB");
    }
    
    #[test]
    fn test_find_video_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Создаем тестовые файлы
        File::create(temp_dir.path().join("video1.mp4")).unwrap();
        File::create(temp_dir.path().join("video2.MP4")).unwrap();
        File::create(temp_dir.path().join("document.txt")).unwrap();
        File::create(temp_dir.path().join("video3.avi")).unwrap();
        
        // Создаем подпапку с видео
        let sub_dir = temp_dir.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();
        File::create(sub_dir.join("video4.mp4")).unwrap();
        
        let result = find_video_files(temp_dir.path()).unwrap();
        
        // Должны найти 3 .mp4 файла (video1, video2, video4)
        assert_eq!(result.len(), 3);
        
        // Проверяем, что все найденные файлы имеют правильное расширение
        for path in &result {
            assert!(is_supported_video_file(path));
        }
    }
}
