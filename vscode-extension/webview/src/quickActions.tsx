import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import './tailwind.css';

declare global {
  interface Window {
    acquireVsCodeApi: () => {
      postMessage: (message: any) => void;
    };
  }
}

const vscode = window.acquireVsCodeApi();

interface TypeSearchResult {
  name: string;
  facet: string;
  certainty: string;
}

function QuickActions() {
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<TypeSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      switch (message.type) {
        case 'searchResults':
          setSearchResults(message.data);
          setIsSearching(false);
          break;
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, []);

  const handleSearch = (query: string) => {
    setSearchQuery(query);
    if (query.length >= 2) {
      setIsSearching(true);
      vscode.postMessage({ type: 'searchTypes', query });
    } else {
      setSearchResults([]);
    }
  };

  const handleActionClick = (action: string) => {
    vscode.postMessage({ type: 'executeAction', action });
  };

  const handleTypeClick = (typeName: string) => {
    vscode.postMessage({ type: 'showTypeDetails', typeName });
  };

  return (
    <div className="bg-vscode-bg text-vscode-fg p-4 min-h-screen">
      <h1 className="text-xl font-bold mb-4">BSL Quick Actions</h1>

      <div className="mb-6 relative">
        <input
          type="text"
          placeholder="Поиск типов (Справочники, Документы, Массив...)"
          value={searchQuery}
          onChange={(e) => handleSearch(e.target.value)}
          className="w-full px-4 py-2 bg-vscode-input-bg text-vscode-input-fg rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        {isSearching && (
          <div className="absolute right-3 top-2.5">
            <span className="animate-spin text-lg">⏳</span>
          </div>
        )}

        {searchResults.length > 0 && (
          <div className="absolute z-10 w-full mt-1 bg-vscode-input-bg border border-vscode-fg/20 rounded-md shadow-lg max-h-64 overflow-y-auto">
            {searchResults.map((result, idx) => (
              <div
                key={idx}
                onClick={() => handleTypeClick(result.name)}
                className="p-3 hover:bg-vscode-list-hover cursor-pointer border-b border-vscode-fg/10 last:border-b-0"
              >
                <div className="flex items-center justify-between">
                  <code className="text-sm font-medium">{result.name}</code>
                  <span className="text-xs px-2 py-0.5 bg-purple-600 text-white rounded">
                    {result.facet}
                  </span>
                </div>
                <div className="text-xs text-vscode-fg/60 mt-1">
                  {result.certainty}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <button
          onClick={() => handleActionClick('analyzeProject')}
          className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors"
        >
          <span className="text-2xl mb-2 block">📊</span>
          <p className="font-medium">Анализ проекта</p>
          <p className="text-xs text-vscode-fg/60 mt-1">Полный анализ типов</p>
        </button>

        <button
          onClick={() => handleActionClick('buildIndex')}
          className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors"
        >
          <span className="text-2xl mb-2 block">🔍</span>
          <p className="font-medium">Типы платформы</p>
          <p className="text-xs text-vscode-fg/60 mt-1">3927 типов</p>
        </button>

        <button
          onClick={() => handleActionClick('openSettings')}
          className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors"
        >
          <span className="text-2xl mb-2 block">⚙️</span>
          <p className="font-medium">Настройки</p>
          <p className="text-xs text-vscode-fg/60 mt-1">Конфигурация LSP</p>
        </button>

        <button
          onClick={() => handleActionClick('showDocs')}
          className="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors"
        >
          <span className="text-2xl mb-2 block">📚</span>
          <p className="font-medium">Документация</p>
          <p className="text-xs text-vscode-fg/60 mt-1">Открыть CLAUDE.md</p>
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
