//! Демонстрация возможностей BSL Gradual Type System

use std::fs;
use std::path::Path;
use bsl_gradual_types::parser::{BslParser, lexer::tokenize};
use bsl_gradual_types::core::type_checker::{TypeChecker, DiagnosticSeverity};
use clap::Parser as ClapParser;
use colored::*;

#[derive(ClapParser, Debug)]
#[command(name = "bsl-demo")]
#[command(about = "BSL Gradual Type System Demo", long_about = None)]
struct Args {
    /// BSL файл для анализа
    #[arg(short, long)]
    file: String,
    
    /// Показать токены
    #[arg(short = 't', long)]
    show_tokens: bool,
    
    /// Показать AST
    #[arg(short = 'a', long)]
    show_ast: bool,
    
    /// Показать выведенные типы
    #[arg(short = 'i', long)]
    show_types: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Читаем файл
    let content = fs::read_to_string(&args.file)?;
    let file_name = Path::new(&args.file)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    println!("{}", format!("═══ BSL Gradual Type System Demo ═══").bold().cyan());
    println!("{} {}\n", "Файл:".bold(), args.file);
    
    // Токенизация
    if args.show_tokens {
        println!("{}", "▶ ТОКЕНЫ:".bold().green());
        match tokenize(&content) {
            Ok((_, tokens)) => {
                for (i, token) in tokens.iter().enumerate() {
                    if i > 20 {
                        println!("... и ещё {} токенов", tokens.len() - 20);
                        break;
                    }
                    println!("  {:?}", token);
                }
            }
            Err(e) => {
                println!("{} {}", "Ошибка токенизации:".red(), e);
                return Ok(());
            }
        }
        println!();
    }
    
    // Парсинг
    println!("{}", "▶ ПАРСИНГ:".bold().green());
    let mut parser = match BslParser::new(&content) {
        Ok(p) => p,
        Err(e) => {
            println!("{} {}", "Ошибка создания парсера:".red(), e);
            return Ok(());
        }
    };
    
    let program = match parser.parse() {
        Ok(p) => {
            println!("  {} Программа успешно распарсена", "✓".green());
            println!("  Операторов: {}", p.statements.len());
            p
        }
        Err(e) => {
            println!("{} {}", "Ошибка парсинга:".red(), e);
            return Ok(());
        }
    };
    
    // Показываем AST
    if args.show_ast {
        println!("\n{}", "▶ ABSTRACT SYNTAX TREE:".bold().green());
        for (i, stmt) in program.statements.iter().enumerate() {
            if i > 10 {
                println!("  ... и ещё {} операторов", program.statements.len() - 10);
                break;
            }
            println!("  {:?}", stmt);
        }
    }
    
    // Type checking
    println!("\n{}", "▶ АНАЛИЗ ТИПОВ:".bold().green());
    let checker = TypeChecker::new(file_name);
    let (context, diagnostics) = checker.check(&program);
    
    // Статистика
    let errors = diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Error).count();
    let warnings = diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Warning).count();
    let infos = diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Info).count();
    
    println!("  Переменных найдено: {}", context.variables.len());
    println!("  Функций найдено: {}", context.functions.len());
    println!("  {} Ошибок: {}", if errors > 0 { "✗".red() } else { "✓".green() }, errors);
    println!("  {} Предупреждений: {}", if warnings > 0 { "⚠".yellow() } else { "✓".green() }, warnings);
    println!("  ℹ Информационных: {}", infos);
    
    // Выводим диагностику
    if !diagnostics.is_empty() {
        println!("\n{}", "▶ ДИАГНОСТИКА:".bold().yellow());
        for diag in &diagnostics {
            let icon = match diag.severity {
                DiagnosticSeverity::Error => "✗".red(),
                DiagnosticSeverity::Warning => "⚠".yellow(),
                DiagnosticSeverity::Info => "ℹ".blue(),
                DiagnosticSeverity::Hint => "💡".cyan(),
            };
            
            let severity = match diag.severity {
                DiagnosticSeverity::Error => "ERROR".red().bold(),
                DiagnosticSeverity::Warning => "WARN".yellow().bold(),
                DiagnosticSeverity::Info => "INFO".blue(),
                DiagnosticSeverity::Hint => "HINT".cyan(),
            };
            
            println!("  {} [{}] Строка {}: {}", 
                icon, 
                severity, 
                diag.line,
                diag.message
            );
        }
    }
    
    // Показываем выведенные типы
    if args.show_types && !context.variables.is_empty() {
        println!("\n{}", "▶ ВЫВЕДЕННЫЕ ТИПЫ ПЕРЕМЕННЫХ:".bold().green());
        for (name, type_res) in context.variables.iter().take(15) {
            let certainty = match &type_res.certainty {
                bsl_gradual_types::core::types::Certainty::Known => "известен".green(),
                bsl_gradual_types::core::types::Certainty::Inferred(conf) => 
                    format!("выведен ({:.0}%)", conf * 100.0).yellow(),
                bsl_gradual_types::core::types::Certainty::Unknown => "неизвестен".red(),
            };
            
            println!("  {} {}: тип {}", 
                "•".cyan(),
                name.bold(), 
                certainty
            );
        }
        
        if context.variables.len() > 15 {
            println!("  ... и ещё {} переменных", context.variables.len() - 15);
        }
    }
    
    // Показываем функции
    if !context.functions.is_empty() {
        println!("\n{}", "▶ НАЙДЕННЫЕ ФУНКЦИИ:".bold().green());
        for (name, sig) in context.functions.iter().take(10) {
            let params = sig.params.iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            
            let export = if sig.exported { " [Экспорт]".cyan() } else { "".normal() };
            
            println!("  {} {}({}){}", 
                "•".cyan(),
                name.bold(),
                params,
                export
            );
        }
        
        if context.functions.len() > 10 {
            println!("  ... и ещё {} функций", context.functions.len() - 10);
        }
    }
    
    println!("\n{}", "═══════════════════════════════════".cyan());
    
    Ok(())
}