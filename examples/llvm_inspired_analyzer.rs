//! LLVM-inspired Static Analyzer для BSL (концепт)
//!
//! Демонстрирует, как можно использовать идеи LLVM для статического анализа 1С кода
//! без самого LLVM IR.

use std::collections::HashMap;

/// Analysis Pass trait (аналог LLVM Pass)
pub trait AnalysisPass {
    fn name(&self) -> &str;
    fn run(&self, code: &str) -> Vec<AnalysisError>;
}

/// Результат анализа
#[derive(Debug)]
pub struct AnalysisError {
    pub pass_name: String,
    pub line: usize,
    pub message: String,
}

/// Pass 1: Type Safety Check (как в твоём TypeResolver)
pub struct TypeSafetyPass;

impl AnalysisPass for TypeSafetyPass {
    fn name(&self) -> &str {
        "Type Safety Check"
    }

    fn run(&self, code: &str) -> Vec<AnalysisError> {
        let mut errors = vec![];

        // Пример: проверка несовместимых типов
        if code.contains("Число + Строка") {
            errors.push(AnalysisError {
                pass_name: self.name().to_string(),
                line: 1,
                message: "Нельзя складывать Число и Строку".to_string(),
            });
        }

        errors
    }
}

/// Pass 2: Null Safety Check
pub struct NullSafetyPass;

impl AnalysisPass for NullSafetyPass {
    fn name(&self) -> &str {
        "Null Safety Check"
    }

    fn run(&self, code: &str) -> Vec<AnalysisError> {
        let mut errors = vec![];

        // Пример: проверка использования Неопределено
        if code.contains("Неопределено.") {
            errors.push(AnalysisError {
                pass_name: self.name().to_string(),
                line: 1,
                message: "Вызов метода на Неопределено вызовет runtime ошибку".to_string(),
            });
        }

        errors
    }
}

/// Pass 3: Dead Code Detection
pub struct DeadCodePass;

impl AnalysisPass for DeadCodePass {
    fn name(&self) -> &str {
        "Dead Code Detection"
    }

    fn run(&self, code: &str) -> Vec<AnalysisError> {
        let mut errors = vec![];

        // Пример: код после Возврат
        let lines: Vec<&str> = code.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("Возврат")
                && i + 1 < lines.len()
                && !lines[i + 1].trim().starts_with("КонецФункции")
            {
                errors.push(AnalysisError {
                    pass_name: self.name().to_string(),
                    line: i + 2,
                    message: "Недостижимый код после Возврат".to_string(),
                });
            }
        }

        errors
    }
}

/// Pipeline анализа (аналог LLVM PassManager)
pub struct AnalysisPipeline {
    passes: Vec<Box<dyn AnalysisPass>>,
}

impl AnalysisPipeline {
    pub fn new() -> Self {
        Self {
            passes: vec![
                Box::new(TypeSafetyPass),
                Box::new(NullSafetyPass),
                Box::new(DeadCodePass),
            ],
        }
    }

    pub fn run(&self, code: &str) -> Vec<AnalysisError> {
        let mut all_errors = vec![];

        for pass in &self.passes {
            println!("Running pass: {}", pass.name());
            let errors = pass.run(code);
            all_errors.extend(errors);
        }

        all_errors
    }
}

impl Default for AnalysisPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Виртуальная компиляция (type inference без генерации кода)
pub struct VirtualCompiler {
    #[allow(dead_code)]
    type_env: HashMap<String, String>,
}

impl VirtualCompiler {
    pub fn new() -> Self {
        Self {
            type_env: HashMap::new(),
        }
    }

    /// Инферим тип переменной из использования
    pub fn infer_type(&mut self, _var: &str, usage: &str) -> String {
        // Простой пример type inference
        if usage.contains("+") || usage.contains("-") {
            "Число".to_string()
        } else if usage.contains("\"") {
            "Строка".to_string()
        } else if usage.contains(".Добавить(") {
            "Массив".to_string()
        } else {
            "Произвольный".to_string()
        }
    }

    /// Проверка совместимости типов
    pub fn check_compatibility(&self, expected: &str, actual: &str) -> Result<(), String> {
        if expected == actual || expected == "Произвольный" {
            Ok(())
        } else {
            Err(format!("Ожидался тип {}, получен {}", expected, actual))
        }
    }
}

impl Default for VirtualCompiler {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("=== LLVM-inspired Static Analyzer для BSL ===\n");

    // Пример 1: Analysis Pipeline
    let code = r#"
Функция ПримерФункции()
    Переменная = Неопределено;
    Переменная.Метод(); // Ошибка!

    Возврат Истина;
    Сообщить("Этот код недостижим"); // Dead code
КонецФункции
"#;

    let pipeline = AnalysisPipeline::new();
    let errors = pipeline.run(code);

    println!("Найдено ошибок: {}\n", errors.len());
    for error in errors {
        println!(
            "[{}] Line {}: {}",
            error.pass_name, error.line, error.message
        );
    }

    // Пример 2: Virtual Compiler (Type Inference)
    println!("\n=== Виртуальная компиляция (Type Inference) ===\n");

    let mut compiler = VirtualCompiler::new();

    let var_a_type = compiler.infer_type("А", "А + 5");
    println!("Инферим: А имеет тип {}", var_a_type);

    let var_b_type = compiler.infer_type("Б", "Б.Добавить(1)");
    println!("Инферим: Б имеет тип {}", var_b_type);

    // Проверка совместимости
    match compiler.check_compatibility("Число", "Строка") {
        Ok(_) => println!("Типы совместимы"),
        Err(e) => println!("Ошибка типов: {}", e),
    }
}
