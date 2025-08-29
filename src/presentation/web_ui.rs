//! Улучшенная HTML страница с иерархическим отображением типов BSL

/// Генерация улучшенной HTML главной страницы с иерархией типов
pub fn generate_enhanced_index_html() -> String {
    r#"
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BSL Type Browser - Иерархический просмотр типов</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0d1117; 
            color: #c9d1d9; 
            line-height: 1.6;
            overflow-x: hidden;
        }
        
        .app-container {
            display: flex;
            height: 100vh;
        }
        
        /* Боковая панель с иерархией */
        .sidebar {
            width: 350px;
            background: #161b22;
            border-right: 1px solid #30363d;
            overflow-y: auto;
            padding: 20px 0;
            position: relative;
        }
        
        .sidebar-header {
            padding: 0 20px 20px;
            border-bottom: 1px solid #21262d;
        }
        
        .sidebar h2 {
            color: #58a6ff;
            font-size: 1.2em;
            margin-bottom: 15px;
        }
        
        .search-input {
            width: 100%;
            padding: 10px 12px;
            background: #0d1117;
            border: 1px solid #30363d;
            border-radius: 6px;
            color: #c9d1d9;
            font-size: 14px;
        }
        
        .search-input:focus {
            outline: none;
            border-color: #58a6ff;
            box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.3);
        }
        
        /* Дерево категорий */
        .tree {
            padding: 20px 0;
        }
        
        .tree-category {
            margin-bottom: 2px;
        }
        
        .category-header {
            display: flex;
            align-items: center;
            padding: 8px 20px;
            cursor: pointer;
            border-radius: 6px;
            margin: 0 10px;
            transition: background 0.2s;
        }
        
        .category-header:hover {
            background: #21262d;
        }
        
        .category-header.active {
            background: #1f6feb;
            color: white;
        }
        
        .category-icon {
            margin-right: 8px;
            font-size: 14px;
            transition: transform 0.2s;
        }
        
        .category-header.expanded .category-icon {
            transform: rotate(90deg);
        }
        
        .category-name {
            flex: 1;
            font-weight: 500;
        }
        
        .category-count {
            background: #30363d;
            color: #8b949e;
            padding: 2px 8px;
            border-radius: 10px;
            font-size: 12px;
            font-weight: normal;
        }
        
        .category-types {
            display: none;
            margin-left: 20px;
            border-left: 1px solid #30363d;
        }
        
        .category-types.expanded {
            display: block;
        }
        
        .type-item {
            display: flex;
            align-items: center;
            padding: 6px 20px 6px 30px;
            cursor: pointer;
            transition: background 0.2s;
            border-radius: 0 6px 6px 0;
            margin-right: 10px;
        }
        
        .type-item:hover {
            background: #161b22;
        }
        
        .type-item.selected {
            background: #0969da;
            color: white;
        }
        
        .type-icon {
            margin-right: 8px;
            color: #7d8590;
        }
        
        .type-item.selected .type-icon {
            color: white;
        }
        
        /* Основная область */
        .main-content {
            flex: 1;
            display: flex;
            flex-direction: column;
        }
        
        .main-header {
            background: #161b22;
            border-bottom: 1px solid #30363d;
            padding: 20px 30px;
        }
        
        .main-title {
            color: #58a6ff;
            font-size: 1.8em;
            margin-bottom: 8px;
        }
        
        .main-subtitle {
            color: #8b949e;
            font-size: 1em;
        }
        
        .main-body {
            flex: 1;
            padding: 30px;
            overflow-y: auto;
        }
        
        /* Карточки типов */
        .type-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .type-card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 20px;
            transition: all 0.2s;
            cursor: pointer;
        }
        
        .type-card:hover {
            border-color: #58a6ff;
            box-shadow: 0 4px 12px rgba(88, 166, 255, 0.15);
            transform: translateY(-2px);
        }
        
        .type-card-header {
            display: flex;
            align-items: center;
            margin-bottom: 12px;
        }
        
        .type-card-icon {
            font-size: 24px;
            margin-right: 12px;
        }
        
        .type-card-title {
            color: #f0f6fc;
            font-size: 1.2em;
            font-weight: 600;
        }
        
        .type-card-description {
            color: #8b949e;
            margin-bottom: 15px;
            line-height: 1.5;
        }
        
        .type-card-stats {
            display: flex;
            gap: 15px;
        }
        
        .type-stat {
            display: flex;
            align-items: center;
            color: #7d8590;
            font-size: 13px;
        }
        
        .type-stat-icon {
            margin-right: 4px;
        }
        
        /* Детальная информация о типе */
        .type-details {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 25px;
        }
        
        .details-header {
            display: flex;
            align-items: center;
            margin-bottom: 20px;
            padding-bottom: 15px;
            border-bottom: 1px solid #21262d;
        }
        
        .details-icon {
            font-size: 28px;
            margin-right: 15px;
        }
        
        .details-title {
            flex: 1;
        }
        
        .details-name {
            color: #f0f6fc;
            font-size: 1.5em;
            font-weight: 600;
            margin-bottom: 4px;
        }
        
        .details-category {
            color: #7d8590;
            font-size: 14px;
        }
        
        .details-section {
            margin-bottom: 25px;
        }
        
        .section-title {
            color: #58a6ff;
            font-size: 1.1em;
            font-weight: 600;
            margin-bottom: 12px;
            display: flex;
            align-items: center;
        }
        
        .section-icon {
            margin-right: 8px;
        }
        
        .methods-list, .properties-list {
            display: grid;
            gap: 8px;
        }
        
        .method-item, .property-item {
            background: #0d1117;
            border: 1px solid #21262d;
            border-radius: 6px;
            padding: 12px 15px;
            transition: border-color 0.2s;
        }
        
        .method-item:hover, .property-item:hover {
            border-color: #30363d;
        }
        
        .method-signature, .property-signature {
            color: #79c0ff;
            font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', monospace;
            font-size: 14px;
            margin-bottom: 4px;
        }
        
        .method-description, .property-description {
            color: #8b949e;
            font-size: 13px;
        }
        
        /* Статистика */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .stat-card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 20px;
            text-align: center;
        }
        
        .stat-value {
            color: #79c0ff;
            font-size: 2.2em;
            font-weight: 700;
            margin-bottom: 8px;
        }
        
        .stat-label {
            color: #8b949e;
            font-size: 14px;
            font-weight: 500;
        }
        
        /* Утилиты */
        .loading {
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 40px;
            color: #8b949e;
        }
        
        .loading-spinner {
            margin-right: 10px;
            animation: spin 1s linear infinite;
        }
        
        @keyframes spin {
            from { transform: rotate(0deg); }
            to { transform: rotate(360deg); }
        }
        
        .error {
            background: #5a1e1e;
            border: 1px solid #f85149;
            color: #f85149;
            padding: 15px;
            border-radius: 6px;
            margin: 20px 0;
        }
        
        .empty-state {
            text-align: center;
            padding: 60px 20px;
            color: #8b949e;
        }
        
        .empty-state-icon {
            font-size: 48px;
            margin-bottom: 16px;
            opacity: 0.5;
        }
        
        /* Адаптивность */
        @media (max-width: 768px) {
            .app-container {
                flex-direction: column;
            }
            
            .sidebar {
                width: 100%;
                height: auto;
                position: static;
            }
            
            .main-content {
                height: auto;
            }
            
            .type-grid {
                grid-template-columns: 1fr;
            }
            
            .stats-grid {
                grid-template-columns: repeat(2, 1fr);
            }
        }
        
        /* Кнопки */
        .btn {
            display: inline-flex;
            align-items: center;
            padding: 6px 12px;
            border: 1px solid #30363d;
            border-radius: 6px;
            background: #21262d;
            color: #c9d1d9;
            text-decoration: none;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
        }
        
        .btn:hover {
            background: #30363d;
            border-color: #8b949e;
        }
        
        .btn-primary {
            background: #1f6feb;
            border-color: #1f6feb;
            color: white;
        }
        
        .btn-primary:hover {
            background: #0969da;
            border-color: #0969da;
        }
    </style>
</head>
<body>
    <div class="app-container">
        <!-- Боковая панель с иерархией -->
        <div class="sidebar">
            <div class="sidebar-header">
                <h2>📚 BSL Types</h2>
                <input type="text" class="search-input" id="typeSearch" placeholder="Поиск типов...">
            </div>
            
            <div class="tree" id="typeTree">
                <div class="loading">
                    <div class="loading-spinner">⏳</div>
                    Загрузка типов...
                </div>
            </div>
        </div>
        
        <!-- Основная область -->
        <div class="main-content">
            <div class="main-header">
                <h1 class="main-title">🚀 BSL Gradual Type System</h1>
                <p class="main-subtitle">Production-ready система типов для 1С:Предприятие</p>
            </div>
            
            <div class="main-body" id="mainContent">
                <!-- Статистика -->
                <div class="stats-grid" id="statsGrid">
                    <div class="stat-card">
                        <div class="stat-value" id="totalTypes">-</div>
                        <div class="stat-label">Всего типов</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="platformTypes">-</div>
                        <div class="stat-label">Платформенных</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="methodsCount">-</div>
                        <div class="stat-label">Методов</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="propertiesCount">-</div>
                        <div class="stat-label">Свойств</div>
                    </div>
                </div>
                
                <!-- Контент по умолчанию -->
                <div id="defaultContent">
                    <div class="empty-state">
                        <div class="empty-state-icon">📋</div>
                        <h3>Выберите тип для просмотра</h3>
                        <p>Используйте дерево типов слева для навигации</p>
                    </div>
                </div>
                
                <!-- Сетка типов -->
                <div class="type-grid" id="typeGrid" style="display: none;">
                    <!-- Будет заполнено JS -->
                </div>
                
                <!-- Детальная информация о типе -->
                <div class="type-details" id="typeDetails" style="display: none;">
                    <!-- Будет заполнено JS -->
                </div>
            </div>
        </div>
    </div>
    
    <script>
        // Глобальное состояние
        let currentTypes = [];
        let currentCategory = null;
        let currentType = null;
        
        // Категории типов BSL (базовая структура)
        const typeCategories = {
            "platform": {
                name: "🏗️ Платформенные типы",
                icon: "🏗️",
                subcategories: {
                    "collections": {
                        name: "Универсальные коллекции",
                        icon: "📦",
                        types: ["Массив", "Структура", "Соответствие", "ТаблицаЗначений", "СписокЗначений"]
                    },
                    "io": {
                        name: "Файлы и потоки",
                        icon: "📁",
                        types: ["Файл", "ТекстовыйДокумент", "ДвоичныеДанные", "ПотокВПамяти"]
                    },
                    "system": {
                        name: "Системные объекты",
                        icon: "⚙️",
                        types: ["ОбъектXDTO", "WSПрокси", "HTTPЗапрос", "HTTPОтвет", "WSОпределения"]
                    }
                }
            },
            "metadata": {
                name: "🗃️ Объекты метаданных",
                icon: "🗃️",
                subcategories: {
                    "catalogs": {
                        name: "Справочники",
                        icon: "📇",
                        types: []
                    },
                    "documents": {
                        name: "Документы",
                        icon: "📋",
                        types: []
                    },
                    "registers": {
                        name: "Регистры",
                        icon: "📊",
                        types: []
                    }
                }
            },
            "user": {
                name: "👤 Пользовательские типы",
                icon: "👤",
                subcategories: {}
            }
        };
        
        // Инициализация
        document.addEventListener('DOMContentLoaded', function() {
            initializeApp();
        });
        
        async function initializeApp() {
            await loadStats();
            await loadTypeTree();
            setupEventListeners();
        }
        
        // Загрузка статистики
        async function loadStats() {
            try {
                const response = await fetch('/api/stats');
                const stats = await response.json();
                
                document.getElementById('totalTypes').textContent = stats.platform_types || 0;
                document.getElementById('platformTypes').textContent = stats.platform_types || 0;
                document.getElementById('methodsCount').textContent = stats.total_functions || 0;
                document.getElementById('propertiesCount').textContent = stats.total_variables || 0;
            } catch (error) {
                console.error('Ошибка загрузки статистики:', error);
            }
        }
        
        // Загрузка дерева типов
        async function loadTypeTree() {
            const treeContainer = document.getElementById('typeTree');
            
            try {
                // Пытаемся получить категории из API
                let categories;
                try {
                    const response = await fetch('/api/v1/categories');
                    categories = await response.json();
                } catch {
                    // Используем статические данные
                    categories = { categories: [] };
                }
                
                // Строим дерево
                const treeHTML = buildTypeTree(typeCategories);
                treeContainer.innerHTML = treeHTML;
                
            } catch (error) {
                console.error('Ошибка загрузки дерева типов:', error);
                treeContainer.innerHTML = '<div class="error">❌ Ошибка загрузки типов</div>';
            }
        }
        
        // Построение HTML дерева
        function buildTypeTree(categories) {
            let html = '';
            
            for (const [catId, category] of Object.entries(categories)) {
                html += `
                    <div class="tree-category">
                        <div class="category-header" onclick="toggleCategory('${catId}')">
                            <span class="category-icon">▶</span>
                            <span class="category-name">${category.name}</span>
                            <span class="category-count">${Object.keys(category.subcategories || {}).length}</span>
                        </div>
                        <div class="category-types" id="category-${catId}">
                            ${buildSubcategories(catId, category.subcategories || {})}
                        </div>
                    </div>
                `;
            }
            
            return html;
        }
        
        // Построение подкатегорий
        function buildSubcategories(parentId, subcategories) {
            let html = '';
            
            for (const [subId, subcategory] of Object.entries(subcategories)) {
                const fullId = `${parentId}-${subId}`;
                html += `
                    <div class="tree-category">
                        <div class="category-header" onclick="toggleSubcategory('${fullId}')">
                            <span class="category-icon">▶</span>
                            <span class="category-name">${subcategory.icon} ${subcategory.name}</span>
                            <span class="category-count">${subcategory.types.length}</span>
                        </div>
                        <div class="category-types" id="subcategory-${fullId}">
                            ${buildTypeItems(subcategory.types)}
                        </div>
                    </div>
                `;
            }
            
            return html;
        }
        
        // Построение элементов типов
        function buildTypeItems(types) {
            return types.map(type => `
                <div class="type-item" onclick="selectType('${type}')">
                    <span class="type-icon">🔧</span>
                    <span>${type}</span>
                </div>
            `).join('');
        }
        
        // Переключение категории
        function toggleCategory(categoryId) {
            const header = document.querySelector(`[onclick="toggleCategory('${categoryId}')"]`);
            const content = document.getElementById(`category-${categoryId}`);
            const icon = header.querySelector('.category-icon');
            
            if (content.classList.contains('expanded')) {
                content.classList.remove('expanded');
                header.classList.remove('expanded');
                icon.textContent = '▶';
            } else {
                content.classList.add('expanded');
                header.classList.add('expanded');
                icon.textContent = '▼';
            }
        }
        
        // Переключение подкатегории
        function toggleSubcategory(subcategoryId) {
            const header = document.querySelector(`[onclick="toggleSubcategory('${subcategoryId}')"]`);
            const content = document.getElementById(`subcategory-${subcategoryId}`);
            const icon = header.querySelector('.category-icon');
            
            if (content.classList.contains('expanded')) {
                content.classList.remove('expanded');
                header.classList.remove('expanded');
                icon.textContent = '▶';
            } else {
                content.classList.add('expanded');
                header.classList.add('expanded');
                icon.textContent = '▼';
                
                // Загружаем типы для подкатегории
                loadTypesForSubcategory(subcategoryId);
            }
        }
        
        // Загрузка типов для подкатегории
        async function loadTypesForSubcategory(subcategoryId) {
            try {
                // Определяем поисковый запрос на основе подкатегории
                let searchQuery = '';
                if (subcategoryId.includes('collections')) {
                    searchQuery = 'Массив|Структура|Соответствие|Таблица';
                } else if (subcategoryId.includes('io')) {
                    searchQuery = 'Файл|Поток|Данные';
                } else if (subcategoryId.includes('system')) {
                    searchQuery = 'XDTO|HTTP|WS';
                }
                
                if (searchQuery) {
                    const response = await fetch(`/api/types?search=${encodeURIComponent(searchQuery)}&per_page=20`);
                    const data = await response.json();
                    
                    // Обновляем содержимое подкатегории
                    const container = document.getElementById(`subcategory-${subcategoryId}`);
                    const typeItems = data.types.map(type => `
                        <div class="type-item" onclick="selectType('${type.name}')">
                            <span class="type-icon">🔧</span>
                            <span>${type.name}</span>
                        </div>
                    `).join('');
                    
                    container.innerHTML = typeItems;
                }
            } catch (error) {
                console.error('Ошибка загрузки типов:', error);
            }
        }
        
        // Выбор типа
        async function selectType(typeName) {
            // Убираем предыдущее выделение
            document.querySelectorAll('.type-item.selected').forEach(item => {
                item.classList.remove('selected');
            });
            
            // Выделяем текущий элемент
            event.target.closest('.type-item').classList.add('selected');
            
            // Скрываем другие контенты
            document.getElementById('defaultContent').style.display = 'none';
            document.getElementById('typeGrid').style.display = 'none';
            
            // Показываем детали типа
            await showTypeDetails(typeName);
        }
        
        // Отображение деталей типа
        async function showTypeDetails(typeName) {
            const detailsContainer = document.getElementById('typeDetails');
            detailsContainer.style.display = 'block';
            
            try {
                const response = await fetch(`/api/types/${encodeURIComponent(typeName)}`);
                const typeDetails = await response.json();
                
                detailsContainer.innerHTML = `
                    <div class="details-header">
                        <div class="details-icon">🔧</div>
                        <div class="details-title">
                            <div class="details-name">${typeDetails.name}</div>
                            <div class="details-category">${typeDetails.category}</div>
                        </div>
                        <button class="btn btn-primary" onclick="openInNewTab('${typeDetails.name}')">
                            📖 Документация
                        </button>
                    </div>
                    
                    ${typeDetails.description ? `
                        <div class="details-section">
                            <h3 class="section-title">
                                <span class="section-icon">📝</span>
                                Описание
                            </h3>
                            <p>${typeDetails.description}</p>
                        </div>
                    ` : ''}
                    
                    ${typeDetails.methods.length > 0 ? `
                        <div class="details-section">
                            <h3 class="section-title">
                                <span class="section-icon">⚙️</span>
                                Методы (${typeDetails.methods.length})
                            </h3>
                            <div class="methods-list">
                                ${typeDetails.methods.map(method => `
                                    <div class="method-item">
                                        <div class="method-signature">${method.name}(${method.parameters.join(', ')})</div>
                                        ${method.return_type ? `<div class="method-signature">→ ${method.return_type}</div>` : ''}
                                        ${method.description ? `<div class="method-description">${method.description}</div>` : ''}
                                    </div>
                                `).join('')}
                            </div>
                        </div>
                    ` : ''}
                    
                    ${typeDetails.properties.length > 0 ? `
                        <div class="details-section">
                            <h3 class="section-title">
                                <span class="section-icon">🏷️</span>
                                Свойства (${typeDetails.properties.length})
                            </h3>
                            <div class="properties-list">
                                ${typeDetails.properties.map(prop => `
                                    <div class="property-item">
                                        <div class="property-signature">${prop.name}: ${prop.type_name}</div>
                                        ${prop.readonly ? '<span class="readonly-badge">только чтение</span>' : ''}
                                        ${prop.description ? `<div class="property-description">${prop.description}</div>` : ''}
                                    </div>
                                `).join('')}
                            </div>
                        </div>
                    ` : ''}
                    
                    ${typeDetails.related_types.length > 0 ? `
                        <div class="details-section">
                            <h3 class="section-title">
                                <span class="section-icon">🔗</span>
                                Связанные типы
                            </h3>
                            <div class="related-types">
                                ${typeDetails.related_types.map(relType => `
                                    <button class="btn" onclick="selectType('${relType}')">${relType}</button>
                                `).join(' ')}
                            </div>
                        </div>
                    ` : ''}
                `;
                
            } catch (error) {
                console.error('Ошибка загрузки деталей типа:', error);
                detailsContainer.innerHTML = `
                    <div class="error">
                        ❌ Ошибка загрузки деталей типа "${typeName}"
                    </div>
                `;
            }
        }
        
        // Настройка обработчиков событий
        function setupEventListeners() {
            // Поиск типов
            const searchInput = document.getElementById('typeSearch');
            let searchTimeout;
            
            searchInput.addEventListener('input', (e) => {
                clearTimeout(searchTimeout);
                searchTimeout = setTimeout(() => {
                    performSearch(e.target.value);
                }, 300);
            });
        }
        
        // Выполнение поиска
        async function performSearch(query) {
            if (!query.trim()) {
                await loadTypeTree();
                return;
            }
            
            try {
                const response = await fetch(`/api/types?search=${encodeURIComponent(query)}&per_page=50`);
                const data = await response.json();
                
                const treeContainer = document.getElementById('typeTree');
                
                if (data.types.length === 0) {
                    treeContainer.innerHTML = `
                        <div class="empty-state">
                            <div class="empty-state-icon">🔍</div>
                            <p>Типы не найдены</p>
                        </div>
                    `;
                    return;
                }
                
                // Отображаем результаты поиска
                const searchResults = data.types.map(type => `
                    <div class="type-item" onclick="selectType('${type.name}')">
                        <span class="type-icon">🔧</span>
                        <span>${type.name}</span>
                    </div>
                `).join('');
                
                treeContainer.innerHTML = `
                    <div class="search-results">
                        <h3 style="color: #58a6ff; margin: 0 20px 15px; font-size: 14px;">
                            🔍 Результаты поиска (${data.types.length})
                        </h3>
                        ${searchResults}
                    </div>
                `;
                
            } catch (error) {
                console.error('Ошибка поиска:', error);
            }
        }
        
        // Открыть документацию в новой вкладке
        function openInNewTab(typeName) {
            window.open(`https://its.1c.ru/db/v8std/content/${typeName}/`, '_blank');
        }
    </script>
</body>
</html>
    "#.to_string()
}
