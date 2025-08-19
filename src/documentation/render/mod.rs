//! Система рендеринга документации в разные форматы

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use super::core::hierarchy::{TypeHierarchy, TypeDocumentationFull};
use super::search::SearchResults;

/// Движок рендеринга документации
pub struct RenderEngine {
    /// HTML рендерер для веб-интерфейса
    html_renderer: HtmlDocumentationRenderer,
    
    /// JSON рендерер для API
    json_renderer: JsonDocumentationRenderer,
    
    /// PDF рендерер для экспорта
    pdf_renderer: Option<PdfDocumentationRenderer>,
    
    /// Markdown рендерер
    markdown_renderer: MarkdownDocumentationRenderer,
    
    /// Система шаблонов
    template_engine: TemplateEngine,
}

/// HTML рендерер с полным функционалом
pub struct HtmlDocumentationRenderer {
    /// Активная тема
    current_theme: DocumentationTheme,
    
    /// Доступные темы
    available_themes: HashMap<String, DocumentationTheme>,
    
    /// Компоненты UI
    ui_components: UiComponentLibrary,
    
    /// Настройки рендеринга
    render_settings: HtmlRenderSettings,
}

/// Тема документации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationTheme {
    /// Название темы
    pub name: String,
    
    /// Цветовая схема
    pub color_scheme: ColorScheme,
    
    /// Иконки для разных типов
    pub type_icons: HashMap<String, String>,
    
    /// CSS стили
    pub css_styles: String,
    
    /// JavaScript код
    pub javascript_code: String,
    
    /// Шрифты
    pub fonts: FontConfig,
}

/// Цветовая схема
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// Основной цвет фона
    pub background_primary: String,
    
    /// Вторичный цвет фона
    pub background_secondary: String,
    
    /// Основной цвет текста
    pub text_primary: String,
    
    /// Вторичный цвет текста
    pub text_secondary: String,
    
    /// Цвет акцента
    pub accent_color: String,
    
    /// Цвета для разных типов
    pub type_colors: HashMap<String, String>,
}

/// Конфигурация шрифтов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Основной шрифт
    pub primary_font: String,
    
    /// Моноширинный шрифт для кода
    pub code_font: String,
    
    /// Размеры шрифтов
    pub font_sizes: HashMap<String, String>,
}

/// Настройки HTML рендеринга
#[derive(Debug, Clone)]
pub struct HtmlRenderSettings {
    /// Включить синтаксическую подсветку кода
    pub enable_syntax_highlighting: bool,
    
    /// Включить интерактивные примеры
    pub enable_interactive_examples: bool,
    
    /// Показывать навигацию по иерархии
    pub show_breadcrumbs: bool,
    
    /// Включить поиск в реальном времени
    pub enable_live_search: bool,
    
    /// Минифицировать выходной HTML
    pub minify_output: bool,
    
    /// Включить PWA функциональность
    pub enable_pwa: bool,
}

/// Библиотека UI компонентов
pub struct UiComponentLibrary {
    /// Компоненты для разных типов узлов
    components: HashMap<String, UiComponent>,
}

/// UI компонент
#[derive(Debug, Clone)]
pub struct UiComponent {
    /// Название компонента
    pub name: String,
    
    /// HTML шаблон
    pub template: String,
    
    /// CSS стили
    pub styles: String,
    
    /// JavaScript поведение
    pub behavior: String,
}

/// Интерактивное дерево с lazy loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveTree {
    /// ID дерева
    pub id: String,
    
    /// Корневые узлы
    pub root_nodes: Vec<InteractiveTreeNode>,
    
    /// Настройки дерева
    pub settings: TreeSettings,
    
    /// Состояние развёрнутых узлов
    pub expanded_nodes: std::collections::HashSet<String>,
    
    /// Выбранный узел
    pub selected_node: Option<String>,
}

/// Узел интерактивного дерева
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveTreeNode {
    /// Уникальный ID узла
    pub id: String,
    
    /// Отображаемое название
    pub display_name: String,
    
    /// Тип узла
    pub node_type: TreeNodeType,
    
    /// Иконка узла
    pub icon: String,
    
    /// Описание (tooltip)
    pub description: Option<String>,
    
    /// Дочерние узлы
    pub children: Vec<InteractiveTreeNode>,
    
    /// Может ли иметь дочерние узлы
    pub has_children: bool,
    
    /// Загружены ли дочерние узлы
    pub children_loaded: bool,
    
    /// URL для загрузки дочерних узлов
    pub children_url: Option<String>,
    
    /// Метаданные узла
    pub metadata: std::collections::HashMap<String, String>,
    
    /// Поддерживает ли drag & drop
    pub draggable: bool,
    
    /// Может ли быть drop target
    pub droppable: bool,
}

/// Тип узла дерева
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TreeNodeType {
    /// Категория
    Category,
    /// Подкатегория
    SubCategory,
    /// Платформенный тип
    PlatformType,
    /// Конфигурационный тип
    ConfigurationType,
    /// Метод
    Method,
    /// Свойство
    Property,
    /// Параметр метода
    Parameter,
    /// Закладка
    Bookmark,
    /// Избранное
    Favorite,
}

/// Настройки дерева
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSettings {
    /// Включить lazy loading
    pub lazy_loading: bool,
    
    /// Включить drag & drop
    pub drag_drop: bool,
    
    /// Включить контекстные меню
    pub context_menus: bool,
    
    /// Включить поиск в дереве
    pub tree_search: bool,
    
    /// Включить закладки
    pub bookmarks: bool,
    
    /// Включить избранное
    pub favorites: bool,
    
    /// Максимальная глубина загрузки
    pub max_depth: usize,
    
    /// Количество узлов на уровень
    pub nodes_per_level: usize,
}

/// JSON рендерер
pub struct JsonDocumentationRenderer {
    /// Настройки сериализации
    serialization_settings: JsonSerializationSettings,
}

/// Настройки JSON сериализации
#[derive(Debug, Clone)]
pub struct JsonSerializationSettings {
    /// Красивое форматирование
    pub pretty_print: bool,
    
    /// Включать null значения
    pub include_nulls: bool,
    
    /// Сжимать вывод
    pub compress_output: bool,
    
    /// Включать метаданные
    pub include_metadata: bool,
}

/// PDF рендерер (опциональный)
pub struct PdfDocumentationRenderer {
    /// Настройки PDF
    pdf_settings: PdfSettings,
}

/// Настройки PDF генерации
#[derive(Debug, Clone)]
pub struct PdfSettings {
    /// Размер страницы
    pub page_size: PageSize,
    
    /// Ориентация
    pub orientation: PageOrientation,
    
    /// Поля страницы
    pub margins: PageMargins,
    
    /// Включать оглавление
    pub include_toc: bool,
    
    /// Включать индекс
    pub include_index: bool,
}

/// Размер страницы
#[derive(Debug, Clone)]
pub enum PageSize {
    A4,
    Letter,
    Legal,
    Custom { width: f32, height: f32 },
}

/// Ориентация страницы
#[derive(Debug, Clone)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

/// Поля страницы
#[derive(Debug, Clone)]
pub struct PageMargins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Markdown рендерер
pub struct MarkdownDocumentationRenderer {
    /// Настройки Markdown
    markdown_settings: MarkdownSettings,
}

/// Настройки Markdown
#[derive(Debug, Clone)]
pub struct MarkdownSettings {
    /// Включать оглавление
    pub include_toc: bool,
    
    /// Включать ссылки
    pub include_links: bool,
    
    /// Формат кода
    pub code_format: CodeFormat,
}

/// Формат кода в Markdown
#[derive(Debug, Clone)]
pub enum CodeFormat {
    /// Блоки кода с подсветкой
    FencedCodeBlocks,
    
    /// Обычные блоки кода
    IndentedCodeBlocks,
    
    /// Inline код
    InlineCode,
}

/// Система шаблонов
pub struct TemplateEngine {
    /// Загруженные шаблоны
    templates: HashMap<String, Template>,
    
    /// Настройки шаблонизатора
    settings: TemplateSettings,
}

/// Шаблон
#[derive(Debug, Clone)]
pub struct Template {
    /// Название шаблона
    pub name: String,
    
    /// Содержимое шаблона
    pub content: String,
    
    /// Зависимые шаблоны
    pub dependencies: Vec<String>,
}

/// Настройки шаблонизатора
#[derive(Debug, Clone)]
pub struct TemplateSettings {
    /// Кеширование шаблонов
    pub cache_templates: bool,
    
    /// Автоматическое обновление
    pub auto_reload: bool,
    
    /// Строгий режим
    pub strict_mode: bool,
}

impl RenderEngine {
    /// Создать новый движок рендеринга
    pub fn new() -> Self {
        Self {
            html_renderer: HtmlDocumentationRenderer::new(),
            json_renderer: JsonDocumentationRenderer::new(),
            pdf_renderer: None, // Создается по требованию
            markdown_renderer: MarkdownDocumentationRenderer::new(),
            template_engine: TemplateEngine::new(),
        }
    }
    
    /// Получить HTML рендерер
    pub fn html_renderer(&self) -> &HtmlDocumentationRenderer {
        &self.html_renderer
    }
    
    /// Рендеринг иерархии в HTML
    pub async fn render_hierarchy_html(&self, hierarchy: &TypeHierarchy) -> Result<String> {
        self.html_renderer.render_hierarchy(hierarchy).await
    }
    
    /// Рендеринг результатов поиска в HTML
    pub async fn render_search_results_html(&self, results: &SearchResults) -> Result<String> {
        self.html_renderer.render_search_results(results).await
    }
    
    /// Рендеринг типа в JSON
    pub async fn render_type_json(&self, type_doc: &TypeDocumentationFull) -> Result<String> {
        self.json_renderer.render_type(type_doc).await
    }
    
    /// Получить доступные темы
    pub fn get_available_themes(&self) -> Vec<String> {
        self.html_renderer.available_themes.keys().cloned().collect()
    }
    
    /// Установить тему
    pub async fn set_theme(&mut self, theme_name: &str) -> Result<()> {
        self.html_renderer.set_theme(theme_name).await
    }
}

impl HtmlDocumentationRenderer {
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        
        // Добавляем встроенные темы
        themes.insert("dark".to_string(), Self::create_dark_theme());
        themes.insert("light".to_string(), Self::create_light_theme());
        themes.insert("vscode".to_string(), Self::create_vscode_theme());
        
        Self {
            current_theme: Self::create_dark_theme(),
            available_themes: themes,
            ui_components: UiComponentLibrary::new(),
            render_settings: HtmlRenderSettings::default(),
        }
    }
    
    /// Создать интерактивное дерево из иерархии типов
    pub fn create_interactive_tree(&self, hierarchy: &TypeHierarchy) -> InteractiveTree {
        let mut tree = InteractiveTree {
            id: "main_tree".to_string(),
            root_nodes: Vec::new(),
            settings: TreeSettings::default(),
            expanded_nodes: std::collections::HashSet::new(),
            selected_node: None,
        };
        
        // Преобразуем категории в узлы дерева
        for category in &hierarchy.root_categories {
            let node = self.convert_category_to_tree_node(category);
            tree.root_nodes.push(node);
        }
        
        tree
    }
    
    /// Преобразовать категорию в узел дерева
    fn convert_category_to_tree_node(&self, category: &super::core::hierarchy::CategoryNode) -> InteractiveTreeNode {
        let node_id = format!("category_{}", category.name.replace(" ", "_"));
        
        InteractiveTreeNode {
            id: node_id.clone(),
            display_name: category.name.clone(),
            node_type: TreeNodeType::Category,
            icon: "📁".to_string(),
            description: Some(format!("Категория типов: {} элементов", category.children.len())),
            children: Vec::new(), // Будут загружены по запросу
            has_children: !category.children.is_empty(),
            children_loaded: false,
            children_url: Some(format!("/api/tree/children/{}", node_id)),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("type".to_string(), "category".to_string());
                meta.insert("count".to_string(), category.children.len().to_string());
                meta
            },
            draggable: false,
            droppable: true,
        }
    }
    
    /// Рендеринг интерактивного дерева в HTML
    pub fn render_interactive_tree(&self, tree: &InteractiveTree) -> Result<String> {
        let mut html = String::new();
        
        // Контейнер дерева
        html.push_str(&format!(
            "<div class='interactive-tree' id='{}'>\n\
             <div class='tree-header'>\n\
             <div class='tree-controls'>\n\
             <button class='btn-expand-all' onclick='expandAllNodes()'>📂 Развернуть все</button>\n\
             <button class='btn-collapse-all' onclick='collapseAllNodes()'>📁 Свернуть все</button>\n\
             </div>\n\
             <div class='tree-search'>\n\
             <input type='text' id='tree-search-input' placeholder='Поиск в дереве...' onkeyup='searchInTree(this.value)'>\n\
             <button class='clear-search' onclick='clearTreeSearch()'>❌</button>\n\
             </div>\n\
             </div>\n",
            tree.id
        ));
        
        // Корень дерева
        html.push_str("<div class='tree-root' data-tree-root='true'>\n");
        
        // Рендерим корневые узлы
        for node in &tree.root_nodes {
            html.push_str(&self.render_tree_node(node, 0, tree)?);
        }
        
        html.push_str("</div>\n"); // tree-root
        html.push_str("</div>\n"); // interactive-tree
        
        Ok(html)
    }
    
    /// Рендеринг отдельного узла дерева
    fn render_tree_node(&self, node: &InteractiveTreeNode, depth: usize, tree: &InteractiveTree) -> Result<String> {
        let mut html = String::new();
        let indent = depth * 20; // px
        let is_expanded = tree.expanded_nodes.contains(&node.id);
        let is_selected = tree.selected_node.as_ref() == Some(&node.id);
        
        // Основной элемент узла
        html.push_str(&format!(
            "<div class='tree-node {}{}{}' \
             id='node_{}' \
             data-node-id='{}' \
             data-node-type='{}' \
             data-has-children='{}' \
             data-children-loaded='{}' \
             style='padding-left: {}px;' \
             {} \
             {} \
             onclick='handleNodeClick(event, \"{}\")' \
             oncontextmenu='showNodeContextMenu(event, \"{}\")'>\n",
            match node.node_type {
                TreeNodeType::Category => "category-node",
                TreeNodeType::SubCategory => "subcategory-node", 
                TreeNodeType::PlatformType => "platform-type-node",
                TreeNodeType::ConfigurationType => "config-type-node",
                TreeNodeType::Method => "method-node",
                TreeNodeType::Property => "property-node",
                TreeNodeType::Parameter => "parameter-node",
                TreeNodeType::Bookmark => "bookmark-node",
                TreeNodeType::Favorite => "favorite-node",
            },
            if is_expanded { " expanded" } else { "" },
            if is_selected { " selected" } else { "" },
            node.id, node.id,
            serde_json::to_string(&node.node_type).unwrap_or_default().trim_matches('"'),
            node.has_children, node.children_loaded, indent,
            if node.draggable { "draggable='true' ondragstart='handleDragStart(event)'" } else { "" },
            if node.droppable { "ondragover='handleDragOver(event)' ondrop='handleDrop(event)'" } else { "" },
            node.id, node.id
        ));
        
        // Expand/collapse индикатор
        if node.has_children {
            html.push_str(&format!(
                "<span class='expand-indicator {}' onclick='toggleNodeExpansion(event, \"{}\")' data-expanded='{}'>{}</span>\n",
                if is_expanded { "expanded" } else { "collapsed" },
                node.id, is_expanded,
                if is_expanded { "▼" } else { "▶" }
            ));
        } else {
            html.push_str("<span class='expand-placeholder'></span>\n");
        }
        
        // Иконка и название
        html.push_str(&format!(
            "<span class='node-icon'>{}</span>\n\
             <span class='node-title' title='{}'>{}</span>\n",
            node.icon,
            node.description.as_deref().unwrap_or(""),
            node.display_name
        ));
        
        // Метаданные (например, количество дочерних элементов)
        if let Some(count) = node.metadata.get("count") {
            html.push_str(&format!(
                "<span class='node-meta'>({} эл.)</span>\n",
                count
            ));
        }
        
        html.push_str("</div>\n"); // tree-node
        
        // Дочерние узлы (если загружены и развернуты)
        if node.children_loaded && is_expanded && !node.children.is_empty() {
            html.push_str("<div class='tree-children' data-parent-id='{}'>\n");
            
            for child in &node.children {
                html.push_str(&self.render_tree_node(child, depth + 1, tree)?);
            }
            
            html.push_str("</div>\n");
        } else if node.has_children && is_expanded {
            // Placeholder для lazy loading
            html.push_str(&format!(
                "<div class='tree-children loading' data-parent-id='{}'>\n\
                 <div class='loading-placeholder'>Загрузка...</div>\n\
                 </div>\n",
                node.id
            ));
        }
        
        Ok(html)
    }
    
    /// Рендеринг полной иерархии типов в HTML
    pub async fn render_hierarchy(&self, hierarchy: &TypeHierarchy) -> Result<String> {
        let mut html = String::new();
        
        // Начинаем с основного контейнера
        html.push_str(&self.render_page_header("BSL Type Hierarchy"));
        html.push_str("<div class='hierarchy-container'>\n");
        
        // Боковая панель с интерактивным деревом
        html.push_str("<div class='sidebar'>\n");
        html.push_str("<div class='tree-container'>\n");
        
        // Создаем и рендерим интерактивное дерево
        let interactive_tree = self.create_interactive_tree(hierarchy);
        html.push_str(&self.render_interactive_tree(&interactive_tree)?);
        
        html.push_str("</div>\n</div>\n");
        
        // Основная область с деталями
        html.push_str("<div class='main-content'>\n");
        html.push_str("<div id='type-details'>\n");
        html.push_str("<div class='welcome-message'>\n");
        html.push_str("<h2>🚀 BSL Type Browser v2.0 - Интерактивный режим</h2>\n");
        html.push_str("<div class='feature-highlights'>\n");
        html.push_str("<div class='feature-item'>📂 <strong>Lazy Loading</strong> - дочерние элементы загружаются по требованию</div>\n");
        html.push_str("<div class='feature-item'>🔍 <strong>Поиск в дереве</strong> - мгновенный поиск по всей иерархии</div>\n");
        html.push_str("<div class='feature-item'>🎯 <strong>Drag & Drop</strong> - перетаскивание для организации</div>\n");
        html.push_str("<div class='feature-item'>📱 <strong>Контекстные меню</strong> - правый клик для опций</div>\n");
        html.push_str("<div class='feature-item'>⭐ <strong>Закладки</strong> - сохранение избранных типов</div>\n");
        html.push_str("</div>\n");
        html.push_str("<p class='instruction'>Выберите категорию или тип в дереве слева для просмотра детальной информации.</p>\n");
        html.push_str("</div>\n</div>\n</div>\n");
        
        html.push_str("</div>\n"); // hierarchy-container
        html.push_str(&self.render_page_footer());
        
        Ok(html)
    }
    
    /// Рендеринг результатов поиска в HTML
    pub async fn render_search_results(&self, results: &SearchResults) -> Result<String> {
        let mut html = String::new();
        
        // Заголовок с результатами
        html.push_str(&format!(
            "<div class='search-results-header'>\n\
             <h2>Результаты поиска</h2>\n\
             <div class='search-meta'>\n\
             <span class='results-count'>Найдено: {}</span>\n\
             <span class='search-time'>Время: {}ms</span>\n\
             </div>\n</div>\n",
            results.total_count, results.search_time_ms
        ));
        
        // Фасеты (фильтры)
        if !results.facets.is_empty() {
            html.push_str("<div class='facets-panel'>\n");
            html.push_str("<h3>Фильтры</h3>\n");
            
            for facet in &results.facets {
                html.push_str(&format!("<div class='facet-group'>\n"));
                html.push_str(&format!("<h4>{}</h4>\n", facet.name));
                
                for value in &facet.values {
                    let selected = if value.selected { "selected" } else { "" };
                    let checked = if value.selected { "checked" } else { "" };
                    html.push_str(&format!(
                        "<label class='facet-item {}'>\n\
                         <input type='checkbox' value='{}' {}>\n\
                         <span>{} ({})</span>\n\
                         </label>\n",
                        selected, value.value, checked, value.value, value.count
                    ));
                }
                html.push_str("</div>\n");
            }
            html.push_str("</div>\n");
        }
        
        // Результаты поиска
        html.push_str("<div class='search-results-list'>\n");
        
        for item in &results.items {
            html.push_str(&self.render_search_result_item(item).await?);
        }
        
        html.push_str("</div>\n");
        
        // Пагинация
        html.push_str(&self.render_pagination(&results.pagination_info));
        
        Ok(html)
    }
    
    /// Рендеринг отдельного результата поиска
    async fn render_search_result_item(&self, item: &super::search::SearchResultItem) -> Result<String> {
        let mut html = String::new();
        
        html.push_str("<div class='search-result-item'>\n");
        
        // Заголовок с названием типа
        html.push_str(&format!(
            "<div class='result-header'>\n\
             <h3 class='type-name'>{}</h3>\n\
             <span class='type-category'>{}</span>\n\
             <span class='relevance-score'>Score: {:.2}</span>\n\
             </div>\n",
            item.display_name, item.category, item.relevance_score
        ));
        
        // Описание
        html.push_str(&format!(
            "<div class='result-description'>{}</div>\n",
            item.description.chars().take(200).collect::<String>()
        ));
        
        // Хлебные крошки
        if !item.breadcrumb.is_empty() {
            html.push_str("<div class='breadcrumb'>\n");
            html.push_str(&item.breadcrumb.join(" → "));
            html.push_str("</div>\n");
        }
        
        // Подсветка совпадений
        if !item.highlights.is_empty() {
            html.push_str("<div class='highlights'>\n");
            for highlight in &item.highlights {
                html.push_str(&format!(
                    "<div class='highlight-fragment'>{}</div>\n",
                    highlight.highlighted_text
                ));
            }
            html.push_str("</div>\n");
        }
        
        html.push_str("</div>\n");
        
        Ok(html)
    }
    
    pub async fn set_theme(&mut self, theme_name: &str) -> Result<()> {
        if let Some(theme) = self.available_themes.get(theme_name) {
            self.current_theme = theme.clone();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Theme '{}' not found", theme_name))
        }
    }
    
    // Встроенные темы
    fn create_dark_theme() -> DocumentationTheme {
        DocumentationTheme {
            name: "Dark".to_string(),
            color_scheme: ColorScheme {
                background_primary: "#1e1e1e".to_string(),
                background_secondary: "#2d2d30".to_string(),
                text_primary: "#d4d4d4".to_string(),
                text_secondary: "#9cdcfe".to_string(),
                accent_color: "#569cd6".to_string(),
                type_colors: HashMap::new(),
            },
            type_icons: HashMap::new(),
            css_styles: String::new(),
            javascript_code: String::new(),
            fonts: FontConfig::default(),
        }
    }
    
    fn create_light_theme() -> DocumentationTheme {
        DocumentationTheme {
            name: "Light".to_string(),
            color_scheme: ColorScheme {
                background_primary: "#ffffff".to_string(),
                background_secondary: "#f8f8f8".to_string(),
                text_primary: "#333333".to_string(),
                text_secondary: "#666666".to_string(),
                accent_color: "#0066cc".to_string(),
                type_colors: HashMap::new(),
            },
            type_icons: HashMap::new(),
            css_styles: String::new(),
            javascript_code: String::new(),
            fonts: FontConfig::default(),
        }
    }
    
    fn create_vscode_theme() -> DocumentationTheme {
        DocumentationTheme {
            name: "VSCode".to_string(),
            color_scheme: ColorScheme {
                background_primary: "#1e1e1e".to_string(),
                background_secondary: "#252526".to_string(),
                text_primary: "#cccccc".to_string(),
                text_secondary: "#9cdcfe".to_string(),
                accent_color: "#007acc".to_string(),
                type_colors: HashMap::new(),
            },
            type_icons: HashMap::new(),
            css_styles: String::new(),
            javascript_code: String::new(),
            fonts: FontConfig::default(),
        }
    }
    
    // === ВСПОМОГАТЕЛЬНЫЕ МЕТОДЫ РЕНДЕРИНГА ===
    
    /// Рендеринг заголовка страницы
    fn render_page_header(&self, title: &str) -> String {
        format!(
            "<!DOCTYPE html>\n\
             <html lang='ru'>\n\
             <head>\n\
             <meta charset='UTF-8'>\n\
             <meta name='viewport' content='width=device-width, initial-scale=1.0'>\n\
             <title>{}</title>\n\
             {}\n\
             </head>\n\
             <body class='theme-{}'>\n\
             <header class='page-header'>\n\
             <h1>{}</h1>\n\
             <div class='theme-switcher'>\n\
             <button onclick='switchTheme(\"dark\")'>🌙 Темная</button>\n\
             <button onclick='switchTheme(\"light\")'>☀️ Светлая</button>\n\
             <button onclick='switchTheme(\"vscode\")'>💻 VSCode</button>\n\
             </div>\n\
             </header>\n",
            title, 
            self.render_css(),
            self.current_theme.name.to_lowercase(),
            title
        )
    }
    
    /// Рендеринг подвала страницы
    fn render_page_footer(&self) -> String {
        format!(
            "<footer class='page-footer'>\n\
             <p>BSL Gradual Type System v1.0.0 | Enterprise Documentation</p>\n\
             </footer>\n\
             {}\n\
             </body>\n\
             </html>",
            self.render_javascript()
        )
    }
    
    /// Рендеринг CSS стилей
    pub fn render_css(&self) -> String {
        let theme = &self.current_theme;
        format!(
            "<style>\n\
             /* === БАЗОВЫЕ СТИЛИ === */\n\
             * {{ margin: 0; padding: 0; box-sizing: border-box; }}\n\
             \n\
             body {{\n\
               font-family: {};\n\
               background: {};\n\
               color: {};\n\
               line-height: 1.6;\n\
               font-size: 14px;\n\
             }}\n\
             \n\
             /* === LAYOUT === */\n\
             .page-header {{\n\
               background: {};\n\
               border-bottom: 1px solid #3c3c3c;\n\
               padding: 1rem 2rem;\n\
               display: flex;\n\
               justify-content: space-between;\n\
               align-items: center;\n\
             }}\n\
             \n\
             .hierarchy-container {{\n\
               display: flex;\n\
               height: calc(100vh - 140px);\n\
             }}\n\
             \n\
             .sidebar {{\n\
               width: 350px;\n\
               background: {};\n\
               border-right: 1px solid #3c3c3c;\n\
               padding: 1rem;\n\
               overflow-y: auto;\n\
             }}\n\
             \n\
             .main-content {{\n\
               flex: 1;\n\
               padding: 2rem;\n\
               overflow-y: auto;\n\
             }}\n\
             \n\
             /* === ДЕРЕВО ТИПОВ === */\n\
             .tree-node {{\n\
               margin: 0.25rem 0;\n\
               cursor: pointer;\n\
               padding: 0.5rem;\n\
               border-radius: 4px;\n\
               transition: background 0.2s;\n\
             }}\n\
             \n\
             .tree-node:hover {{\n\
               background: rgba(255, 255, 255, 0.1);\n\
             }}\n\
             \n\
             .tree-node.selected {{\n\
               background: {};\n\
               color: white;\n\
             }}\n\
             \n\
             .tree-children {{\n\
               margin-left: 1.5rem;\n\
               border-left: 1px solid #3c3c3c;\n\
               padding-left: 0.5rem;\n\
             }}\n\
             \n\
             /* === РЕЗУЛЬТАТЫ ПОИСКА === */\n\
             .search-result-item {{\n\
               background: {};\n\
               border: 1px solid #3c3c3c;\n\
               border-radius: 8px;\n\
               padding: 1.5rem;\n\
               margin-bottom: 1rem;\n\
               transition: transform 0.2s, box-shadow 0.2s;\n\
             }}\n\
             \n\
             .search-result-item:hover {{\n\
               transform: translateY(-2px);\n\
               box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);\n\
             }}\n\
             \n\
             .result-header {{\n\
               display: flex;\n\
               justify-content: space-between;\n\
               align-items: flex-start;\n\
               margin-bottom: 0.75rem;\n\
             }}\n\
             \n\
             .type-name {{\n\
               color: #4ec9b0;\n\
               font-size: 1.3em;\n\
               font-weight: 600;\n\
               margin: 0;\n\
             }}\n\
             \n\
             .type-category {{\n\
               color: {};\n\
               font-size: 0.9em;\n\
               background: rgba(156, 220, 254, 0.1);\n\
               padding: 0.25rem 0.5rem;\n\
               border-radius: 4px;\n\
             }}\n\
             \n\
             .relevance-score {{\n\
               color: #ffcc99;\n\
               font-size: 0.8em;\n\
               font-weight: 500;\n\
             }}\n\
             \n\
             .result-description {{\n\
               color: {};\n\
               margin-bottom: 0.75rem;\n\
               line-height: 1.5;\n\
             }}\n\
             \n\
             .breadcrumb {{\n\
               color: #9cdcfe;\n\
               font-size: 0.85em;\n\
               margin-bottom: 0.5rem;\n\
             }}\n\
             \n\
             mark {{\n\
               background: {};\n\
               color: black;\n\
               padding: 0.1rem 0.2rem;\n\
               border-radius: 2px;\n\
               font-weight: 600;\n\
             }}\n\
             \n\
             /* === ПЕРЕКЛЮЧАТЕЛЬ ТЕМ === */\n\
             .theme-switcher button {{\n\
               background: {};\n\
               color: {};\n\
               border: 1px solid #3c3c3c;\n\
               padding: 0.5rem 1rem;\n\
               margin-left: 0.5rem;\n\
               border-radius: 4px;\n\
               cursor: pointer;\n\
               transition: background 0.2s;\n\
             }}\n\
             \n\
             .theme-switcher button:hover {{\n\
               background: {};\n\
             }}\n\
             \n\
             /* === RESPONSIVE === */\n\
             @media (max-width: 768px) {{\n\
               .hierarchy-container {{ flex-direction: column; }}\n\
               .sidebar {{ width: 100%; height: 200px; }}\n\
               .page-header {{ flex-direction: column; gap: 1rem; }}\n\
             }}\n\
             </style>",
            theme.fonts.primary_font,
            theme.color_scheme.background_primary,
            theme.color_scheme.text_primary,
            theme.color_scheme.background_secondary,
            theme.color_scheme.background_secondary,
            theme.color_scheme.accent_color,
            theme.color_scheme.background_secondary,
            theme.color_scheme.text_secondary,
            theme.color_scheme.text_primary,
            "#ffeb3b", // Желтая подсветка для mark
            theme.color_scheme.background_secondary,
            theme.color_scheme.text_primary,
            theme.color_scheme.accent_color
        )
    }
    
    /// Рендеринг JavaScript кода
    pub fn render_javascript(&self) -> String {
        r#"
<script>
// === ПЕРЕКЛЮЧЕНИЕ ТЕМ ===
function switchTheme(themeName) {
    document.body.className = 'theme-' + themeName;
    localStorage.setItem('bsl-docs-theme', themeName);
    console.log('Switched to theme:', themeName);
}

// === ИНТЕРАКТИВНОЕ ДЕРЕВО ===

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
    event.stopPropagation();
    
    const node = document.getElementById(`node_${nodeId}`);
    if (!node) return;
    
    const hasChildren = node.dataset.hasChildren === 'true';
    const childrenLoaded = node.dataset.childrenLoaded === 'true';
    
    if (!hasChildren) return;
    
    const isExpanded = treeState.expandedNodes.has(nodeId);
    const indicator = node.querySelector('.expand-indicator');
    
    if (isExpanded) {
        // Сворачиваем
        treeState.expandedNodes.delete(nodeId);
        const children = document.querySelector(`.tree-children[data-parent-id="${nodeId}"]`);
        if (children) {
            children.style.display = 'none';
        }
        indicator.textContent = '▶';
        indicator.dataset.expanded = 'false';
        node.classList.remove('expanded');
    } else {
        // Раскрываем
        treeState.expandedNodes.add(nodeId);
        
        if (!childrenLoaded) {
            // Lazy loading - загружаем дочерние узлы
            loadChildrenNodes(nodeId);
        } else {
            // Показываем уже загруженные узлы
            const children = document.querySelector(`.tree-children[data-parent-id="${nodeId}"]`);
            if (children) {
                children.style.display = 'block';
            }
        }
        
        indicator.textContent = '▼';
        indicator.dataset.expanded = 'true';
        node.classList.add('expanded');
    }
}

// === LAZY LOADING ДОЧЕРНИХ УЗЛОВ ===
async function loadChildrenNodes(nodeId) {
    const childrenContainer = document.querySelector(`.tree-children[data-parent-id="${nodeId}"]`);
    if (!childrenContainer) return;
    
    childrenContainer.classList.add('loading');
    childrenContainer.innerHTML = '<div class="loading-placeholder">⏳ Загрузка дочерних элементов...</div>';
    
    try {
        const response = await fetch(`/api/tree/children/${nodeId}`);
        const children = await response.json();
        
        // Рендерим дочерние узлы
        childrenContainer.innerHTML = renderChildrenNodes(children, nodeId);
        childrenContainer.classList.remove('loading');
        childrenContainer.style.display = 'block';
        
        // Помечаем как загруженные
        const parentNode = document.getElementById(`node_${nodeId}`);
        if (parentNode) {
            parentNode.dataset.childrenLoaded = 'true';
        }
        
    } catch (error) {
        console.error('Error loading children:', error);
        childrenContainer.innerHTML = '<div class="loading-placeholder">❌ Ошибка загрузки</div>';
        childrenContainer.classList.remove('loading');
    }
}

// === РЕНДЕРИНГ ДОЧЕРНИХ УЗЛОВ ===
function renderChildrenNodes(children, parentId) {
    return children.map(child => `
        <div class='tree-node ${child.node_type}-node' 
             id='node_${child.id}' 
             data-node-id='${child.id}' 
             data-node-type='${child.node_type}' 
             data-has-children='${child.has_children}' 
             data-children-loaded='${child.children_loaded}'
             ${child.draggable ? "draggable='true' ondragstart='handleDragStart(event)'" : ""}
             ${child.droppable ? "ondragover='handleDragOver(event)' ondrop='handleDrop(event)'" : ""}
             onclick='handleNodeClick(event, "${child.id}")' 
             oncontextmenu='showNodeContextMenu(event, "${child.id}")'>
            
            ${child.has_children ? 
                `<span class='expand-indicator collapsed' onclick='toggleNodeExpansion(event, "${child.id}")' data-expanded='false'>▶</span>` :
                `<span class='expand-placeholder'></span>`
            }
            
            <span class='node-icon'>${child.icon}</span>
            <span class='node-title' title='${child.description || ""}'>${child.display_name}</span>
            
            ${child.metadata && child.metadata.count ? 
                `<span class='node-meta'>(${child.metadata.count} эл.)</span>` : ''
            }
        </div>
        
        ${child.has_children ? 
            `<div class='tree-children' data-parent-id='${child.id}' style='display: none;'></div>` : ''
        }
    `).join('');
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
    document.getElementById('tree-search-input').value = '';
    searchInTree('');
}

// === РАЗВЕРНУТЬ ВСЕ УЗЛЫ ===
function expandAllNodes() {
    document.querySelectorAll('.tree-node[data-has-children="true"]').forEach(node => {
        const nodeId = node.dataset.nodeId;
        if (!treeState.expandedNodes.has(nodeId)) {
            toggleNodeExpansion({ stopPropagation: () => {} }, nodeId);
        }
    });
}

// === СВЕРНУТЬ ВСЕ УЗЛЫ ===
function collapseAllNodes() {
    document.querySelectorAll('.tree-node.expanded').forEach(node => {
        const nodeId = node.dataset.nodeId;
        toggleNodeExpansion({ stopPropagation: () => {} }, nodeId);
    });
}

// === DRAG & DROP ===
function handleDragStart(event) {
    treeState.draggedNode = event.target;
    event.target.classList.add('dragging');
    event.dataTransfer.effectAllowed = 'move';
}

function handleDragOver(event) {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    event.target.closest('.tree-node').classList.add('drag-over');
}

function handleDrop(event) {
    event.preventDefault();
    
    const dropTarget = event.target.closest('.tree-node');
    dropTarget.classList.remove('drag-over');
    
    if (treeState.draggedNode && dropTarget !== treeState.draggedNode) {
        // Логика перемещения узла
        console.log('Moving node:', treeState.draggedNode.dataset.nodeId, 'to:', dropTarget.dataset.nodeId);
        // TODO: Реализовать перемещение узла
    }
    
    if (treeState.draggedNode) {
        treeState.draggedNode.classList.remove('dragging');
        treeState.draggedNode = null;
    }
}

// === КОНТЕКСТНОЕ МЕНЮ ===
function showNodeContextMenu(event, nodeId) {
    event.preventDefault();
    
    // Удаляем старое меню
    const existingMenu = document.querySelector('.context-menu');
    if (existingMenu) {
        existingMenu.remove();
    }
    
    const node = document.getElementById(`node_${nodeId}`);
    const nodeType = node.dataset.nodeType;
    
    // Создаем меню
    const menu = createContextMenu(nodeId, nodeType);
    document.body.appendChild(menu);
    
    // Позиционируем меню
    menu.style.left = event.pageX + 'px';
    menu.style.top = event.pageY + 'px';
    menu.classList.add('show');
    
    // Закрытие меню по клику вне его
    setTimeout(() => {
        document.addEventListener('click', function closeMenu() {
            menu.remove();
            document.removeEventListener('click', closeMenu);
        });
    }, 100);
}

function createContextMenu(nodeId, nodeType) {
    const menu = document.createElement('div');
    menu.className = 'context-menu';
    
    const menuItems = [
        { text: '📂 Развернуть всё', action: () => expandAllChildrenOf(nodeId) },
        { text: '📁 Свернуть всё', action: () => collapseAllChildrenOf(nodeId) },
        { separator: true },
        { text: '⭐ Добавить в избранное', action: () => addToFavorites(nodeId) },
        { text: '🔖 Добавить закладку', action: () => addBookmark(nodeId) },
        { separator: true },
        { text: '📋 Копировать путь', action: () => copyNodePath(nodeId) },
        { text: '🔗 Копировать ссылку', action: () => copyNodeLink(nodeId) }
    ];
    
    menuItems.forEach(item => {
        if (item.separator) {
            const separator = document.createElement('div');
            separator.className = 'context-menu-separator';
            menu.appendChild(separator);
        } else {
            const menuItem = document.createElement('div');
            menuItem.className = 'context-menu-item';
            menuItem.textContent = item.text;
            menuItem.onclick = item.action;
            menu.appendChild(menuItem);
        }
    });
    
    return menu;
}

// === ЗАГРУЗКА ДЕТАЛЕЙ УЗЛА ===
async function loadNodeDetails(nodeId, nodeType) {
    const detailsContainer = document.getElementById('type-details');
    detailsContainer.innerHTML = '<div class="loading">🔄 Загрузка деталей...</div>';
    
    try {
        const response = await fetch(`/api/node/${nodeId}/details`);
        const details = await response.json();
        
        detailsContainer.innerHTML = renderNodeDetails(details);
    } catch (error) {
        console.error('Error loading details:', error);
        detailsContainer.innerHTML = `<div class="error">❌ Ошибка загрузки: ${error.message}</div>`;
    }
}

// === РЕНДЕРИНГ ДЕТАЛЕЙ УЗЛА ===
function renderNodeDetails(details) {
    return `
        <div class="node-details">
            <div class="details-header">
                <h2>${details.icon || '📄'} ${details.display_name}</h2>
                <div class="node-type-badge ${details.node_type}">${details.node_type}</div>
            </div>
            
            ${details.description ? `<div class="description">${details.description}</div>` : ''}
            
            <div class="details-tabs">
                <button class="tab-btn active" onclick="showTab('overview')">Обзор</button>
                <button class="tab-btn" onclick="showTab('methods')">Методы</button>
                <button class="tab-btn" onclick="showTab('properties')">Свойства</button>
                <button class="tab-btn" onclick="showTab('examples')">Примеры</button>
            </div>
            
            <div id="tab-overview" class="tab-content active">
                <h3>Общая информация</h3>
                <table class="details-table">
                    <tr><td>Тип:</td><td>${details.node_type}</td></tr>
                    <tr><td>ID:</td><td>${details.id}</td></tr>
                    ${details.metadata ? Object.entries(details.metadata).map(([key, value]) => 
                        `<tr><td>${key}:</td><td>${value}</td></tr>`
                    ).join('') : ''}
                </table>
            </div>
            
            <div id="tab-methods" class="tab-content">
                <h3>Методы (${details.methods?.length || 0})</h3>
                ${details.methods?.map(method => `
                    <div class="method-item">
                        <h4>🔧 ${method.name}</h4>
                        ${method.description ? `<p>${method.description}</p>` : ''}
                        ${method.parameters ? `
                            <div class="parameters">
                                <strong>Параметры:</strong>
                                <ul>
                                    ${method.parameters.map(param => `
                                        <li><code>${param.name}</code> (${param.type}) - ${param.description || 'Без описания'}</li>
                                    `).join('')}
                                </ul>
                            </div>
                        ` : ''}
                    </div>
                `).join('') || '<p>Методы не найдены</p>'}
            </div>
            
            <div id="tab-properties" class="tab-content">
                <h3>Свойства (${details.properties?.length || 0})</h3>
                ${details.properties?.map(prop => `
                    <div class="property-item">
                        <h4>⚙️ ${prop.name}</h4>
                        <span class="property-type">${prop.type_name}</span>
                        ${prop.description ? `<p>${prop.description}</p>` : ''}
                    </div>
                `).join('') || '<p>Свойства не найдены</p>'}
            </div>
            
            <div id="tab-examples" class="tab-content">
                <h3>Примеры использования</h3>
                ${details.examples?.map(example => `
                    <div class="example-item">
                        <h4>${example.title}</h4>
                        <pre><code class="language-bsl">${example.code}</code></pre>
                        ${example.description ? `<p>${example.description}</p>` : ''}
                    </div>
                `).join('') || '<p>Примеры пока не добавлены</p>'}
            </div>
        </div>
    `;
}

// === ВКЛАДКИ ===
function showTab(tabName) {
    // Убираем активность со всех вкладок
    document.querySelectorAll('.tab-btn').forEach(btn => btn.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(content => content.classList.remove('active'));
    
    // Активируем выбранную вкладку
    document.querySelector(`.tab-btn[onclick="showTab('${tabName}')"]`).classList.add('active');
    document.getElementById(`tab-${tabName}`).classList.add('active');
}

// === ИЗБРАННОЕ И ЗАКЛАДКИ ===
function addToFavorites(nodeId) {
    let favorites = JSON.parse(localStorage.getItem('bsl-docs-favorites') || '[]');
    if (!favorites.includes(nodeId)) {
        favorites.push(nodeId);
        localStorage.setItem('bsl-docs-favorites', JSON.stringify(favorites));
        showNotification('⭐ Добавлено в избранное');
    }
}

function addBookmark(nodeId) {
    const name = prompt('Название закладки:');
    if (name) {
        let bookmarks = JSON.parse(localStorage.getItem('bsl-docs-bookmarks') || '{}');
        bookmarks[nodeId] = name;
        localStorage.setItem('bsl-docs-bookmarks', JSON.stringify(bookmarks));
        showNotification('🔖 Закладка создана');
    }
}

function copyNodePath(nodeId) {
    // TODO: Реализовать получение пути к узлу
    navigator.clipboard.writeText(`BSL Type: ${nodeId}`);
    showNotification('📋 Путь скопирован');
}

function copyNodeLink(nodeId) {
    const url = `${window.location.origin}${window.location.pathname}#node-${nodeId}`;
    navigator.clipboard.writeText(url);
    showNotification('🔗 Ссылка скопирована');
}

// === УВЕДОМЛЕНИЯ ===
function showNotification(message) {
    const notification = document.createElement('div');
    notification.className = 'notification';
    notification.textContent = message;
    notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        background: var(--accent-color);
        color: white;
        padding: 1rem;
        border-radius: 4px;
        z-index: 2000;
        animation: slideIn 0.3s ease;
    `;
    
    document.body.appendChild(notification);
    
    setTimeout(() => {
        notification.style.animation = 'slideOut 0.3s ease';
        setTimeout(() => notification.remove(), 300);
    }, 3000);
}

// === ИНИЦИАЛИЗАЦИЯ ===
document.addEventListener('DOMContentLoaded', function() {
    // Восстанавливаем тему из localStorage
    const savedTheme = localStorage.getItem('bsl-docs-theme') || 'dark';
    switchTheme(savedTheme);
    
    // Закрытие контекстного меню по Escape
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape') {
            const menu = document.querySelector('.context-menu');
            if (menu) menu.remove();
        }
    });
    
    console.log('🚀 BSL Interactive Tree initialized');
});
</script>

<style>
/* Дополнительные стили для нотификаций и анимаций */
@keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
}

@keyframes slideOut {
    from { transform: translateX(0); opacity: 1; }
    to { transform: translateX(100%); opacity: 0; }
}

.details-tabs {
    display: flex;
    gap: 1rem;
    margin: 1rem 0;
    border-bottom: 1px solid #3c3c3c;
}

.tab-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 0.5rem 1rem;
    cursor: pointer;
    transition: all 0.2s;
}

.tab-btn.active {
    color: var(--accent-color);
    border-bottom: 2px solid var(--accent-color);
}

.tab-content {
    display: none;
    padding: 1rem 0;
}

.tab-content.active {
    display: block;
}

.details-table {
    width: 100%;
    border-collapse: collapse;
}

.details-table td {
    padding: 0.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.details-table td:first-child {
    font-weight: bold;
    width: 120px;
}

.search-match {
    background: rgba(255, 235, 59, 0.2) !important;
}
</style>
        "#.to_string()
    }
    
    /// Рендеринг дерева типов (упрощенная версия)
    async fn render_type_tree(&self, categories: &[super::core::hierarchy::CategoryNode]) -> Result<String> {
        let mut html = String::new();
        
        html.push_str("<div class='tree-root'>\n");
        
        // Простой рендеринг без глубокой рекурсии
        for category in categories {
            let node_id = format!("category_{}", category.name.replace(" ", "_"));
            
            html.push_str(&format!(
                "<div class='tree-node category-node' id='{}' onclick='toggleTreeNode(\"{}\")'>\n\
                 <span class='tree-icon'>📁</span>\n\
                 <span class='category-name'>{}</span>\n\
                 <span class='category-count'>({} дочерних)</span>\n\
                 </div>\n",
                node_id, node_id, category.name, category.children.len()
            ));
            
            // Простое отображение дочерних элементов
            if !category.children.is_empty() {
                html.push_str("<div class='tree-children' style='margin-left: 1.5rem;'>\n");
                
                for (i, child_node) in category.children.iter().enumerate().take(10) {
                    html.push_str(&self.render_simple_node(child_node, i));
                }
                
                if category.children.len() > 10 {
                    html.push_str(&format!(
                        "<div class='tree-node more-items'>... и еще {} элементов</div>\n",
                        category.children.len() - 10
                    ));
                }
                
                html.push_str("</div>\n");
            }
        }
        
        html.push_str("</div>\n");
        
        Ok(html)
    }
    
    /// Простой рендеринг узла без рекурсии
    fn render_simple_node(&self, node: &super::core::hierarchy::DocumentationNode, index: usize) -> String {
        match node {
            super::core::hierarchy::DocumentationNode::SubCategory(sub_cat) => {
                format!(
                    "<div class='tree-node subcategory-node'>\n\
                     <span class='tree-icon'>📁</span>\n\
                     <span class='subcategory-name'>{}</span>\n\
                     </div>\n",
                    sub_cat.name
                )
            }
            super::core::hierarchy::DocumentationNode::PlatformType(platform_type) => {
                format!(
                    "<div class='tree-node type-node' onclick='selectType(\"{}\", \"{}\")'>\n\
                     <span class='type-icon'>🔧</span>\n\
                     <span class='type-name'>{}</span>\n\
                     </div>\n",
                    platform_type.base_info.id, platform_type.base_info.russian_name,
                    platform_type.base_info.russian_name
                )
            }
            super::core::hierarchy::DocumentationNode::ConfigurationType(config_type) => {
                format!(
                    "<div class='tree-node config-type-node' onclick='selectType(\"{}\", \"{}\")'>\n\
                     <span class='type-icon'>⚙️</span>\n\
                     <span class='type-name'>{}</span>\n\
                     </div>\n",
                    config_type.base_info.id, config_type.base_info.russian_name,
                    config_type.base_info.russian_name
                )
            }
            _ => {
                format!(
                    "<div class='tree-node unknown-node'>\n\
                     <span class='type-icon'>❓</span>\n\
                     <span class='type-name'>Элемент {}</span>\n\
                     </div>\n",
                    index + 1
                )
            }
        }
    }
    
    /// Рендеринг узла типа
    async fn render_type_node(&self, type_doc: &super::core::hierarchy::TypeDocumentationFull, depth: usize) -> Result<String> {
        let indent = "  ".repeat(depth);
        let node_id = format!("type_{}", type_doc.id);
        
        Ok(format!(
            "{}<div class='tree-node type-node' id='{}' onclick='selectType(\"{}\", \"{}\")'>\n\
             {}<span class='type-icon'>📄</span>\n\
             {}<span class='type-name'>{}</span>\n\
             {}<span class='type-info'>({} методов)</span>\n\
             {}</div>\n",
            indent, node_id, type_doc.id, type_doc.russian_name,
            indent, indent, type_doc.russian_name,
            indent, type_doc.methods.len(), indent
        ))
    }
    
    /// Рендеринг пагинации
    fn render_pagination(&self, pagination: &super::search::PaginationInfo) -> String {
        let mut html = String::new();
        
        html.push_str("<div class='pagination'>\n");
        
        // Кнопка "Предыдущая"
        if pagination.has_previous {
            html.push_str(&format!(
                "<button class='pagination-btn' onclick='changePage({})'>&larr; Предыдущая</button>\n",
                pagination.current_page.saturating_sub(1)
            ));
        }
        
        // Информация о страницах
        html.push_str(&format!(
            "<span class='pagination-info'>Страница {} из {}</span>\n",
            pagination.current_page + 1, pagination.total_pages
        ));
        
        // Кнопка "Следующая"
        if pagination.has_next {
            html.push_str(&format!(
                "<button class='pagination-btn' onclick='changePage({})'>Следующая &rarr;</button>\n",
                pagination.current_page + 1
            ));
        }
        
        html.push_str("</div>\n");
        
        html
    }
}

impl JsonDocumentationRenderer {
    pub fn new() -> Self {
        Self {
            serialization_settings: JsonSerializationSettings::default(),
        }
    }
    
    pub async fn render_type(&self, _type_doc: &TypeDocumentationFull) -> Result<String> {
        // TODO: JSON сериализация типа
        Ok("{}".to_string())
    }
}

impl MarkdownDocumentationRenderer {
    pub fn new() -> Self {
        Self {
            markdown_settings: MarkdownSettings::default(),
        }
    }
}

impl UiComponentLibrary {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            settings: TemplateSettings::default(),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            primary_font: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif".to_string(),
            code_font: "'Fira Code', 'JetBrains Mono', 'Consolas', monospace".to_string(),
            font_sizes: HashMap::new(),
        }
    }
}

impl Default for HtmlRenderSettings {
    fn default() -> Self {
        Self {
            enable_syntax_highlighting: true,
            enable_interactive_examples: true,
            show_breadcrumbs: true,
            enable_live_search: true,
            minify_output: false,
            enable_pwa: false,
        }
    }
}

impl Default for JsonSerializationSettings {
    fn default() -> Self {
        Self {
            pretty_print: true,
            include_nulls: false,
            compress_output: false,
            include_metadata: true,
        }
    }
}

impl Default for MarkdownSettings {
    fn default() -> Self {
        Self {
            include_toc: true,
            include_links: true,
            code_format: CodeFormat::FencedCodeBlocks,
        }
    }
}

impl Default for TemplateSettings {
    fn default() -> Self {
        Self {
            cache_templates: true,
            auto_reload: false,
            strict_mode: true,
        }
    }
}

impl Default for TreeSettings {
    fn default() -> Self {
        Self {
            lazy_loading: true,
            drag_drop: true,
            context_menus: true,
            tree_search: true,
            bookmarks: true,
            favorites: true,
            max_depth: 5,
            nodes_per_level: 50,
        }
    }
}