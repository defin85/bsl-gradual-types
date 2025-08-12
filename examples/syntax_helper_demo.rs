//! Демонстрация парсинга файлов синтакс-помощника 1С

use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

fn main() -> anyhow::Result<()> {
    println!("=== Демонстрация парсинга синтакс-помощника 1С ===\n");
    
    // Пути к файлам
    let context_file = "examples/syntax_helper/rebuilt.shcntx_ru.zip";
    let lang_file = "examples/syntax_helper/rebuilt.shlang_ru.zip";
    
    // Анализируем файл контекстной справки
    println!("1. Анализ контекстной справки (shcntx_ru):");
    analyze_archive(context_file)?;
    
    // Анализируем файл справки по языку
    println!("\n2. Анализ справки по языку (shlang_ru):");
    analyze_archive(lang_file)?;
    
    // Пример извлечения глобального контекста
    println!("\n3. Извлечение глобального контекста:");
    extract_global_context(context_file)?;
    
    Ok(())
}

fn analyze_archive(path: &str) -> anyhow::Result<()> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    
    println!("   Файлов в архиве: {}", archive.len());
    
    let mut html_count = 0;
    let mut st_count = 0;
    let mut total_size = 0u64;
    
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name();
        total_size += file.size();
        
        if name.ends_with(".html") {
            html_count += 1;
        } else if name.ends_with(".st") {
            st_count += 1;
        }
    }
    
    println!("   HTML файлов: {}", html_count);
    println!("   ST файлов: {}", st_count);
    println!("   Общий размер (распакованный): {} МБ", total_size / 1_048_576);
    
    // Показываем примеры файлов
    println!("   Примеры файлов:");
    for i in 0..5.min(archive.len()) {
        let file = archive.by_index(i)?;
        println!("     - {}", file.name());
    }
    
    Ok(())
}

fn extract_global_context(path: &str) -> anyhow::Result<()> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    
    // Ищем файл Global context.html
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name() == "objects/Global context.html" {
            println!("   Найден файл глобального контекста!");
            println!("   Размер: {} КБ", file.size() / 1024);
            
            // Читаем первые 500 байт для примера
            let mut buffer = vec![0; 500.min(file.size() as usize)];
            file.read_exact(&mut buffer)?;
            
            let preview = String::from_utf8_lossy(&buffer);
            println!("   Превью содержимого:");
            println!("   {}", &preview[..200.min(preview.len())]);
            println!("   ...");
            
            break;
        }
    }
    
    Ok(())
}