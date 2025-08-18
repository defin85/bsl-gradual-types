//! Оптимизация использования памяти для больших кодовых баз
//!
//! Этот модуль предоставляет инструменты для мониторинга и оптимизации
//! использования памяти при анализе больших проектов 1С.

use std::collections::HashMap;
use std::sync::Arc;
use std::mem;
use serde::{Serialize, Deserialize};

use crate::core::types::TypeResolution;
use crate::core::type_checker::TypeContext;

/// Профиль использования памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProfile {
    /// Общее использование памяти в байтах
    pub total_memory_bytes: usize,
    /// Память для типов
    pub types_memory_bytes: usize,
    /// Память для контекста
    pub context_memory_bytes: usize,
    /// Количество объектов в памяти
    pub object_counts: ObjectCounts,
    /// Время профилирования
    pub timestamp: std::time::SystemTime,
}

/// Счетчики объектов в памяти
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectCounts {
    /// Количество TypeResolution объектов
    pub type_resolutions: usize,
    /// Количество TypeContext объектов
    pub type_contexts: usize,
    /// Количество кешированных AST
    pub cached_asts: usize,
    /// Количество строк в интернированном пуле
    pub interned_strings: usize,
}

/// Статистика памяти
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub current_memory_bytes: usize,
    pub peak_memory_bytes: usize,
    pub average_memory_bytes: usize,
    pub total_objects: usize,
    pub memory_efficiency: f64,
}

impl MemoryStats {
    /// Форматировать статистику
    pub fn format(&self) -> String {
        format!(
            "🧠 Статистика памяти:\n\
             📊 Текущее использование: {:.2} MB\n\
             📈 Пиковое использование: {:.2} MB\n\
             📉 Среднее использование: {:.2} MB\n\
             🔢 Объектов в памяти: {}\n\
             ⚡ Эффективность: {:.1}%",
            self.current_memory_bytes as f64 / (1024.0 * 1024.0),
            self.peak_memory_bytes as f64 / (1024.0 * 1024.0),
            self.average_memory_bytes as f64 / (1024.0 * 1024.0),
            self.total_objects,
            self.memory_efficiency * 100.0
        )
    }
}

/// Монитор памяти
pub struct MemoryMonitor {
    /// Профили памяти по времени
    profiles: Vec<MemoryProfile>,
    /// Максимальное количество сохраняемых профилей
    max_profiles: usize,
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMonitor {
    /// Создать новый монитор памяти
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
            max_profiles: 100,
        }
    }
    
    /// Создать снимок текущего использования памяти
    pub fn take_snapshot(&mut self, contexts: &[&TypeContext]) -> MemoryProfile {
        let mut total_memory = 0;
        let mut types_memory = 0;
        let mut context_memory = 0;
        let mut object_counts = ObjectCounts::default();
        
        for context in contexts {
            // Приблизительно вычисляем размер TypeContext
            let ctx_size = mem::size_of::<TypeContext>() +
                context.variables.len() * (mem::size_of::<String>() + mem::size_of::<TypeResolution>()) +
                context.functions.len() * mem::size_of::<(String, crate::core::type_checker::FunctionSignature)>();
            
            context_memory += ctx_size;
            total_memory += ctx_size;
            
            // Подсчитываем объекты
            object_counts.type_contexts += 1;
            object_counts.type_resolutions += context.variables.len() + context.functions.len();
            
            // Вычисляем размер типов
            for type_res in context.variables.values() {
                types_memory += Self::estimate_type_resolution_size(type_res);
            }
        }
        
        let profile = MemoryProfile {
            total_memory_bytes: total_memory,
            types_memory_bytes: types_memory,
            context_memory_bytes: context_memory,
            object_counts,
            timestamp: std::time::SystemTime::now(),
        };
        
        self.profiles.push(profile.clone());
        
        // Ограничиваем количество профилей
        if self.profiles.len() > self.max_profiles {
            self.profiles.remove(0);
        }
        
        profile
    }
    
    /// Приблизительно вычислить размер TypeResolution
    fn estimate_type_resolution_size(type_res: &TypeResolution) -> usize {
        let mut size = mem::size_of::<TypeResolution>();
        
        // Добавляем размер метаданных
        size += type_res.metadata.notes.iter()
            .map(|note| note.len())
            .sum::<usize>();
        
        size
    }
    
    /// Получить статистику использования памяти
    pub fn get_memory_stats(&self) -> Option<MemoryStats> {
        if self.profiles.is_empty() {
            return None;
        }
        
        let latest = self.profiles.last().unwrap();
        let peak_memory = self.profiles.iter()
            .map(|p| p.total_memory_bytes)
            .max()
            .unwrap_or(0);
        
        let avg_memory = if !self.profiles.is_empty() {
            self.profiles.iter()
                .map(|p| p.total_memory_bytes)
                .sum::<usize>() / self.profiles.len()
        } else {
            0
        };
        
        Some(MemoryStats {
            current_memory_bytes: latest.total_memory_bytes,
            peak_memory_bytes: peak_memory,
            average_memory_bytes: avg_memory,
            total_objects: latest.object_counts.type_resolutions + 
                          latest.object_counts.type_contexts,
            memory_efficiency: Self::calculate_efficiency(latest),
        })
    }
    
    /// Вычислить эффективность использования памяти
    fn calculate_efficiency(profile: &MemoryProfile) -> f64 {
        if profile.total_memory_bytes == 0 {
            return 1.0;
        }
        
        // Эффективность = полезная память / общая память
        let useful_memory = profile.types_memory_bytes + profile.context_memory_bytes;
        useful_memory as f64 / profile.total_memory_bytes as f64
    }
}

/// String interning pool для экономии памяти на дублирующихся строках
pub struct StringInterner {
    /// Пул интернированных строк
    strings: HashMap<String, Arc<str>>,
    /// Статистика
    total_strings: usize,
    saved_bytes: usize,
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

impl StringInterner {
    /// Создать новый string interner
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            total_strings: 0,
            saved_bytes: 0,
        }
    }
    
    /// Интернировать строку
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        self.total_strings += 1;
        
        if let Some(interned) = self.strings.get(s) {
            self.saved_bytes += s.len();
            interned.clone()
        } else {
            let arc_str: Arc<str> = Arc::from(s);
            self.strings.insert(s.to_string(), arc_str.clone());
            arc_str
        }
    }
    
    /// Получить статистику интернирования
    pub fn get_stats(&self) -> StringInterningStats {
        StringInterningStats {
            unique_strings: self.strings.len(),
            total_requests: self.total_strings,
            saved_bytes: self.saved_bytes,
            hit_rate: if self.total_strings > 0 {
                (self.total_strings - self.strings.len()) as f64 / self.total_strings as f64
            } else {
                0.0
            },
        }
    }
}

/// Статистика string interning
#[derive(Debug, Clone)]
pub struct StringInterningStats {
    pub unique_strings: usize,
    pub total_requests: usize,
    pub saved_bytes: usize,
    pub hit_rate: f64,
}

impl StringInterningStats {
    /// Форматировать статистику
    pub fn format(&self) -> String {
        format!(
            "🧵 String Interning:\n\
             📝 Уникальных строк: {}\n\
             🔄 Всего запросов: {}\n\
             💾 Сэкономлено: {:.2} KB\n\
             🎯 Hit rate: {:.1}%",
            self.unique_strings,
            self.total_requests,
            self.saved_bytes as f64 / 1024.0,
            self.hit_rate * 100.0
        )
    }
}

/// Менеджер оптимизации памяти
pub struct MemoryOptimizationManager {
    monitor: MemoryMonitor,
    interner: StringInterner,
}

impl Default for MemoryOptimizationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryOptimizationManager {
    /// Создать новый менеджер
    pub fn new() -> Self {
        Self {
            monitor: MemoryMonitor::new(),
            interner: StringInterner::new(),
        }
    }
    
    /// Получить отчет об оптимизации
    pub fn generate_optimization_report(&self) -> MemoryOptimizationReport {
        let memory_stats = self.monitor.get_memory_stats();
        let string_stats = self.interner.get_stats();
        
        MemoryOptimizationReport {
            memory_stats,
            string_interning_stats: string_stats,
            total_saved_bytes: 0, // TODO: Вычислить реальную экономию
        }
    }
}

/// Отчет об оптимизации памяти
#[derive(Debug, Clone)]
pub struct MemoryOptimizationReport {
    pub memory_stats: Option<MemoryStats>,
    pub string_interning_stats: StringInterningStats,
    pub total_saved_bytes: usize,
}

impl MemoryOptimizationReport {
    /// Форматировать отчет
    pub fn format_human_readable(&self) -> String {
        let mut report = String::new();
        
        report.push_str("🧠 Отчет об оптимизации памяти\n\n");
        
        if let Some(ref stats) = self.memory_stats {
            report.push_str(&stats.format());
            report.push('\n');
        }
        
        report.push_str(&format!(
            "\n💾 Общая экономия: {:.2} KB\n\n",
            self.total_saved_bytes as f64 / 1024.0
        ));
        
        report.push_str(&self.string_interning_stats.format());
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    fn create_test_context() -> TypeContext {
        let mut variables = HashMap::new();
        variables.insert("var1".to_string(), crate::core::standard_types::primitive_type(
            crate::core::types::PrimitiveType::String
        ));
        variables.insert("var2".to_string(), crate::core::standard_types::primitive_type(
            crate::core::types::PrimitiveType::Number
        ));
        
        TypeContext {
            variables,
            functions: HashMap::new(),
            current_scope: crate::core::dependency_graph::Scope::Global,
            scope_stack: vec![],
        }
    }
    
    #[test]
    fn test_memory_monitor() {
        let mut monitor = MemoryMonitor::new();
        let context = create_test_context();
        
        let profile = monitor.take_snapshot(&[&context]);
        
        assert!(profile.total_memory_bytes > 0);
        assert!(profile.object_counts.type_resolutions > 0);
        assert_eq!(monitor.profiles.len(), 1);
    }
    
    #[test]
    fn test_string_interner() {
        let mut interner = StringInterner::new();
        
        let s1 = interner.intern("test");
        let s2 = interner.intern("test");
        let s3 = interner.intern("other");
        
        // Одинаковые строки должны быть тем же Arc
        assert!(Arc::ptr_eq(&s1, &s2));
        assert!(!Arc::ptr_eq(&s1, &s3));
        
        let stats = interner.get_stats();
        assert_eq!(stats.unique_strings, 2);
        assert_eq!(stats.total_requests, 3);
        assert!(stats.hit_rate > 0.0);
    }
    
    #[test]
    fn test_memory_optimization_manager() {
        let manager = MemoryOptimizationManager::new();
        
        let report = manager.generate_optimization_report();
        // Должен создаваться без ошибок
        assert!(report.format_human_readable().contains("🧠 Отчет об оптимизации памяти"));
    }
}