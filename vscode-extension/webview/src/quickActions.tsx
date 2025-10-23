import React from 'react';
import { createRoot } from 'react-dom/client';
import './tailwind.css';

function QuickActions() {
  return (
    <div className="bg-vscode-bg text-vscode-fg p-4 min-h-screen">
      <h1 className="text-xl font-bold mb-4">BSL Quick Actions (Tailwind Test)</h1>

      {/* Test VSCode theme integration */}
      <div className="mb-6">
        <input
          type="text"
          placeholder="Поиск типов..."
          className="w-full px-4 py-2 bg-vscode-input-bg text-vscode-input-fg rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Test button styles */}
      <div className="grid grid-cols-2 gap-4">
        <button className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors">
          <span className="text-2xl mb-2 block">📊</span>
          <p className="font-medium">Анализ проекта</p>
        </button>
        <button className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors">
          <span className="text-2xl mb-2 block">🔍</span>
          <p className="font-medium">Типы платформы</p>
        </button>
        <button className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors">
          <span className="text-2xl mb-2 block">⚙️</span>
          <p className="font-medium">Настройки</p>
        </button>
        <button className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors">
          <span className="text-2xl mb-2 block">📚</span>
          <p className="font-medium">Документация</p>
        </button>
      </div>
    </div>
  );
}

const container = document.getElementById('root');
if (container) {
  const root = createRoot(container);
  root.render(<QuickActions />);
}
