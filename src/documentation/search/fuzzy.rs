//! Нечеткий поиск (Fuzzy Search) для BSL документации
//! 
//! Реализует алгоритмы для поиска с опечатками и неточными совпадениями

use std::collections::HashMap;

/// Нечеткий поисковик
#[derive(Debug)]
pub struct FuzzyMatcher {
    /// Максимальное расстояние для считания совпадением
    max_distance: usize,
    
    /// Минимальный коэффициент схожести (0.0 - 1.0)
    min_similarity: f64,
    
    /// Кеш для расчетов расстояний
    distance_cache: HashMap<(String, String), usize>,
}

impl FuzzyMatcher {
    /// Создать новый fuzzy matcher
    pub fn new(max_distance: usize, min_similarity: f64) -> Self {
        Self {
            max_distance,
            min_similarity,
            distance_cache: HashMap::new(),
        }
    }
    
    /// Создать с настройками по умолчанию для BSL
    pub fn default_for_bsl() -> Self {
        Self::new(3, 0.6) // Максимум 3 ошибки, минимум 60% схожести
    }
    
    /// Найти fuzzy совпадения для запроса в списке терминов
    pub fn find_matches(&mut self, query: &str, terms: &[String]) -> Vec<FuzzyMatch> {
        let mut matches = Vec::new();
        
        for term in terms {
            if let Some(fuzzy_match) = self.calculate_match(query, term) {
                matches.push(fuzzy_match);
            }
        }
        
        // Сортируем по убыванию схожести
        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        
        matches
    }
    
    /// Рассчитать fuzzy совпадение между двумя строками
    pub fn calculate_match(&mut self, query: &str, term: &str) -> Option<FuzzyMatch> {
        // Точное совпадение
        if query.eq_ignore_ascii_case(term) {
            return Some(FuzzyMatch {
                term: term.to_string(),
                distance: 0,
                similarity: 1.0,
                match_type: FuzzyMatchType::Exact,
                highlighted: term.to_string(),
            });
        }
        
        // Проверка префикса
        if term.to_lowercase().starts_with(&query.to_lowercase()) {
            return Some(FuzzyMatch {
                term: term.to_string(),
                distance: 0,
                similarity: 0.95,
                match_type: FuzzyMatchType::Prefix,
                highlighted: self.highlight_prefix(term, query.len()),
            });
        }
        
        // Проверка содержания
        if term.to_lowercase().contains(&query.to_lowercase()) {
            return Some(FuzzyMatch {
                term: term.to_string(),
                distance: 0,
                similarity: 0.8,
                match_type: FuzzyMatchType::Contains,
                highlighted: self.highlight_substring(term, query),
            });
        }
        
        // Расчет расстояния Левенштейна
        let distance = self.levenshtein_distance(query, term);
        
        if distance <= self.max_distance {
            let similarity = self.calculate_similarity(query, term, distance);
            
            if similarity >= self.min_similarity {
                return Some(FuzzyMatch {
                    term: term.to_string(),
                    distance,
                    similarity,
                    match_type: FuzzyMatchType::Fuzzy,
                    highlighted: self.highlight_fuzzy_match(term, query),
                });
            }
        }
        
        None
    }
    
    /// Рассчитать расстояние Левенштейна между двумя строками
    pub fn levenshtein_distance(&mut self, s1: &str, s2: &str) -> usize {
        let key = (s1.to_lowercase(), s2.to_lowercase());
        
        // Проверяем кеш
        if let Some(&cached_distance) = self.distance_cache.get(&key) {
            return cached_distance;
        }
        
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let s1_len = s1_chars.len();
        let s2_len = s2_chars.len();
        
        // Создаем матрицу расстояний
        let mut matrix = vec![vec![0; s2_len + 1]; s1_len + 1];
        
        // Инициализация первой строки и столбца
        for i in 0..=s1_len {
            matrix[i][0] = i;
        }
        for j in 0..=s2_len {
            matrix[0][j] = j;
        }
        
        // Заполняем матрицу
        for i in 1..=s1_len {
            for j in 1..=s2_len {
                let cost = if s1_chars[i - 1].to_lowercase().next() == s2_chars[j - 1].to_lowercase().next() {
                    0
                } else {
                    1
                };
                
                matrix[i][j] = std::cmp::min(
                    std::cmp::min(
                        matrix[i - 1][j] + 1,      // Удаление
                        matrix[i][j - 1] + 1       // Вставка
                    ),
                    matrix[i - 1][j - 1] + cost    // Замена
                );
            }
        }
        
        let distance = matrix[s1_len][s2_len];
        
        // Кешируем результат
        self.distance_cache.insert(key, distance);
        
        distance
    }
    
    /// Рассчитать коэффициент схожести (0.0 - 1.0)
    fn calculate_similarity(&self, s1: &str, s2: &str, distance: usize) -> f64 {
        let max_len = std::cmp::max(s1.len(), s2.len());
        if max_len == 0 {
            return 1.0;
        }
        
        1.0 - (distance as f64 / max_len as f64)
    }
    
    /// Подсветить префикс
    fn highlight_prefix(&self, term: &str, prefix_len: usize) -> String {
        if prefix_len >= term.len() {
            format!("<mark>{}</mark>", term)
        } else {
            format!("<mark>{}</mark>{}", &term[..prefix_len], &term[prefix_len..])
        }
    }
    
    /// Подсветить подстроку
    fn highlight_substring(&self, term: &str, query: &str) -> String {
        let term_lower = term.to_lowercase();
        let query_lower = query.to_lowercase();
        
        if let Some(pos) = term_lower.find(&query_lower) {
            let end_pos = pos + query.len();
            format!("{}<mark>{}</mark>{}", 
                &term[..pos], 
                &term[pos..end_pos], 
                &term[end_pos..]
            )
        } else {
            term.to_string()
        }
    }
    
    /// Подсветить fuzzy совпадение (упрощенная версия)
    fn highlight_fuzzy_match(&self, term: &str, _query: &str) -> String {
        // Для fuzzy совпадений пока простая подсветка всего термина
        format!("<mark>{}</mark>", term)
    }
    
    /// Очистить кеш расстояний
    pub fn clear_cache(&mut self) {
        self.distance_cache.clear();
    }
    
    /// Получить статистику кеша
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            entries_count: self.distance_cache.len(),
            memory_estimate_bytes: self.distance_cache.len() * 64, // Примерная оценка
        }
    }
}

/// Результат fuzzy совпадения
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    /// Найденный термин
    pub term: String,
    
    /// Расстояние Левенштейна
    pub distance: usize,
    
    /// Коэффициент схожести (0.0 - 1.0)
    pub similarity: f64,
    
    /// Тип совпадения
    pub match_type: FuzzyMatchType,
    
    /// Текст с подсветкой
    pub highlighted: String,
}

/// Тип fuzzy совпадения
#[derive(Debug, Clone, PartialEq)]
pub enum FuzzyMatchType {
    /// Точное совпадение
    Exact,
    
    /// Совпадение по префиксу
    Prefix,
    
    /// Содержит подстроку
    Contains,
    
    /// Нечеткое совпадение
    Fuzzy,
}

/// Статистика кеша
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Количество записей в кеше
    pub entries_count: usize,
    
    /// Примерный объем памяти в байтах
    pub memory_estimate_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_levenshtein_distance() {
        let mut matcher = FuzzyMatcher::default_for_bsl();
        
        // Точные совпадения
        assert_eq!(matcher.levenshtein_distance("cat", "cat"), 0);
        assert_eq!(matcher.levenshtein_distance("", ""), 0);
        
        // Простые случаи
        assert_eq!(matcher.levenshtein_distance("cat", "bat"), 1);
        assert_eq!(matcher.levenshtein_distance("cat", "cats"), 1);
        assert_eq!(matcher.levenshtein_distance("cat", ""), 3);
        
        // BSL специфичные тесты
        assert_eq!(matcher.levenshtein_distance("ТаблицаЗначений", "ТаблицаЗначении"), 1);
        assert_eq!(matcher.levenshtein_distance("Справочники", "Справочнки"), 1);
    }
    
    #[test]
    fn test_fuzzy_matching() {
        let mut matcher = FuzzyMatcher::default_for_bsl();
        
        let terms = vec![
            "ТаблицаЗначений".to_string(),
            "СписокЗначений".to_string(),
            "ДеревоЗначений".to_string(),
            "Справочники".to_string(),
            "Документы".to_string(),
        ];
        
        // Тест с опечаткой
        let matches = matcher.find_matches("ТаблицаЗначений", &terms);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].match_type, FuzzyMatchType::Exact);
        
        // Тест с частичным совпадением
        let matches = matcher.find_matches("Таблица", &terms);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].match_type, FuzzyMatchType::Prefix);
        
        // Тест с опечаткой
        let matches = matcher.find_matches("ТаблицаЗначении", &terms);
        assert!(!matches.is_empty());
        assert!(matches[0].similarity > 0.8);
    }
    
    #[test]
    fn test_cache_functionality() {
        let mut matcher = FuzzyMatcher::default_for_bsl();
        
        // Первый вызов
        let distance1 = matcher.levenshtein_distance("test", "best");
        
        // Второй вызов (должен использовать кеш)
        let distance2 = matcher.levenshtein_distance("test", "best");
        
        assert_eq!(distance1, distance2);
        
        let stats = matcher.cache_stats();
        assert!(stats.entries_count > 0);
        
        matcher.clear_cache();
        let stats_after_clear = matcher.cache_stats();
        assert_eq!(stats_after_clear.entries_count, 0);
    }
}