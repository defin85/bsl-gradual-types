import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import './tailwind.css';

interface TypeMethod {
  name: string;
  description: string;
  returns?: string;
  parameters?: Array<{ name: string; type: string }>;
}

interface TypeProperty {
  name: string;
  type: string;
  readonly?: boolean;
}

interface TypeInfo {
  name: string;
  certainty: string;
  facet: string;
  methods: TypeMethod[];
  properties: TypeProperty[];
  documentation?: string;
}

declare global {
  interface Window {
    acquireVsCodeApi: () => {
      postMessage: (message: any) => void;
      setState: (state: any) => void;
      getState: () => any;
    };
  }
}

const vscode = window.acquireVsCodeApi();

export function TypeDetailsModal() {
  const [typeInfo, setTypeInfo] = useState<TypeInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      switch (message.type) {
        case 'updateTypeInfo':
          setTypeInfo(message.data);
          setLoading(false);
          break;
        case 'error':
          setError(message.error);
          setLoading(false);
          break;
      }
    };

    window.addEventListener('message', handleMessage);
    vscode.postMessage({ type: 'ready' });
    return () => window.removeEventListener('message', handleMessage);
  }, []);

  if (loading) {
    return (
      <div className="bg-vscode-bg text-vscode-fg p-6 min-h-screen flex items-center justify-center">
        <div className="text-center">
          <div className="text-4xl mb-4 animate-pulse">⏳</div>
          <p className="text-vscode-fg/70">Загрузка информации о типе...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-vscode-bg text-vscode-fg p-6 min-h-screen flex items-center justify-center">
        <div className="text-center">
          <div className="text-4xl mb-4">❌</div>
          <p className="text-red-400 mb-2">Ошибка загрузки</p>
          <p className="text-vscode-fg/70 text-sm">{error}</p>
        </div>
      </div>
    );
  }

  if (!typeInfo) {
    return (
      <div className="bg-vscode-bg text-vscode-fg p-6 min-h-screen flex items-center justify-center">
        <p className="text-vscode-fg/70">Информация о типе недоступна</p>
      </div>
    );
  }

  const certaintyClass = typeInfo.certainty === 'Known' || typeInfo.certainty.includes('100%') 
    ? 'bg-green-600 text-white'
    : typeInfo.certainty.includes('Inferred')
    ? 'bg-yellow-600 text-white'
    : 'bg-gray-600 text-white';

  const facetColors: Record<string, string> = {
    Manager: 'bg-purple-600 text-white',
    Object: 'bg-blue-600 text-white',
    Reference: 'bg-green-600 text-white',
    Selection: 'bg-orange-600 text-white',
    List: 'bg-pink-600 text-white',
    Primitive: 'bg-gray-600 text-white',
  };
  const facetClass = facetColors[typeInfo.facet] || 'bg-gray-600 text-white';

  return (
    <div className="bg-vscode-bg text-vscode-fg p-6 min-h-screen">
      <div className="max-w-5xl mx-auto">
        <div className="border-b border-vscode-fg/20 pb-6 mb-6">
          <h1 className="text-3xl font-bold mb-4">{typeInfo.name}</h1>
          <div className="flex gap-3 flex-wrap">
            <span className={`px-3 py-1.5 rounded-md text-sm font-medium ${certaintyClass}`}>
              {typeInfo.certainty}
            </span>
            <span className={`px-3 py-1.5 rounded-md text-sm font-medium ${facetClass}`}>
              {typeInfo.facet}
            </span>
          </div>
          {typeInfo.documentation && (
            <p className="mt-4 text-vscode-fg/80 text-sm leading-relaxed">
              {typeInfo.documentation}
            </p>
          )}
        </div>

        <section className="mb-8">
          <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
            <span>📝</span> Методы ({typeInfo.methods.length})
          </h2>
          {typeInfo.methods.length > 0 ? (
            <div className="space-y-3">
              {typeInfo.methods.map((method, idx) => (
                <div
                  key={idx}
                  className="p-4 bg-vscode-input-bg hover:bg-vscode-list-hover rounded-lg transition-colors group"
                >
                  <div className="flex items-start justify-between">
                    <code className="text-blue-400 font-semibold text-base">
                      {method.name}()
                    </code>
                    {method.returns && (
                      <span className="text-xs text-vscode-fg/60 bg-vscode-bg px-2 py-1 rounded">
                        → {method.returns}
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-vscode-fg/70 mt-2 leading-relaxed">
                    {method.description}
                  </p>
                  {method.parameters && method.parameters.length > 0 && (
                    <div className="mt-3 pl-4 border-l-2 border-vscode-fg/20">
                      <p className="text-xs text-vscode-fg/60 mb-1">Параметры:</p>
                      <div className="space-y-1">
                        {method.parameters.map((param, pidx) => (
                          <div key={pidx} className="text-xs">
                            <code className="text-green-400">{param.name}</code>
                            <span className="text-vscode-fg/60 ml-2">: {param.type}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <p className="text-vscode-fg/60 text-sm italic">Методы отсутствуют</p>
          )}
        </section>

        <section>
          <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
            <span>🔧</span> Свойства ({typeInfo.properties.length})
          </h2>
          {typeInfo.properties.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {typeInfo.properties.map((prop, idx) => (
                <div
                  key={idx}
                  className="p-3 bg-vscode-input-bg rounded-lg hover:bg-vscode-list-hover transition-colors"
                >
                  <div className="flex items-center justify-between mb-1">
                    <code className="text-green-400 font-semibold text-sm">
                      {prop.name}
                    </code>
                    {prop.readonly && (
                      <span className="text-xs text-vscode-fg/50">🔒</span>
                    )}
                  </div>
                  <p className="text-xs text-vscode-fg/60">{prop.type}</p>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-vscode-fg/60 text-sm italic">Свойства отсутствуют</p>
          )}
        </section>

        <div className="mt-8 pt-6 border-t border-vscode-fg/20 flex justify-end">
          <button
            onClick={() => vscode.postMessage({ type: 'close' })}
            className="px-5 py-2.5 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-md transition-colors font-medium"
          >
            Закрыть
          </button>
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
