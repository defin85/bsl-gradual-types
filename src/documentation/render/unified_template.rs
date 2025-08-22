//! Унифицированная система шаблонов для всех страниц

/// Базовый шаблон для всех страниц
pub struct UnifiedPageTemplate {
    /// Заголовок страницы
    pub title: String,

    /// Активная секция навигации
    pub active_section: String,

    /// Статистика для header
    pub stats: PageStatistics,

    /// Основной контент
    pub content: String,

    /// Активная тема
    pub theme: String,
}

/// Статистика для отображения в header
#[derive(Debug, Clone)]
pub struct PageStatistics {
    pub functions_count: usize,
    pub variables_count: usize,
    pub platform_types_count: usize,
    pub memory_usage_mb: f32,
}

impl UnifiedPageTemplate {
    /// Создать новый шаблон
    pub fn new(title: &str, active_section: &str) -> Self {
        Self {
            title: title.to_string(),
            active_section: active_section.to_string(),
            stats: PageStatistics::default(),
            content: String::new(),
            theme: "dark".to_string(),
        }
    }

    /// Установить статистику
    pub fn with_stats(mut self, stats: PageStatistics) -> Self {
        self.stats = stats;
        self
    }

    /// Установить контент
    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    /// Установить тему
    pub fn with_theme(mut self, theme: String) -> Self {
        self.theme = theme;
        self
    }

    /// Рендеринг полной страницы
    pub fn render(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="ru" class="theme-{}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    {}
    {}
</head>
<body class="theme-{}">
    <div class="page-layout">
        {}
        <div class="page-content">
            {}
        </div>
        {}
    </div>
    {}
</body>
</html>"#,
            self.theme,               // html class
            self.title,               // title
            self.render_shared_css(), // CSS
            self.render_page_css(),   // Дополнительный CSS
            self.theme,               // body class
            self.render_header(),     // header
            self.content,             // main content
            self.render_footer(),     // footer
            self.render_javascript()  // JavaScript
        )
    }

    /// Рендеринг общих CSS стилей
    fn render_shared_css(&self) -> String {
        // Включаем содержимое shared_styles.css
        let shared_css = include_str!("shared_styles.css");
        format!("<style>\n{}\n</style>", shared_css)
    }

    /// Рендеринг дополнительных CSS для конкретной страницы
    fn render_page_css(&self) -> String {
        match self.active_section.as_str() {
            "hierarchy" => self.render_hierarchy_css(),
            "search" => self.render_search_css(),
            _ => String::new(),
        }
    }

    /// Рендеринг header со статистикой и навигацией
    fn render_header(&self) -> String {
        format!(
            r#"<header class="page-header">
                <div class="theme-switcher">
                    <button class="theme-btn" onclick="switchTheme('dark')">🌙 Темная</button>
                    <button class="theme-btn" onclick="switchTheme('light')">☀️ Светлая</button>
                    <button class="theme-btn" onclick="switchTheme('vscode')">💻 VSCode</button>
                </div>
                
                <div class="header-brand">
                    <h1>🚀 BSL Type Browser</h1>
                    <p>Production-ready система типов для 1С:Предприятие</p>
                </div>
                
                <nav class="main-navigation">
                    <a href="/" class="nav-tab {}">🏠 Главная</a>
                    <a href="/hierarchy" class="nav-tab {}">🌳 Иерархия</a>
                    <a href="/search" class="nav-tab {}">🔍 Поиск</a>
                    <a href="/analyzer" class="nav-tab {}">⚡ Анализатор</a>
                    <a href="/stats" class="nav-tab {}">📊 Статистика</a>
                    <a href="/api" class="nav-tab {}">🔗 API</a>
                </nav>
                
                {}
            </header>"#,
            if self.active_section == "home" {
                "active"
            } else {
                ""
            },
            if self.active_section == "hierarchy" {
                "active"
            } else {
                ""
            },
            if self.active_section == "search" {
                "active"
            } else {
                ""
            },
            if self.active_section == "analyzer" {
                "active"
            } else {
                ""
            },
            if self.active_section == "stats" {
                "active"
            } else {
                ""
            },
            if self.active_section == "api" {
                "active"
            } else {
                ""
            },
            self.render_stats_section()
        )
    }

    /// Условный рендеринг статистики (только для главной страницы)
    fn render_stats_section(&self) -> String {
        if self.active_section == "home" {
            format!(
                r#"<div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-value">{}</div>
                        <div class="stat-label">Функций</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value">{}</div>
                        <div class="stat-label">Переменных</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value">{}</div>
                        <div class="stat-label">Платформенных типов</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value">{:.1}</div>
                        <div class="stat-label">Память (MB)</div>
                    </div>
                </div>"#,
                self.stats.functions_count,
                self.stats.variables_count,
                self.stats.platform_types_count,
                self.stats.memory_usage_mb
            )
        } else {
            // Для других страниц - только компактная информация
            format!(
                r#"<div class="compact-stats">
                    <span class="compact-stat">📊 {} типов</span>
                    <span class="compact-stat">💾 {:.1} MB</span>
                </div>"#,
                self.stats.platform_types_count, self.stats.memory_usage_mb
            )
        }
    }

    /// Рендеринг footer
    fn render_footer(&self) -> String {
        r#"<footer class="page-footer">
            <p>BSL Gradual Type System v1.0.0 | Enterprise Documentation | 
               <a href="https://github.com/your-repo/bsl-gradual-types" style="color: var(--accent-color);">GitHub</a>
            </p>
        </footer>"#.to_string()
    }

    /// CSS специфичный для страницы иерархии
    fn render_hierarchy_css(&self) -> String {
        r#"<style>
/* Специфичные стили для страницы иерархии */
.hierarchy-layout {
    display: flex;
    height: 100%;
}

.hierarchy-sidebar {
    width: var(--sidebar-width);
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
}

.hierarchy-content {
    flex: 1;
    padding: var(--spacing-xl);
    background: var(--bg-primary);
}

/* Переопределяем стили дерева для единообразия */
.tree-node {
    font-family: var(--font-family);
    font-size: 0.9em;
}

.tree-node.selected {
    background: var(--accent-color);
    color: white;
}

.tree-node:hover {
    background: var(--bg-tertiary);
}

/* Стили для исправленной иерархии типов */
.type-hierarchy-fixed {
    padding: var(--spacing-lg);
    background: var(--bg-primary);
    border-radius: var(--border-radius);
    margin: var(--spacing-md) 0;
}

.system-status {
    background: var(--bg-secondary);
    padding: var(--spacing-md);
    border-radius: var(--border-radius);
    margin: var(--spacing-md) 0;
    border-left: 4px solid var(--success-color, #4CAF50);
}

.system-status p {
    margin: var(--spacing-xs) 0;
    color: var(--text-secondary);
}

.categories-list {
    margin: var(--spacing-lg) 0;
}

.category-item {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    margin: var(--spacing-md) 0;
    padding: var(--spacing-md);
    transition: all 0.2s ease;
}

.category-item:hover {
    border-color: var(--accent-color);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.category-item h4 {
    color: var(--accent-color);
    margin: 0 0 var(--spacing-sm) 0;
    font-size: 1.1em;
}

.types-in-category {
    display: grid;
    gap: var(--spacing-xs);
}

.type-item {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs);
    background: var(--bg-primary);
    border-radius: calc(var(--border-radius) / 2);
    align-items: center;
}

.type-name {
    font-family: var(--font-mono);
    color: var(--primary-color);
    font-weight: 500;
}

.type-certainty {
    background: var(--accent-color);
    color: white;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 0.75em;
    text-transform: uppercase;
}

.type-facets {
    color: var(--text-secondary);
    font-size: 0.8em;
    font-family: var(--font-mono);
}

.more-types {
    color: var(--text-secondary);
    font-style: italic;
    text-align: center;
    padding: var(--spacing-xs);
    background: var(--bg-tertiary);
    border-radius: calc(var(--border-radius) / 2);
}

.success-message {
    background: var(--success-bg, #e8f5e8);
    border: 1px solid var(--success-color, #4CAF50);
    border-radius: var(--border-radius);
    padding: var(--spacing-lg);
    margin: var(--spacing-lg) 0;
}

.success-message h4 {
    color: var(--success-color, #4CAF50);
    margin: 0 0 var(--spacing-sm) 0;
}

.success-message ul {
    margin: var(--spacing-sm) 0;
    padding-left: var(--spacing-lg);
}

.success-message li {
    margin: var(--spacing-xs) 0;
    color: var(--text-primary);
}

/* Цвета для разных тем */
:root.theme-dark {
    --success-color: #4CAF50;
    --success-bg: #1e2e1e;
}

:root.theme-light {
    --success-color: #2E7D32;
    --success-bg: #f1f8e9;
}

:root.theme-vscode {
    --success-color: #4EC9B0;
    --success-bg: #1e1e1e;
}

/* Стили для правильной иерархии */
.category-description {
    background: var(--bg-tertiary);
    padding: var(--spacing-sm);
    border-radius: calc(var(--border-radius) / 2);
    margin: var(--spacing-sm) 0;
    border-left: 3px solid var(--accent-color);
}

.category-description p {
    margin: var(--spacing-xs) 0;
    color: var(--text-secondary);
    font-size: 0.9em;
}

.subcategories-section, .types-section {
    margin: var(--spacing-md) 0;
    padding: var(--spacing-sm);
    background: var(--bg-tertiary);
    border-radius: calc(var(--border-radius) / 2);
}

.subcategories-section h5, .types-section h5 {
    margin: 0 0 var(--spacing-sm) 0;
    color: var(--accent-color);
    font-size: 0.9em;
    text-transform: uppercase;
    font-weight: 600;
}

.subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-xs);
    margin: var(--spacing-xs) 0;
    background: var(--bg-primary);
    border-radius: calc(var(--border-radius) / 3);
    border-left: 2px solid var(--accent-color);
}

.subcategory-name {
    font-family: var(--font-mono);
    color: var(--primary-color);
    font-weight: 500;
}

.subcategory-counts {
    color: var(--text-secondary);
    font-size: 0.8em;
    font-style: italic;
}

.more-subcategories {
    color: var(--text-secondary);
    font-style: italic;
    text-align: center;
    padding: var(--spacing-xs);
    background: var(--bg-secondary);
    border-radius: calc(var(--border-radius) / 3);
    margin: var(--spacing-xs) 0;
}
</style>"#
            .to_string()
    }

    /// CSS для страницы поиска
    fn render_search_css(&self) -> String {
        r#"<style>
/* Специфичные стили для страницы поиска */
.search-layout {
    padding: var(--spacing-xl);
}
</style>"#
            .to_string()
    }

    /// Рендеринг JavaScript
    fn render_javascript(&self) -> String {
        r#"<script>
// === УНИФИЦИРОВАННАЯ СИСТЕМА НАВИГАЦИИ ===

// Глобальное состояние
window.bslBrowser = {
    currentTheme: localStorage.getItem('bsl-theme') || 'dark',
    currentSection: '',
    isLoading: false
};

// === ПЕРЕКЛЮЧЕНИЕ ТЕМ ===
function switchTheme(themeName) {
    // Обновляем CSS класс
    document.documentElement.className = 'theme-' + themeName;
    document.body.className = 'theme-' + themeName;
    
    // Сохраняем в localStorage
    localStorage.setItem('bsl-theme', themeName);
    window.bslBrowser.currentTheme = themeName;
    
    // Обновляем активную кнопку темы
    document.querySelectorAll('.theme-btn').forEach(btn => {
        btn.classList.remove('active');
    });
    document.querySelector(`[onclick="switchTheme('${themeName}')"]`)?.classList.add('active');
    
    console.log('🎨 Theme switched to:', themeName);
}

// === SPA-ПОДОБНАЯ НАВИГАЦИЯ ===
function navigateToSection(section, pushState = true) {
    if (window.bslBrowser.isLoading) return;
    
    window.bslBrowser.isLoading = true;
    window.bslBrowser.currentSection = section;
    
    // Обновляем активную вкладку
    document.querySelectorAll('.nav-tab').forEach(tab => {
        tab.classList.remove('active');
    });
    document.querySelector(`[href="/${section}"]`)?.classList.add('active');
    
    // Обновляем URL без перезагрузки
    if (pushState) {
        const url = section === 'home' ? '/' : `/${section}`;
        history.pushState({section}, '', url);
    }
    
    // Показываем индикатор загрузки
    showLoadingIndicator();
    
    // Загружаем контент (если это SPA)
    if (window.loadSectionContent) {
        window.loadSectionContent(section);
    }
    
    window.bslBrowser.isLoading = false;
}

// === ИНДИКАТОР ЗАГРУЗКИ ===
function showLoadingIndicator() {
    const indicator = document.getElementById('loading-indicator');
    if (indicator) {
        indicator.style.display = 'block';
    }
}

function hideLoadingIndicator() {
    const indicator = document.getElementById('loading-indicator');
    if (indicator) {
        indicator.style.display = 'none';
    }
}

// === УВЕДОМЛЕНИЯ ===
function showNotification(message, type = 'info') {
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.innerHTML = `
        <div class="notification-content">
            <span class="notification-text">${message}</span>
            <button class="notification-close" onclick="this.parentElement.parentElement.remove()">×</button>
        </div>
    `;
    
    notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        background: ${type === 'error' ? 'var(--error-color)' : 'var(--accent-color)'};
        color: white;
        padding: 1rem;
        border-radius: var(--border-radius);
        z-index: 9999;
        min-width: 300px;
        animation: slideIn 0.3s ease;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    `;
    
    document.body.appendChild(notification);
    
    // Автоматическое удаление через 5 секунд
    setTimeout(() => {
        if (notification.parentElement) {
            notification.style.animation = 'slideOut 0.3s ease';
            setTimeout(() => notification.remove(), 300);
        }
    }, 5000);
}

// === ИНИЦИАЛИЗАЦИЯ ===
document.addEventListener('DOMContentLoaded', function() {
    // Применяем сохраненную тему
    switchTheme(window.bslBrowser.currentTheme);
    
    // Определяем текущую секцию по URL
    const path = window.location.pathname;
    const section = path === '/' ? 'home' : path.substring(1);
    window.bslBrowser.currentSection = section;
    
    // Обновляем активную вкладку
    document.querySelectorAll('.nav-tab').forEach(tab => {
        tab.classList.remove('active');
        if (tab.getAttribute('href') === (section === 'home' ? '/' : `/${section}`)) {
            tab.classList.add('active');
        }
    });
    
    // Специальная инициализация для страницы иерархии
    if (section === 'hierarchy') {
        setTimeout(() => {
            // Инициализируем состояние дерева с развернутыми корневыми узлами
            document.querySelectorAll('.tree-node[data-node-type="Category"]').forEach(categoryNode => {
                const nodeId = categoryNode.dataset.nodeId;
                if (nodeId) {
                    treeState.expandedNodes.add(nodeId);
                    categoryNode.classList.add('expanded');
                    
                    // Разворачиваем дочерние элементы
                    const children = document.querySelector(`.tree-children[data-parent-id="${nodeId}"]`);
                    if (children) {
                        children.style.display = 'block';
                    }
                    
                    // Обновляем индикатор
                    const indicator = categoryNode.querySelector('.expand-indicator');
                    if (indicator) {
                        indicator.textContent = '▼';
                        indicator.dataset.expanded = 'true';
                    }
                    
                    console.log('📂 Автоматически развернули категорию:', nodeId);
                }
            });
            
            console.log('📊 Expanded nodes initialized:', Array.from(treeState.expandedNodes));
        }, 100);
    }
    
    // Обработка кнопок назад/вперед в браузере
    window.addEventListener('popstate', function(event) {
        if (event.state && event.state.section) {
            navigateToSection(event.state.section, false);
        }
    });
    
    console.log('🚀 BSL Browser Unified UI initialized');
    console.log('📍 Current section:', section);
    console.log('🎨 Current theme:', window.bslBrowser.currentTheme);
});

// === ДОПОЛНИТЕЛЬНЫЕ АНИМАЦИИ ===
const additionalCSS = `
<style>
@keyframes slideOut {
    from { transform: translateX(0); opacity: 1; }
    to { transform: translateX(100%); opacity: 0; }
}

.notification {
    animation: slideIn 0.3s ease;
}

.notification-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.notification-close {
    background: none;
    border: none;
    color: white;
    font-size: 1.2em;
    cursor: pointer;
    padding: 0 0 0 10px;
}

.loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 10000;
}

.loading-spinner {
    width: 60px;
    height: 60px;
    border: 4px solid rgba(255, 255, 255, 0.1);
    border-left: 4px solid var(--accent-color);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}
</style>
`;

// Добавляем CSS в head
document.head.insertAdjacentHTML('beforeend', additionalCSS);

// === ФУНКЦИИ ДЛЯ ГЛАВНОЙ СТРАНИЦЫ ===

// Поиск типов
function performSearch(query) {
    const resultsContainer = document.getElementById('search-results');
    
    if (query.length < 2) {
        resultsContainer.innerHTML = '';
        return;
    }
    
    console.log('🔍 Performing search:', query);
    resultsContainer.innerHTML = '<div class="loading">🔄 Поиск...</div>';
    
    // Используем правильный API endpoint
    fetch(`/api/types?search=${encodeURIComponent(query)}&per_page=10`)
        .then(response => {
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }
            return response.json();
        })
        .then(data => {
            displaySearchResults(data.types || data.results || data);
        })
        .catch(error => {
            console.error('Search error:', error);
            resultsContainer.innerHTML = `<div class="error">❌ Ошибка поиска: ${error.message}</div>`;
        });
}

// Отображение результатов поиска
function displaySearchResults(results) {
    const container = document.getElementById('search-results');
    if (!container) return;
    
    // Обрабатываем разные форматы ответа API
    let searchResults = [];
    if (Array.isArray(results)) {
        searchResults = results;
    } else if (results.types) {
        searchResults = results.types;
    } else if (results.results) {
        searchResults = results.results;
    }
    
    if (searchResults.length === 0) {
        container.innerHTML = '<div class="no-results">📭 Типы не найдены. Попробуйте другой поисковый запрос.</div>';
        return;
    }
    
    const html = searchResults.slice(0, 8).map(result => `
        <div class="search-result-card" onclick="openTypeDetails('${result.name || result.id}')">
            <h3>${result.name || result.id}</h3>
            <p>${result.description || result.russian_name || 'Описание отсутствует'}</p>
            <span class="result-category">${result.category || result.type_name || 'Тип'}</span>
        </div>
    `).join('');
    
    container.innerHTML = `
        <div class="search-results-header">
            <h3>🔍 Результаты поиска (${searchResults.length})</h3>
        </div>
        ${html}
        ${searchResults.length > 8 ? `<div class="more-results">... и еще ${searchResults.length - 8} результатов</div>` : ''}
    `;
}

// Открытие деталей типа
function openTypeDetails(typeName) {
    console.log('🔍 Opening details for:', typeName);
    // Перенаправляем на страницу иерархии с выбранным типом
    window.location.href = `/hierarchy#type-${encodeURIComponent(typeName)}`;
}

// Анализ кода
function analyzeCode() {
    const code = document.getElementById('code-input').value.trim();
    
    if (code.length === 0) {
        showNotification('Введите код для анализа', 'warning');
        return;
    }
    
    console.log('🔍 Analyzing code:', code);
    
    const resultsContainer = document.getElementById('analysis-results');
    resultsContainer.innerHTML = '<div class="loading">🔄 Анализ кода...</div>';
    
    // Временная заглушка для анализа (пока tree-sitter не работает)
    setTimeout(() => {
        const mockResults = {
            types_found: code.split(/\b(Массив|Структура|ТаблицаЗначений|Строка|Число|Булево)\b/gi).length - 1,
            functions_found: (code.match(/\bФункция\s+\w+/gi) || []).length,
            variables_found: (code.match(/\bПерем\s+\w+/gi) || []).length,
            success: true
        };
        
        displayAnalysisResults(mockResults);
        showNotification('Анализ завершен (demo режим)', 'info');
    }, 1000);
    
    // TODO: Раскомментировать когда tree-sitter будет работать
    /*
    fetch('/api/analyze', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ code: code })
    })
    .then(response => response.json())
    .then(data => {
        displayAnalysisResults(data);
    })
    .catch(error => {
        console.error('Analysis error:', error);
        resultsContainer.innerHTML = `<div class="error">❌ Ошибка анализа: ${error.message}</div>`;
    });
    */
}

// Отображение результатов анализа
function displayAnalysisResults(results) {
    const container = document.getElementById('analysis-results');
    
    if (results.errors && results.errors.length > 0) {
        container.innerHTML = `
            <div class="analysis-errors">
                <h4>🚨 Ошибки:</h4>
                ${results.errors.map(error => `<div class="error-item">${error}</div>`).join('')}
            </div>
        `;
        return;
    }
    
    container.innerHTML = `
        <div class="analysis-success">
            <h4>✅ Анализ завершен успешно</h4>
            <div class="analysis-details">
                <p><strong>Обнаружено типов:</strong> ${results.types_found || 0}</p>
                <p><strong>Функций:</strong> ${results.functions_found || 0}</p>
                <p><strong>Переменных:</strong> ${results.variables_found || 0}</p>
            </div>
        </div>
    `;
}

// Очистка кода
function clearCode() {
    document.getElementById('code-input').value = '';
    document.getElementById('analysis-results').innerHTML = '';
    showNotification('Код очищен', 'info');
}

// Загрузка примера
function loadExample() {
    const exampleCode = `Функция ТестоваяФункция(Параметр)
    Перем Результат;
    
    Если ТипЗнч(Параметр) = Тип("Строка") Тогда
        Результат = Строка(Параметр);
    Иначе
        Результат = "";
    КонецЕсли;
    
    Возврат Результат;
КонецФункции

Процедура ТестоваяПроцедура()
    Перем Массив, Структура, ТаблицаЗначений;
    
    Массив = Новый Массив;
    Структура = Новый Структура("Поле1, Поле2");
    ТаблицаЗначений = Новый ТаблицаЗначений;
    
    Сообщить("Создано объектов: " + Массив.Количество());
КонецПроцедуры`;

    document.getElementById('code-input').value = exampleCode;
    document.getElementById('analysis-results').innerHTML = '';
    showNotification('Пример загружен', 'info');
}

// === ИНТЕРАКТИВНОЕ ДЕРЕВО - ФУНКЦИИ ===

// Состояние дерева
let treeState = {
    expandedNodes: new Set(),
    selectedNode: null,
    searchQuery: '',
    draggedNode: null,
    contextMenu: null
};

// === КЛИК ПО УЗЛУ ===
function handleNodeClick(event, nodeId) {
    event.stopPropagation();
    
    // Убираем предыдущее выделение
    document.querySelectorAll('.tree-node.selected').forEach(node => {
        node.classList.remove('selected');
    });
    
    // Выделяем текущий узел
    const node = document.getElementById(`node_${nodeId}`);
    if (node) {
        node.classList.add('selected');
        treeState.selectedNode = nodeId;
        
        // Загружаем детали узла
        const nodeType = node.dataset.nodeType;
        loadNodeDetails(nodeId, nodeType);
    }
}

// === РАСКРЫТИЕ/СВОРАЧИВАНИЕ УЗЛА ===
function toggleNodeExpansion(event, nodeId) {
    if (event && event.stopPropagation) {
        event.stopPropagation();
    }
    
    const node = document.getElementById(`node_${nodeId}`);
    if (!node) {
        console.log('❌ Узел не найден:', nodeId);
        return;
    }
    
    const hasChildren = node.dataset.hasChildren === 'true';
    const childrenLoaded = node.dataset.childrenLoaded === 'true';
    
    if (!hasChildren) {
        console.log('ℹ️ Узел не имеет дочерних элементов:', nodeId);
        return;
    }
    
    const isExpanded = treeState.expandedNodes.has(nodeId);
    const indicator = node.querySelector('.expand-indicator');
    
    console.log(`🔄 Переключаем узел ${nodeId}: ${isExpanded ? 'сворачиваем' : 'разворачиваем'}`);
    
    if (isExpanded) {
        // Сворачиваем
        treeState.expandedNodes.delete(nodeId);
        const children = document.querySelector(`.tree-children[data-parent-id="${nodeId}"]`);
        if (children) {
            children.style.display = 'none';
        }
        if (indicator) {
            indicator.textContent = '▶';
            indicator.dataset.expanded = 'false';
        }
        node.classList.remove('expanded');
        console.log('📁 Узел свернут:', nodeId);
    } else {
        // Раскрываем
        treeState.expandedNodes.add(nodeId);
        
        const children = document.querySelector(`.tree-children[data-parent-id="${nodeId}"]`);
        
        if (!childrenLoaded) {
            console.log('⏳ Загружаем дочерние узлы для:', nodeId);
            // Показываем загруженные элементы
            if (children) {
                children.style.display = 'block';
            }
        } else {
            // Показываем уже загруженные узлы
            if (children) {
                children.style.display = 'block';
            }
        }
        
        if (indicator) {
            indicator.textContent = '▼';
            indicator.dataset.expanded = 'true';
        }
        node.classList.add('expanded');
        console.log('📂 Узел развернут:', nodeId);
    }
}

// === РАЗВЕРНУТЬ ВСЕ УЗЛЫ ===
function expandAllNodes() {
    console.log('🔄 Разворачиваем все узлы...');
    document.querySelectorAll('.tree-node[data-has-children="true"]').forEach(node => {
        const nodeId = node.dataset.nodeId;
        if (nodeId && !treeState.expandedNodes.has(nodeId)) {
            console.log('📂 Разворачиваем узел:', nodeId);
            toggleNodeExpansion({ stopPropagation: () => {} }, nodeId);
        }
    });
}

// === СВЕРНУТЬ ВСЕ УЗЛЫ ===
function collapseAllNodes() {
    console.log('🔄 Сворачиваем все узлы...');
    
    // Сворачиваем все развернутые узлы
    const expandedNodes = Array.from(treeState.expandedNodes);
    expandedNodes.forEach(nodeId => {
        console.log('📁 Сворачиваем узел:', nodeId);
        toggleNodeExpansion({ stopPropagation: () => {} }, nodeId);
    });
}

// === ПОИСК В ДЕРЕВЕ ===
function searchInTree(query) {
    treeState.searchQuery = query.toLowerCase();
    
    if (query.length === 0) {
        // Показываем все узлы
        document.querySelectorAll('.tree-node').forEach(node => {
            node.style.display = 'flex';
            node.classList.remove('search-hidden', 'search-match');
        });
        return;
    }
    
    // Скрываем все узлы и показываем только совпадения
    document.querySelectorAll('.tree-node').forEach(node => {
        const title = node.querySelector('.node-title')?.textContent?.toLowerCase() || '';
        const isMatch = title.includes(query);
        
        if (isMatch) {
            node.style.display = 'flex';
            node.classList.add('search-match');
            node.classList.remove('search-hidden');
            
            // Раскрываем путь к найденному узлу
            expandPathToNode(node);
        } else {
            node.style.display = 'none';
            node.classList.add('search-hidden');
            node.classList.remove('search-match');
        }
    });
}

// === РАСКРЫТИЕ ПУТИ К УЗЛУ ===
function expandPathToNode(node) {
    let parent = node.parentElement;
    
    while (parent && !parent.classList.contains('tree-root')) {
        if (parent.classList.contains('tree-children')) {
            parent.style.display = 'block';
            
            const parentId = parent.dataset.parentId;
            if (parentId) {
                treeState.expandedNodes.add(parentId);
                
                const parentNode = document.getElementById(`node_${parentId}`);
                if (parentNode) {
                    const indicator = parentNode.querySelector('.expand-indicator');
                    if (indicator) {
                        indicator.textContent = '▼';
                        indicator.dataset.expanded = 'true';
                    }
                    parentNode.classList.add('expanded');
                }
            }
        }
        parent = parent.parentElement;
    }
}

// === ОЧИСТКА ПОИСКА ===
function clearTreeSearch() {
    const searchInput = document.getElementById('tree-search-input');
    if (searchInput) {
        searchInput.value = '';
        searchInTree('');
    }
}

// === ЗАГРУЗКА ДЕТАЛЕЙ УЗЛА ===
async function loadNodeDetails(nodeId, nodeType) {
    const detailsContainer = document.getElementById('type-details');
    if (!detailsContainer) return;
    
    detailsContainer.innerHTML = '<div class="loading">🔄 Загрузка деталей...</div>';
    
    // Временная заглушка для деталей
    setTimeout(() => {
        detailsContainer.innerHTML = `
            <div class="node-details">
                <div class="details-header">
                    <h2>📄 ${nodeId}</h2>
                    <div class="node-type-badge ${nodeType}">${nodeType}</div>
                </div>
                
                <div class="description">
                    <p>Детальная информация о типе будет добавлена в следующих версиях.</p>
                    <p><strong>Тип узла:</strong> ${nodeType}</p>
                    <p><strong>ID:</strong> ${nodeId}</p>
                </div>
                
                <div class="placeholder-message">
                    <h3>🚧 В разработке</h3>
                    <p>Детальная информация о методах, свойствах и примерах использования будет добавлена в Milestone 3.3.</p>
                </div>
            </div>
        `;
    }, 500);
}

// === ФУНКЦИИ ДЛЯ ИЕРАРХИИ КАТЕГОРИЙ ===
function toggleCategory(categoryId) {
    const categoryElement = document.getElementById(`category_${categoryId}`);
    const expandIndicator = document.querySelector(`[onclick="toggleCategory('${categoryId}')"] .expand-indicator`);
    
    if (categoryElement.style.display === 'none' || categoryElement.style.display === '') {
        categoryElement.style.display = 'block';
        expandIndicator.textContent = '▼';
        console.log('📂 Развернули категорию:', categoryId);
    } else {
        categoryElement.style.display = 'none';
        expandIndicator.textContent = '▶';
        console.log('📁 Свернули категорию:', categoryId);
    }
}

</script>"#.to_string()
    }
}

impl Default for PageStatistics {
    fn default() -> Self {
        Self {
            functions_count: 0,
            variables_count: 0,
            platform_types_count: 13607, // Базовое значение
            memory_usage_mb: 0.0,
        }
    }
}

/// Создание шаблона для главной страницы
pub fn create_home_template(stats: PageStatistics) -> UnifiedPageTemplate {
    let content = r#"
        <div class="home-layout-clean">
            <div class="welcome-hero">
                <div class="hero-content">
                    <h2 class="hero-title">Добро пожаловать в BSL Type Browser</h2>
                    <p class="hero-subtitle">Первая в мире enterprise-ready система градуальной типизации для языка 1С:Предприятие BSL</p>
                    
                    <div class="feature-grid">
                        <div class="feature-card">
                            <div class="feature-icon">🌳</div>
                            <h3>Иерархия типов</h3>
                            <p>Интерактивное дерево с 13,607 платформенными типами, lazy loading и поиском</p>
                            <a href="/hierarchy" class="feature-link">Открыть иерархию →</a>
                        </div>
                        
                        <div class="feature-card">
                            <div class="feature-icon">🔍</div>
                            <h3>Мощный поиск</h3>
                            <p>Полнотекстовый и fuzzy поиск по всем типам с фильтрами и автодополнением</p>
                            <a href="/search" class="feature-link">Перейти к поиску →</a>
                        </div>
                        
                        <div class="feature-card">
                            <div class="feature-icon">⚡</div>
                            <h3>Анализ кода</h3>
                            <p>Статический анализ BSL кода в реальном времени с градуальной типизацией</p>
                            <a href="/analyzer" class="feature-link">Открыть анализатор →</a>
                        </div>
                        
                        <div class="feature-card">
                            <div class="feature-icon">📊</div>
                            <h3>Статистика</h3>
                            <p>Детальная аналитика типов, метрики производительности и отчеты</p>
                            <a href="/stats" class="feature-link">Посмотреть статистику →</a>
                        </div>
                        
                        <div class="feature-card">
                            <div class="feature-icon">🔗</div>
                            <h3>API Документация</h3>
                            <p>REST API endpoints для интеграции с внешними системами</p>
                            <a href="/api" class="feature-link">Открыть API →</a>
                        </div>
                    </div>
                    
                    <div class="getting-started">
                        <h3>🚀 Быстрый старт</h3>
                        <div class="quick-actions">
                            <a href="/hierarchy" class="action-btn primary">🌳 Просмотр типов</a>
                            <a href="/search" class="action-btn secondary">🔍 Поиск типов</a>
                            <a href="/analyzer" class="action-btn secondary">⚡ Анализ кода</a>
                        </div>
                    </div>
                </div>
            </div>
        </div>"#.to_string();

    UnifiedPageTemplate::new("BSL Type Browser", "home")
        .with_stats(stats)
        .with_content(content)
}

/// Создание шаблона для страницы поиска
pub fn create_search_template(stats: PageStatistics) -> UnifiedPageTemplate {
    let content = r#"
        <div class="search-layout">
            <div class="search-hero">
                <h2 class="section-title-large">🔍 Мощный поиск по типам BSL</h2>
                <p class="section-subtitle">Полнотекстовый и fuzzy поиск по 13,607 платформенным типам с фильтрами и автодополнением</p>
            </div>
            
            <div class="search-interface">
                <div class="search-input-section">
                    <div class="search-container-large">
                        <input type="text" class="search-input-large" id="search-input" 
                               placeholder="Поиск типов BSL... (например: Массив, Структура, ТаблицаЗначений)"
                               onkeyup="performSearch(this.value)" autofocus>
                    </div>
                    <div class="search-options">
                        <label class="search-option">
                            <input type="checkbox" id="fuzzy-search" checked> 
                            <span>🔍 Нечёткий поиск</span>
                        </label>
                        <label class="search-option">
                            <input type="checkbox" id="case-sensitive"> 
                            <span>Aa Учитывать регистр</span>
                        </label>
                        <label class="search-option">
                            <input type="checkbox" id="whole-words"> 
                            <span>📝 Целые слова</span>
                        </label>
                    </div>
                </div>
                
                <div class="search-filters">
                    <div class="filter-group">
                        <label>Категория:</label>
                        <select id="category-filter" onchange="applyFilters()">
                            <option value="">Все категории</option>
                            <option value="platform">Платформенные типы</option>
                            <option value="configuration">Конфигурационные типы</option>
                        </select>
                    </div>
                    
                    <div class="filter-group">
                        <label>Тип объекта:</label>
                        <select id="object-type-filter" onchange="applyFilters()">
                            <option value="">Все типы</option>
                            <option value="object">Объекты</option>
                            <option value="manager">Менеджеры</option>
                            <option value="collection">Коллекции</option>
                        </select>
                    </div>
                    
                    <button class="btn btn-secondary" onclick="clearFilters()">🗑️ Очистить фильтры</button>
                </div>
                
                <div id="search-results" class="search-results-container"></div>
                <div id="search-suggestions" class="suggestions-container"></div>
            </div>
        </div>"#.to_string();

    UnifiedPageTemplate::new("BSL Type Search", "search")
        .with_stats(stats)
        .with_content(content)
}

/// Создание шаблона для страницы анализатора кода
pub fn create_analyzer_template(stats: PageStatistics) -> UnifiedPageTemplate {
    let content = r#"
        <div class="analyzer-layout">
            <div class="analyzer-hero">
                <h2 class="section-title-large">⚡ Анализатор BSL кода</h2>
                <p class="section-subtitle">Статический анализ кода в реальном времени с градуальной типизацией и обнаружением ошибок</p>
            </div>
            
            <div class="analyzer-interface">
                <div class="code-editor-section">
                    <div class="editor-toolbar">
                        <div class="editor-controls">
                            <button class="btn btn-secondary" onclick="loadExample()">📝 Загрузить пример</button>
                            <button class="btn btn-secondary" onclick="clearCode()">🗑️ Очистить</button>
                            <button class="btn btn-secondary" onclick="formatCode()">🎨 Форматировать</button>
                        </div>
                        <div class="editor-stats">
                            <span id="line-count">Строк: 0</span>
                            <span id="char-count">Символов: 0</span>
                        </div>
                    </div>
                    
                    <div class="code-editor-container">
                        <textarea id="code-input" class="code-editor" 
                                  placeholder="Введите BSL код для анализа...

Пример:
Функция РасчётСуммы(Слагаемое1, Слагаемое2)
    Перем Результат;
    
    Если ТипЗнч(Слагаемое1) = Тип(&quot;Число&quot;) И ТипЗнч(Слагаемое2) = Тип(&quot;Число&quot;) Тогда
        Результат = Слагаемое1 + Слагаемое2;
    Иначе
        Результат = 0;
        Сообщить(&quot;Ошибка: неверные типы параметров&quot;);
    КонецЕсли;
    
    Возврат Результат;
КонецФункции

Процедура ТестированиеТипов()
    Перем Массив, Структура, ТаблицаЗначений;
    
    Массив = Новый Массив;
    Массив.Добавить(&quot;Элемент1&quot;);
    Массив.Добавить(123);
    
    Структура = Новый Структура(&quot;Поле1, Поле2&quot;, &quot;Значение1&quot;, 456);
    
    ТаблицаЗначений = Новый ТаблицаЗначений;
    ТаблицаЗначений.Колонки.Добавить(&quot;Наименование&quot;, Новый ОписаниеТипов(&quot;Строка&quot;));
    ТаблицаЗначений.Колонки.Добавить(&quot;Количество&quot;, Новый ОписаниеТипов(&quot;Число&quot;));
КонецПроцедуры"
                                  oninput="updateEditorStats()"></textarea>
                    </div>
                    
                    <div class="analyzer-actions">
                        <button class="btn btn-large" onclick="analyzeCode()">
                            <span>⚡</span> Анализировать код
                        </button>
                        <button class="btn btn-secondary btn-large" onclick="validateSyntax()">
                            <span>✅</span> Проверить синтаксис
                        </button>
                        <button class="btn btn-secondary btn-large" onclick="exportAnalysis()">
                            <span>📤</span> Экспорт результатов
                        </button>
                    </div>
                </div>
                
                <div class="analysis-panel">
                    <div class="analysis-tabs">
                        <button class="tab-btn active" onclick="showAnalysisTab('overview')">📊 Обзор</button>
                        <button class="tab-btn" onclick="showAnalysisTab('types')">🔧 Типы</button>
                        <button class="tab-btn" onclick="showAnalysisTab('errors')">🚨 Ошибки</button>
                        <button class="tab-btn" onclick="showAnalysisTab('suggestions')">💡 Предложения</button>
                    </div>
                    
                    <div id="analysis-overview" class="analysis-tab-content active">
                        <div id="analysis-results" class="analysis-results-detailed">
                            <div class="analysis-placeholder">
                                <div class="placeholder-icon">⚡</div>
                                <h3>Готов к анализу</h3>
                                <p>Введите BSL код слева и нажмите "Анализировать код" для получения детального анализа типов, потенциальных ошибок и рекомендаций.</p>
                            </div>
                        </div>
                    </div>
                    
                    <div id="analysis-types" class="analysis-tab-content">
                        <div id="types-analysis">Детальный анализ типов появится здесь...</div>
                    </div>
                    
                    <div id="analysis-errors" class="analysis-tab-content">
                        <div id="errors-analysis">Ошибки и предупреждения появятся здесь...</div>
                    </div>
                    
                    <div id="analysis-suggestions" class="analysis-tab-content">
                        <div id="suggestions-analysis">Рекомендации по улучшению кода появятся здесь...</div>
                    </div>
                </div>
            </div>
        </div>"#.to_string();

    UnifiedPageTemplate::new("BSL Code Analyzer", "analyzer")
        .with_stats(stats)
        .with_content(content)
}

/// Создание шаблона для страницы API документации
pub fn create_api_template(stats: PageStatistics) -> UnifiedPageTemplate {
    let content = r#"
        <div class="api-layout">
            <div class="api-hero">
                <h2 class="section-title-large">🔗 API Документация</h2>
                <p class="section-subtitle">REST API для интеграции с внешними системами и разработки клиентских приложений</p>
            </div>
            
            <div class="api-documentation">
                <div class="api-section">
                    <h3 class="api-section-title">🔍 Поиск и типы</h3>
                    <div class="api-endpoints-grid">
                        <div class="api-endpoint-card">
                            <div class="endpoint-method get">GET</div>
                            <div class="endpoint-details">
                                <h4>/api/types</h4>
                                <p>Поиск типов с пагинацией</p>
                                <div class="endpoint-params">
                                    <span class="param">?search=<em>query</em></span>
                                    <span class="param">?page=<em>number</em></span>
                                    <span class="param">?per_page=<em>number</em></span>
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testEndpoint('/api/types?search=массив&per_page=5')">Тест</button>
                        </div>
                        
                        <div class="api-endpoint-card">
                            <div class="endpoint-method get">GET</div>
                            <div class="endpoint-details">
                                <h4>/api/types/{name}</h4>
                                <p>Детальная информация о типе</p>
                                <div class="endpoint-example">
                                    <code>/api/types/ТаблицаЗначений</code>
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testEndpoint('/api/types/ТаблицаЗначений')">Тест</button>
                        </div>
                    </div>
                </div>
                
                <div class="api-section">
                    <h3 class="api-section-title">🚀 Расширенный поиск (v1)</h3>
                    <div class="api-endpoints-grid">
                        <div class="api-endpoint-card">
                            <div class="endpoint-method post">POST</div>
                            <div class="endpoint-details">
                                <h4>/api/v1/search</h4>
                                <p>Расширенный поиск с фильтрами</p>
                                <div class="endpoint-json">
                                    <pre><code>{
  "query": "массив",
  "filters": {
    "category": "platform",
    "facet": "collection"
  },
  "pagination": {
    "page": 0,
    "page_size": 20
  }
}</code></pre>
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testAdvancedSearch()">Тест</button>
                        </div>
                        
                        <div class="api-endpoint-card">
                            <div class="endpoint-method get">GET</div>
                            <div class="endpoint-details">
                                <h4>/api/v1/suggestions</h4>
                                <p>Автодополнение для поиска</p>
                                <div class="endpoint-params">
                                    <span class="param">?partial_query=<em>text</em></span>
                                    <span class="param">?limit=<em>number</em></span>
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testEndpoint('/api/v1/suggestions?partial_query=табл&limit=10')">Тест</button>
                        </div>
                        
                        <div class="api-endpoint-card">
                            <div class="endpoint-method get">GET</div>
                            <div class="endpoint-details">
                                <h4>/api/v1/categories</h4>
                                <p>Список всех категорий типов</p>
                                <div class="endpoint-description">
                                    Возвращает структурированный список категорий и подкатегорий
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testEndpoint('/api/v1/categories')">Тест</button>
                        </div>
                        
                        <div class="api-endpoint-card">
                            <div class="endpoint-method get">GET</div>
                            <div class="endpoint-details">
                                <h4>/api/v1/search-stats</h4>
                                <p>Статистика поиска (JSON)</p>
                                <div class="endpoint-description">
                                    Метрики производительности поисковой системы
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testEndpoint('/api/v1/search-stats')">Тест</button>
                        </div>
                    </div>
                </div>
                
                <div class="api-section">
                    <h3 class="api-section-title">⚡ Анализ кода</h3>
                    <div class="api-endpoints-grid">
                        <div class="api-endpoint-card">
                            <div class="endpoint-method post">POST</div>
                            <div class="endpoint-details">
                                <h4>/api/analyze</h4>
                                <p>Анализ BSL кода</p>
                                <div class="endpoint-json">
                                    <pre><code>{
  "code": "Функция Тест()\n  Возврат \"Привет\";\nКонецФункции"
}</code></pre>
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testCodeAnalysis()">Тест</button>
                        </div>
                        
                        <div class="api-endpoint-card">
                            <div class="endpoint-method get">GET</div>
                            <div class="endpoint-details">
                                <h4>/api/status</h4>
                                <p>Статус загрузки данных</p>
                                <div class="endpoint-description">
                                    Прогресс парсинга и загрузки типов
                                </div>
                            </div>
                            <button class="test-endpoint-btn" onclick="testEndpoint('/api/status')">Тест</button>
                        </div>
                    </div>
                </div>
                
                <div class="api-test-area">
                    <h3>🧪 Тестирование API</h3>
                    <div class="test-interface">
                        <div class="test-request">
                            <label>Endpoint URL:</label>
                            <input type="text" id="test-url" class="test-input" placeholder="/api/types?search=массив">
                            <button class="btn" onclick="customApiTest()">Выполнить запрос</button>
                        </div>
                        <div class="test-response">
                            <h4>Ответ:</h4>
                            <pre id="api-response" class="response-container">Результат API запроса появится здесь...</pre>
                        </div>
                    </div>
                </div>
            </div>
        </div>"#.to_string();

    UnifiedPageTemplate::new("BSL API Documentation", "api")
        .with_stats(stats)
        .with_content(content)
}

/// Создание шаблона для страницы статистики
pub fn create_stats_template(stats: PageStatistics) -> UnifiedPageTemplate {
    let content = r#"
        <div class="stats-layout">
            <div class="stats-hero">
                <h2 class="section-title-large">📊 Статистика и аналитика</h2>
                <p class="section-subtitle">Детальные метрики производительности, статистика поиска и аналитика использования типов BSL</p>
            </div>
            
            <div class="stats-dashboard">
                <div class="stats-section">
                    <h3 class="stats-section-title">🏗️ Система типов</h3>
                    <div class="stats-cards-grid">
                        <div class="stats-detailed-card">
                            <div class="stats-card-header">
                                <span class="stats-icon">📚</span>
                                <h4>Платформенные типы</h4>
                            </div>
                            <div class="stats-card-value" id="platform-types-count">13,607</div>
                            <div class="stats-card-detail">Загружено из справки 1С</div>
                        </div>
                        
                        <div class="stats-detailed-card">
                            <div class="stats-card-header">
                                <span class="stats-icon">⚙️</span>
                                <h4>Конфигурационные типы</h4>
                            </div>
                            <div class="stats-card-value" id="config-types-count">0</div>
                            <div class="stats-card-detail">Из текущей конфигурации</div>
                        </div>
                        
                        <div class="stats-detailed-card">
                            <div class="stats-card-header">
                                <span class="stats-icon">🔍</span>
                                <h4>Индексированных документов</h4>
                            </div>
                            <div class="stats-card-value" id="indexed-docs-count">3,884</div>
                            <div class="stats-card-detail">Готовы к поиску</div>
                        </div>
                        
                        <div class="stats-detailed-card">
                            <div class="stats-card-header">
                                <span class="stats-icon">💾</span>
                                <h4>Использование памяти</h4>
                            </div>
                            <div class="stats-card-value" id="memory-usage">0.0 MB</div>
                            <div class="stats-card-detail">Кеш и индексы</div>
                        </div>
                    </div>
                </div>
                
                <div class="stats-section">
                    <h3 class="stats-section-title">🔍 Статистика поиска</h3>
                    <div class="search-stats-container">
                        <div class="search-stat-item">
                            <span class="search-stat-label">Всего запросов:</span>
                            <span class="search-stat-value" id="total-queries">0</span>
                        </div>
                        <div class="search-stat-item">
                            <span class="search-stat-label">Среднее время поиска:</span>
                            <span class="search-stat-value" id="avg-search-time">0.0 мс</span>
                        </div>
                        <div class="search-stat-item">
                            <span class="search-stat-label">Популярные запросы:</span>
                            <div id="popular-queries" class="popular-queries-list">
                                <span class="popular-query">Массив</span>
                                <span class="popular-query">Структура</span>
                                <span class="popular-query">ТаблицаЗначений</span>
                                <span class="popular-query">Строка</span>
                                <span class="popular-query">Справочник</span>
                            </div>
                        </div>
                    </div>
                </div>
                
                <div class="stats-section">
                    <h3 class="stats-section-title">⚡ Производительность</h3>
                    <div class="performance-metrics">
                        <div class="metric-row">
                            <span class="metric-label">Время инициализации:</span>
                            <span class="metric-value">~30 секунд</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">Время построения индексов:</span>
                            <span class="metric-value">~5 секунд</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">Время отклика API:</span>
                            <span class="metric-value">&lt;100 мс</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">Категорий в иерархии:</span>
                            <span class="metric-value">195</span>
                        </div>
                    </div>
                </div>
                
                <div class="stats-section">
                    <h3 class="stats-section-title">🎯 Использование системы</h3>
                    <div class="usage-charts">
                        <div class="chart-placeholder">
                            <div class="chart-icon">📈</div>
                            <h4>Графики использования</h4>
                            <p>Визуализация будет добавлена в следующих версиях</p>
                        </div>
                    </div>
                </div>
                
                <div class="stats-section">
                    <h3 class="stats-section-title">🔗 Дополнительная информация</h3>
                    <div class="info-links">
                        <a href="/api" class="info-link">
                            <span class="info-icon">🔗</span>
                            <div>
                                <h4>API Документация</h4>
                                <p>REST API endpoints и интеграция</p>
                            </div>
                        </a>
                        <a href="/hierarchy" class="info-link">
                            <span class="info-icon">🌳</span>
                            <div>
                                <h4>Иерархия типов</h4>
                                <p>Интерактивное дерево платформенных типов</p>
                            </div>
                        </a>
                    </div>
                </div>
            </div>
        </div>"#.to_string();

    UnifiedPageTemplate::new("BSL Statistics", "stats")
        .with_stats(stats)
        .with_content(content)
}

/// Создание шаблона для страницы иерархии
pub fn create_hierarchy_template(
    stats: PageStatistics,
    tree_content: String,
) -> UnifiedPageTemplate {
    let content = format!(
        r#"<div class="hierarchy-layout">
            <div class="hierarchy-sidebar">
                {}
            </div>
            <div class="hierarchy-content">
                <div id="type-details">
                    <div class="welcome-section">
                        <h2 class="welcome-title">🌳 BSL Type Browser v2.0 - Интерактивный режим</h2>
                        <div class="feature-highlights">
                            <div class="feature-item">
                                <strong>📂 Lazy Loading</strong><br>
                                Дочерние элементы загружаются по требованию
                            </div>
                            <div class="feature-item">
                                <strong>🔍 Поиск в дереве</strong><br>
                                Мгновенный поиск по всей иерархии
                            </div>
                            <div class="feature-item">
                                <strong>🎯 Drag & Drop</strong><br>
                                Перетаскивание для организации
                            </div>
                            <div class="feature-item">
                                <strong>📱 Контекстные меню</strong><br>
                                Правый клик для дополнительных опций
                            </div>
                        </div>
                        <p class="instruction">Выберите категорию или тип в дереве слева для просмотра детальной информации.</p>
                    </div>
                </div>
            </div>
        </div>"#,
        tree_content
    );

    UnifiedPageTemplate::new("BSL Type Hierarchy", "hierarchy")
        .with_stats(stats)
        .with_content(content)
}
