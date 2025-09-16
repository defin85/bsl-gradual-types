// Application data and state
const appData = {
  "types": [
    {
      "id": "array",
      "name": "Массив (Array)",
      "category": "Platform",
      "certainty": 100,
      "certainty_text": "Known 100%",
      "facets": ["Object", "Collection"],
      "methods_count": 15,
      "methods": ["Добавить()", "Удалить()", "Количество()", "Найти()"],
      "source": "Static Analysis",
      "flow_sensitive": false,
      "description": "Базовый тип данных для работы с коллекциями",
      "union_types": null
    },
    {
      "id": "nomenclature",
      "name": "Справочники.Номенклатура",
      "category": "Configuration",
      "certainty": 100,
      "certainty_text": "Known 100%",
      "facets": ["Manager", "Reference", "Object", "Metadata"],
      "methods_count": 12,
      "attributes_count": 8,
      "source": "Configuration",
      "flow_sensitive": false,
      "description": "Иерархический справочник с поддержкой предопределённых элементов и групп",
      "connections": {"incoming": 5, "outgoing": 3},
      "union_types": null
    },
    {
      "id": "operation_result",
      "name": "РезультатОперации",
      "category": "Union",
      "certainty": 85,
      "certainty_text": "Inferred 85%",
      "facets": ["Object"],
      "source": "Flow Analysis",
      "flow_sensitive": true,
      "description": "Union тип с flow-sensitive анализом",
      "union_types": [
        {"type": "Булево", "probability": 70},
        {"type": "Неопределено", "probability": 30}
      ],
      "flow_analysis": {"init": "Неопределено", "check": "Булево", "final": "Булево"}
    },
    {
      "id": "dynamic_object",
      "name": "ДинамическийОбъект",
      "category": "Dynamic",
      "certainty": 30,
      "certainty_text": "Unknown 30%",
      "facets": ["Object"],
      "source": "Runtime",
      "flow_sensitive": true,
      "description": "Тип определяется во время выполнения",
      "union_types": [
        {"type": "Структура", "probability": 60},
        {"type": "ДанныеФормы", "probability": 40}
      ],
      "warning": "Требует runtime проверки",
      "recommendation": "Добавить явную типизацию или контракты"
    },
    {
      "id": "string",
      "name": "Строка (String)",
      "category": "Platform",
      "certainty": 100,
      "certainty_text": "Known 100%",
      "facets": ["Object"],
      "methods_count": 20,
      "source": "Static Analysis",
      "flow_sensitive": false,
      "description": "Базовый строковый тип данных",
      "union_types": null
    }
  ],
  "categories": {
    "Platform": {"color": "#3498db", "icon": "🔧", "count": 2},
    "Configuration": {"color": "#e74c3c", "icon": "⚙️", "count": 1},
    "Union": {"color": "#9b59b6", "icon": "🎯", "count": 1},
    "Dynamic": {"color": "#f39c12", "icon": "🔄", "count": 1},
    "Unknown": {"color": "#95a5a6", "icon": "❓", "count": 0}
  },
  "metrics": {
    "total_types": 5,
    "certainty_high": 3,
    "certainty_medium": 1,
    "certainty_low": 1,
    "flow_sensitive": 2,
    "cache_hit_rate": "94%",
    "analysis_speed": "125ms"
  },
  "connections": [
    {"source": "nomenclature", "target": "array", "type": "uses"},
    {"source": "operation_result", "target": "dynamic_object", "type": "flow"},
    {"source": "array", "target": "string", "type": "contains"},
    {"source": "nomenclature", "target": "string", "type": "attributes"},
    {"source": "operation_result", "target": "nomenclature", "type": "references"}
  ]
};

let appState = {
  currentMode: 'dashboard',
  filters: {
    categories: new Set(Object.keys(appData.categories)),
    certaintyLevels: new Set(['high', 'medium', 'low']),
    flowSensitive: false
  },
  searchQuery: '',
  selectedType: null,
  sortColumn: 'name',
  sortDirection: 'asc',
  graphZoom: 1,
  graphPan: { x: 0, y: 0 }
};

// DOM Elements
const elements = {
  modeTabs: document.querySelectorAll('.mode-tab'),
  modeContents: document.querySelectorAll('.mode-content'),
  sidebar: document.getElementById('sidebar'),
  sidebarToggle: document.getElementById('sidebarToggle'),
  mobileMenuToggle: document.getElementById('mobileMenuToggle'),
  searchInput: document.getElementById('search'),
  categoryFilters: document.getElementById('categoryFilters'),
  clearFilters: document.getElementById('clearFilters'),
  cardsContainer: document.getElementById('cardsContainer'),
  tableBody: document.getElementById('tableBody'),
  graphSvg: document.getElementById('graphSvg'),
  graphDetails: document.getElementById('graphDetails'),
  typeModal: document.getElementById('typeModal'),
  modalTitle: document.getElementById('modalTitle'),
  modalBody: document.getElementById('modalBody'),
  modalClose: document.getElementById('modalClose'),
  modalBackdrop: document.getElementById('modalBackdrop')
};

// Utility functions
function getCertaintyLevel(certainty) {
  if (certainty >= 80) return 'high';
  if (certainty >= 30) return 'medium';
  return 'low';
}

function getCategoryClass(category) {
  return `category-${category.toLowerCase()}`;
}

function formatCertainty(certainty) {
  if (certainty >= 80) return { level: 'high', color: 'var(--color-success)' };
  if (certainty >= 30) return { level: 'medium', color: 'var(--color-warning)' };
  return { level: 'low', color: 'var(--color-error)' };
}

function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

// Filter functions
function getFilteredTypes() {
  return appData.types.filter(type => {
    // Search filter
    if (appState.searchQuery && !type.name.toLowerCase().includes(appState.searchQuery.toLowerCase())) {
      return false;
    }
    
    // Category filter
    if (!appState.filters.categories.has(type.category)) {
      return false;
    }
    
    // Certainty filter
    const certaintyLevel = getCertaintyLevel(type.certainty);
    if (!appState.filters.certaintyLevels.has(certaintyLevel)) {
      return false;
    }
    
    // Flow-sensitive filter
    if (appState.filters.flowSensitive && !type.flow_sensitive) {
      return false;
    }
    
    return true;
  });
}

// Initialize application
function init() {
  setupEventListeners();
  setupFilters();
  updateDashboard();
  updateCards();
  updateTable();
  updateGraph();
  
  // Set initial mode
  switchMode('dashboard');
}

// Event listeners
function setupEventListeners() {
  // Mode switching
  elements.modeTabs.forEach(tab => {
    tab.addEventListener('click', (e) => {
      const mode = e.target.dataset.mode;
      switchMode(mode);
    });
  });
  
  // Sidebar toggle
  elements.sidebarToggle.addEventListener('click', () => {
    elements.sidebar.classList.toggle('open');
  });
  
  elements.mobileMenuToggle.addEventListener('click', () => {
    elements.sidebar.classList.toggle('open');
  });
  
  // Search
  elements.searchInput.addEventListener('input', debounce((e) => {
    appState.searchQuery = e.target.value;
    updateCurrentMode();
  }, 300));
  
  // Clear filters
  elements.clearFilters.addEventListener('click', clearAllFilters);
  
  // Modal
  elements.modalClose.addEventListener('click', closeModal);
  elements.modalBackdrop.addEventListener('click', closeModal);
  
  // Keyboard shortcuts
  document.addEventListener('keydown', (e) => {
    if (e.target.tagName === 'INPUT') return;
    
    switch(e.key) {
      case '1': switchMode('dashboard'); break;
      case '2': switchMode('cards'); break;
      case '3': switchMode('table'); break;
      case '4': switchMode('graph'); break;
      case '/': e.preventDefault(); elements.searchInput.focus(); break;
      case 'Escape': closeModal(); break;
    }
  });
  
  // Close sidebar on outside click (mobile)
  document.addEventListener('click', (e) => {
    if (window.innerWidth <= 768 && 
        elements.sidebar.classList.contains('open') &&
        !elements.sidebar.contains(e.target) &&
        !elements.mobileMenuToggle.contains(e.target)) {
      elements.sidebar.classList.remove('open');
    }
  });
}

// Setup filters
function setupFilters() {
  // Category filters
  Object.entries(appData.categories).forEach(([category, data]) => {
    if (data.count === 0) return;
    
    const filterItem = document.createElement('label');
    filterItem.className = 'filter-item';
    filterItem.innerHTML = `
      <input type="checkbox" value="${category}" checked>
      <span>${data.icon} ${category} (${data.count})</span>
    `;
    
    filterItem.querySelector('input').addEventListener('change', (e) => {
      if (e.target.checked) {
        appState.filters.categories.add(category);
      } else {
        appState.filters.categories.delete(category);
      }
      updateCurrentMode();
    });
    
    elements.categoryFilters.appendChild(filterItem);
  });
  
  // Certainty filters
  const certaintyFilters = document.querySelectorAll('input[type="checkbox"]');
  certaintyFilters.forEach(checkbox => {
    checkbox.addEventListener('change', (e) => {
      const value = e.target.value;
      if (['high', 'medium', 'low'].includes(value)) {
        if (e.target.checked) {
          appState.filters.certaintyLevels.add(value);
        } else {
          appState.filters.certaintyLevels.delete(value);
        }
        updateCurrentMode();
      } else if (value === 'flow-sensitive') {
        appState.filters.flowSensitive = e.target.checked;
        updateCurrentMode();
      }
    });
  });
}

// Mode switching
function switchMode(mode) {
  appState.currentMode = mode;
  
  // Update tabs
  elements.modeTabs.forEach(tab => {
    tab.classList.toggle('active', tab.dataset.mode === mode);
  });
  
  // Update content
  elements.modeContents.forEach(content => {
    content.classList.toggle('active', content.id === `${mode}-mode`);
  });
  
  // Close sidebar on mobile
  if (window.innerWidth <= 768) {
    elements.sidebar.classList.remove('open');
  }
  
  updateCurrentMode();
}

function updateCurrentMode() {
  switch(appState.currentMode) {
    case 'dashboard': updateDashboard(); break;
    case 'cards': updateCards(); break;
    case 'table': updateTable(); break;
    case 'graph': updateGraph(); break;
  }
}

// Dashboard Mode
function updateDashboard() {
  const filteredTypes = getFilteredTypes();
  
  // Update metrics
  document.getElementById('totalTypes').textContent = filteredTypes.length;
  document.getElementById('flowSensitive').textContent = filteredTypes.filter(t => t.flow_sensitive).length;
  
  // Update certainty progress
  const avgCertainty = filteredTypes.length > 0 
    ? Math.round(filteredTypes.reduce((sum, type) => sum + type.certainty, 0) / filteredTypes.length)
    : 0;
  
  const progressCircle = document.querySelector('.progress-ring__circle');
  const progressText = document.querySelector('.progress-ring__text');
  if (progressCircle && progressText) {
    progressCircle.style.setProperty('--progress', avgCertainty);
    progressText.textContent = `${avgCertainty}%`;
  }
  
  // Update category distribution
  const categoryChart = document.getElementById('categoryChart');
  if (categoryChart) {
    const categoryCounts = {};
    filteredTypes.forEach(type => {
      categoryCounts[type.category] = (categoryCounts[type.category] || 0) + 1;
    });
    
    categoryChart.innerHTML = Object.entries(categoryCounts)
      .map(([category, count]) => `
        <div class="category-item">
          <div class="category-color" style="background-color: ${appData.categories[category].color}"></div>
          <span>${appData.categories[category].icon} ${category}: ${count}</span>
        </div>
      `).join('');
  }
}

// Cards Mode
function updateCards() {
  const filteredTypes = getFilteredTypes();
  
  elements.cardsContainer.innerHTML = filteredTypes.map(type => {
    const categoryClass = getCategoryClass(type.category);
    const certaintyInfo = formatCertainty(type.certainty);
    
    return `
      <div class="type-card ${categoryClass}" data-type-id="${type.id}">
        <div class="type-card__header">
          <h3 class="type-card__title">${type.name}</h3>
          <span class="type-card__category">${appData.categories[type.category].icon} ${type.category}</span>
        </div>
        
        <div class="type-card__certainty">
          <div class="certainty-bar">
            <div class="certainty-fill" style="width: ${type.certainty}%"></div>
          </div>
          <span class="certainty-text">${type.certainty}%</span>
        </div>
        
        <p class="type-card__description">${type.description}</p>
        
        <div class="type-card__facets">
          ${type.facets.map(facet => `<span class="facet-badge">${facet}</span>`).join('')}
        </div>
        
        ${type.union_types ? `
          <div class="union-types">
            <h4>Union типы:</h4>
            ${type.union_types.map(union => `
              <div class="union-type">
                <span>${union.type}</span>
                <div class="union-probability">
                  <div class="union-probability-fill" style="width: ${union.probability}%"></div>
                </div>
                <span>${union.probability}%</span>
              </div>
            `).join('')}
          </div>
        ` : ''}
        
        <div class="type-card__meta">
          <span>${type.source}</span>
          <span class="flow-indicator ${type.flow_sensitive ? 'active' : ''}">
            ${type.flow_sensitive ? '🔄 Flow-sensitive' : '📊 Static'}
          </span>
        </div>
      </div>
    `;
  }).join('');
  
  // Add click handlers
  document.querySelectorAll('.type-card').forEach(card => {
    card.addEventListener('click', (e) => {
      const typeId = e.currentTarget.dataset.typeId;
      const type = appData.types.find(t => t.id === typeId);
      if (type) {
        showTypeModal(type);
      }
    });
  });
}

// Table Mode
function updateTable() {
  const filteredTypes = getFilteredTypes();
  
  // Sort types
  const sortedTypes = [...filteredTypes].sort((a, b) => {
    let aVal = a[appState.sortColumn];
    let bVal = b[appState.sortColumn];
    
    if (appState.sortColumn === 'facets') {
      aVal = a.facets.length;
      bVal = b.facets.length;
    }
    
    if (typeof aVal === 'string') {
      aVal = aVal.toLowerCase();
      bVal = bVal.toLowerCase();
    }
    
    if (appState.sortDirection === 'asc') {
      return aVal < bVal ? -1 : aVal > bVal ? 1 : 0;
    } else {
      return aVal > bVal ? -1 : aVal < bVal ? 1 : 0;
    }
  });
  
  elements.tableBody.innerHTML = sortedTypes.map(type => `
    <tr data-type-id="${type.id}">
      <td>${type.name}</td>
      <td>
        <div class="table-category" style="background-color: ${appData.categories[type.category].color}; color: white;">
          ${appData.categories[type.category].icon} ${type.category}
        </div>
      </td>
      <td>
        <div class="certainty-bar" style="width: 80px; display: inline-block;">
          <div class="certainty-fill" style="width: ${type.certainty}%"></div>
        </div>
        <span style="margin-left: 8px;">${type.certainty}%</span>
      </td>
      <td>
        <div class="table-facets">
          ${type.facets.map(facet => `<span class="facet-badge">${facet}</span>`).join('')}
        </div>
      </td>
      <td>${type.flow_sensitive ? '✅' : '❌'}</td>
      <td>
        <div class="actions-dropdown">
          <button class="actions-btn" onclick="showTypeModal(appData.types.find(t => t.id === '${type.id}'))">
            Детали
          </button>
        </div>
      </td>
    </tr>
  `).join('');
  
  // Update sort indicators
  document.querySelectorAll('.data-table th[data-sort]').forEach(th => {
    const column = th.dataset.sort;
    th.classList.toggle('sorted', column === appState.sortColumn);
    
    const indicator = th.querySelector('.sort-indicator');
    if (column === appState.sortColumn) {
      indicator.textContent = appState.sortDirection === 'asc' ? '↑' : '↓';
    } else {
      indicator.textContent = '↕';
    }
  });
  
  // Add sort handlers
  document.querySelectorAll('.data-table th[data-sort]').forEach(th => {
    th.addEventListener('click', () => {
      const column = th.dataset.sort;
      if (appState.sortColumn === column) {
        appState.sortDirection = appState.sortDirection === 'asc' ? 'desc' : 'asc';
      } else {
        appState.sortColumn = column;
        appState.sortDirection = 'asc';
      }
      updateTable();
    });
  });
}

// Graph Mode
function updateGraph() {
  const filteredTypes = getFilteredTypes();
  const svg = elements.graphSvg;
  const width = svg.clientWidth || 800;
  const height = svg.clientHeight || 500;
  
  // Clear existing content
  svg.innerHTML = '';
  
  // Create nodes and links
  const nodes = filteredTypes.map((type, index) => ({
    id: type.id,
    name: type.name,
    category: type.category,
    x: Math.random() * (width - 100) + 50,
    y: Math.random() * (height - 100) + 50,
    radius: Math.max(20, type.certainty / 5)
  }));
  
  const links = appData.connections.filter(conn => 
    nodes.find(n => n.id === conn.source) && nodes.find(n => n.id === conn.target)
  );
  
  // Create links
  const linkGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
  linkGroup.setAttribute('class', 'graph-links');
  svg.appendChild(linkGroup);
  
  links.forEach(link => {
    const sourceNode = nodes.find(n => n.id === link.source);
    const targetNode = nodes.find(n => n.id === link.target);
    
    if (sourceNode && targetNode) {
      const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      line.setAttribute('class', 'graph-link');
      line.setAttribute('x1', sourceNode.x);
      line.setAttribute('y1', sourceNode.y);
      line.setAttribute('x2', targetNode.x);
      line.setAttribute('y2', targetNode.y);
      line.setAttribute('data-source', link.source);
      line.setAttribute('data-target', link.target);
      linkGroup.appendChild(line);
    }
  });
  
  // Create nodes
  const nodeGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
  nodeGroup.setAttribute('class', 'graph-nodes');
  svg.appendChild(nodeGroup);
  
  nodes.forEach(node => {
    const group = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    group.setAttribute('class', 'graph-node-group');
    group.setAttribute('data-node-id', node.id);
    
    // Node circle
    const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    circle.setAttribute('class', 'graph-node');
    circle.setAttribute('cx', node.x);
    circle.setAttribute('cy', node.y);
    circle.setAttribute('r', node.radius);
    circle.setAttribute('fill', appData.categories[node.category].color);
    circle.setAttribute('stroke', '#fff');
    circle.setAttribute('stroke-width', '2');
    
    // Node label
    const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    text.setAttribute('class', 'graph-label');
    text.setAttribute('x', node.x);
    text.setAttribute('y', node.y + node.radius + 15);
    text.textContent = node.name.length > 15 ? node.name.substring(0, 15) + '...' : node.name;
    
    group.appendChild(circle);
    group.appendChild(text);
    nodeGroup.appendChild(group);
    
    // Add event listeners
    group.addEventListener('click', () => selectNode(node));
    group.addEventListener('mouseenter', () => highlightNode(node.id));
    group.addEventListener('mouseleave', () => removeHighlights());
    
    // Drag functionality
    let isDragging = false;
    let startX, startY;
    
    group.addEventListener('mousedown', (e) => {
      isDragging = true;
      startX = e.clientX - node.x;
      startY = e.clientY - node.y;
      svg.classList.add('dragging');
      e.preventDefault();
    });
    
    document.addEventListener('mousemove', (e) => {
      if (isDragging) {
        node.x = e.clientX - startX;
        node.y = e.clientY - startY;
        
        // Update node position
        circle.setAttribute('cx', node.x);
        circle.setAttribute('cy', node.y);
        text.setAttribute('x', node.x);
        text.setAttribute('y', node.y + node.radius + 15);
        
        // Update connected links
        document.querySelectorAll(`[data-source="${node.id}"], [data-target="${node.id}"]`).forEach(link => {
          if (link.getAttribute('data-source') === node.id) {
            link.setAttribute('x1', node.x);
            link.setAttribute('y1', node.y);
          }
          if (link.getAttribute('data-target') === node.id) {
            link.setAttribute('x2', node.x);
            link.setAttribute('y2', node.y);
          }
        });
      }
    });
    
    document.addEventListener('mouseup', () => {
      if (isDragging) {
        isDragging = false;
        svg.classList.remove('dragging');
      }
    });
  });
  
  // Graph controls
  document.getElementById('zoomIn').addEventListener('click', () => zoomGraph(1.2));
  document.getElementById('zoomOut').addEventListener('click', () => zoomGraph(0.8));
  document.getElementById('resetZoom').addEventListener('click', resetGraphView);
}

function selectNode(node) {
  // Remove previous selection
  document.querySelectorAll('.graph-node.selected').forEach(n => {
    n.classList.remove('selected');
  });
  
  // Select new node
  const nodeElement = document.querySelector(`[data-node-id="${node.id}"] .graph-node`);
  if (nodeElement) {
    nodeElement.classList.add('selected');
  }
  
  // Update details panel
  const type = appData.types.find(t => t.id === node.id);
  if (type) {
    elements.graphDetails.innerHTML = `
      <h3>${type.name}</h3>
      <p><strong>Категория:</strong> ${appData.categories[type.category].icon} ${type.category}</p>
      <p><strong>Определенность:</strong> ${type.certainty}%</p>
      <p><strong>Описание:</strong> ${type.description}</p>
      <p><strong>Фасеты:</strong> ${type.facets.join(', ')}</p>
      <p><strong>Flow-sensitive:</strong> ${type.flow_sensitive ? 'Да' : 'Нет'}</p>
      <button class="btn btn--primary btn--sm" onclick="showTypeModal(appData.types.find(t => t.id === '${type.id}'))">
        Подробнее
      </button>
    `;
  }
}

function highlightNode(nodeId) {
  // Highlight connected links
  document.querySelectorAll(`[data-source="${nodeId}"], [data-target="${nodeId}"]`).forEach(link => {
    link.classList.add('highlighted');
  });
}

function removeHighlights() {
  document.querySelectorAll('.graph-link.highlighted').forEach(link => {
    link.classList.remove('highlighted');
  });
}

function zoomGraph(factor) {
  appState.graphZoom *= factor;
  applyGraphTransform();
}

function resetGraphView() {
  appState.graphZoom = 1;
  appState.graphPan = { x: 0, y: 0 };
  applyGraphTransform();
}

function applyGraphTransform() {
  const transform = `scale(${appState.graphZoom}) translate(${appState.graphPan.x}px, ${appState.graphPan.y}px)`;
  elements.graphSvg.style.transform = transform;
}

// Modal functions
function showTypeModal(type) {
  elements.modalTitle.textContent = type.name;
  
  elements.modalBody.innerHTML = `
    <div class="type-details">
      <div class="detail-section">
        <h4>Основная информация</h4>
        <p><strong>Категория:</strong> ${appData.categories[type.category].icon} ${type.category}</p>
        <p><strong>Определенность:</strong> ${type.certainty}% (${type.certainty_text})</p>
        <p><strong>Источник:</strong> ${type.source}</p>
        <p><strong>Flow-sensitive:</strong> ${type.flow_sensitive ? 'Да' : 'Нет'}</p>
      </div>
      
      <div class="detail-section">
        <h4>Описание</h4>
        <p>${type.description}</p>
      </div>
      
      <div class="detail-section">
        <h4>Фасеты</h4>
        <div class="facets-list">
          ${type.facets.map(facet => `<span class="facet-badge">${facet}</span>`).join('')}
        </div>
      </div>
      
      ${type.methods ? `
        <div class="detail-section">
          <h4>Методы (${type.methods_count})</h4>
          <div class="methods-list">
            ${type.methods.map(method => `<code>${method}</code>`).join('')}
          </div>
        </div>
      ` : ''}
      
      ${type.union_types ? `
        <div class="detail-section">
          <h4>Union типы</h4>
          ${type.union_types.map(union => `
            <div class="union-detail">
              <span>${union.type}</span>
              <div class="union-probability-bar">
                <div class="union-probability-fill" style="width: ${union.probability}%"></div>
              </div>
              <span>${union.probability}%</span>
            </div>
          `).join('')}
        </div>
      ` : ''}
      
      ${type.flow_analysis ? `
        <div class="detail-section">
          <h4>Flow анализ</h4>
          <p><strong>Инициализация:</strong> ${type.flow_analysis.init}</p>
          <p><strong>Проверка:</strong> ${type.flow_analysis.check}</p>
          <p><strong>Финальный:</strong> ${type.flow_analysis.final}</p>
        </div>
      ` : ''}
      
      ${type.warning ? `
        <div class="detail-section warning">
          <h4>⚠️ Предупреждение</h4>
          <p>${type.warning}</p>
          ${type.recommendation ? `<p><strong>Рекомендация:</strong> ${type.recommendation}</p>` : ''}
        </div>
      ` : ''}
    </div>
  `;
  
  elements.typeModal.classList.remove('hidden');
  document.body.style.overflow = 'hidden';
}

function closeModal() {
  elements.typeModal.classList.add('hidden');
  document.body.style.overflow = '';
}

// Utility functions
function clearAllFilters() {
  appState.filters.categories = new Set(Object.keys(appData.categories));
  appState.filters.certaintyLevels = new Set(['high', 'medium', 'low']);
  appState.filters.flowSensitive = false;
  appState.searchQuery = '';
  
  // Reset UI
  elements.searchInput.value = '';
  document.querySelectorAll('input[type="checkbox"]').forEach(cb => {
    if (['high', 'medium', 'low'].includes(cb.value) || Object.keys(appData.categories).includes(cb.value)) {
      cb.checked = true;
    } else {
      cb.checked = false;
    }
  });
  
  updateCurrentMode();
}

// Make functions globally available
window.showTypeModal = showTypeModal;
window.appData = appData;

// Initialize application when DOM is loaded
document.addEventListener('DOMContentLoaded', init);

// Handle window resize
window.addEventListener('resize', debounce(() => {
  if (appState.currentMode === 'graph') {
    updateGraph();
  }
}, 250));