import React from 'react';
import { createRoot } from 'react-dom/client';
import './tailwind.css';

// Заглушка для Type Details Modal (будет реализована в Milestone E2)
function TypeDetailsModal() {
  return (
    <div className="bg-vscode-bg text-vscode-fg p-6 rounded-lg max-w-4xl mx-auto">
      {/* Header */}
      <div className="border-b border-vscode-fg/20 pb-4 mb-6">
        <h2 className="text-2xl font-semibold">Type Details Modal (Stub)</h2>
        <div className="flex gap-4 mt-2">
          <span className="px-3 py-1 bg-vscode-button-bg text-vscode-button-fg rounded text-sm">
            Known (100%)
          </span>
          <span className="px-3 py-1 bg-purple-600 text-white rounded text-sm">
            Manager
          </span>
        </div>
      </div>

      {/* Methods */}
      <div className="mb-6">
        <h3 className="text-lg font-semibold mb-3">Методы</h3>
        <div className="space-y-2">
          <div className="p-3 bg-vscode-input-bg hover:bg-vscode-list-hover rounded cursor-pointer transition-colors">
            <code className="text-blue-400">НайтиПоКоду()</code>
            <p className="text-sm text-vscode-fg/70 mt-1">Поиск элемента справочника по коду</p>
          </div>
          <div className="p-3 bg-vscode-input-bg hover:bg-vscode-list-hover rounded cursor-pointer transition-colors">
            <code className="text-blue-400">СоздатьЭлемент()</code>
            <p className="text-sm text-vscode-fg/70 mt-1">Создание нового элемента</p>
          </div>
        </div>
      </div>

      {/* Properties */}
      <div>
        <h3 className="text-lg font-semibold mb-3">Свойства</h3>
        <div className="grid grid-cols-2 gap-3">
          <div className="p-3 bg-vscode-input-bg rounded">
            <code className="text-green-400">Код</code>
            <p className="text-xs text-vscode-fg/70 mt-1">Строка</p>
          </div>
          <div className="p-3 bg-vscode-input-bg rounded">
            <code className="text-green-400">Наименование</code>
            <p className="text-xs text-vscode-fg/70 mt-1">Строка</p>
          </div>
        </div>
      </div>
    </div>
  );
}

const container = document.getElementById('root');
if (container) {
  const root = createRoot(container);
  root.render(<TypeDetailsModal />);
}
