use leptos::prelude::*;

/// Smoke test компонент для проверки Tailwind CSS интеграции
///
/// Тестирует все категории утилит:
/// - Custom Colors (bsl-cream, bsl-teal, bsl-charcoal, etc.)
/// - Typography (font-sans, font-mono, text sizes)
/// - Spacing & Layout (padding, gaps, flex)
/// - Hover & Transitions (hover:bg-*, transition-colors, duration-fast)
/// - Facet Badges (custom gradients через CSS classes)
/// - Dark Mode (dark:bg-*, dark:text-*)
///
/// Использование:
/// ```rust
/// use crate::components::TailwindSmokeTest;
///
/// #[component]
/// fn App() -> impl IntoView {
///     view! {
///         <TailwindSmokeTest />
///     }
/// }
/// ```
#[component]
pub fn TailwindSmokeTest() -> impl IntoView {
    view! {
        <div class="max-w-2xl mx-auto p-8 space-y-6">
            {/* Header */}
            <div class="bg-bsl-cream-100 dark:bg-bsl-charcoal-800 p-6 rounded-lg shadow-md">
                <h1 class="text-3xl font-bold text-bsl-teal-700 dark:text-bsl-teal-300 mb-2">
                    "✅ Tailwind CSS Smoke Test"
                </h1>
                <p class="text-base text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Если вы видите стилизованный текст, Tailwind работает корректно!"
                </p>
            </div>

            {/* Colors Test */}
            <div class="space-y-3">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Custom Colors"
                </h2>
                <div class="flex gap-3">
                    <div class="w-16 h-16 bg-bsl-cream-50 rounded border border-bsl-charcoal-700/20"></div>
                    <div class="w-16 h-16 bg-bsl-teal-500 rounded"></div>
                    <div class="w-16 h-16 bg-bsl-charcoal-800 rounded"></div>
                    <div class="w-16 h-16 bg-bsl-red-500 rounded"></div>
                </div>
            </div>

            {/* Typography Test */}
            <div class="space-y-2">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Typography"
                </h2>
                <p class="text-xs">"text-xs (11px)"</p>
                <p class="text-sm">"text-sm (12px)"</p>
                <p class="text-base">"text-base (14px)"</p>
                <p class="text-lg">"text-lg (16px)"</p>
                <p class="font-mono text-sm">"font-mono (Berkeley Mono)"</p>
            </div>

            {/* Spacing & Layout Test */}
            <div class="space-y-2">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Spacing & Layout"
                </h2>
                <div class="flex gap-4">
                    <div class="p-2 bg-bsl-teal-500/20 rounded">"p-2"</div>
                    <div class="p-4 bg-bsl-teal-500/20 rounded">"p-4"</div>
                    <div class="p-6 bg-bsl-teal-500/20 rounded">"p-6"</div>
                </div>
            </div>

            {/* Hover & Transitions Test */}
            <div class="space-y-2">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Hover & Transitions"
                </h2>
                <button class="px-4 py-2 bg-bsl-teal-500 hover:bg-bsl-teal-600 text-white rounded transition-colors duration-fast">
                    "Hover Me!"
                </button>
            </div>

            {/* Facet Badges Test */}
            <div class="space-y-2">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Facet Badges (Custom Gradients)"
                </h2>
                <div class="flex gap-2">
                    <span class="facet-tag facet-manager">
                        "Manager"
                    </span>
                    <span class="facet-tag facet-object">
                        "Object"
                    </span>
                    <span class="facet-tag facet-reference">
                        "Reference"
                    </span>
                    <span class="facet-tag facet-selection">
                        "Selection"
                    </span>
                    <span class="facet-tag facet-list">
                        "List"
                    </span>
                </div>
            </div>

            {/* Dark Mode Test */}
            <div class="p-4 bg-bsl-red-500/10 border border-bsl-red-500/25 rounded">
                <p class="text-sm text-bsl-red-500">
                    "⚠️ Переключите тёмный режим в OS settings для проверки dark mode!"
                </p>
            </div>

            {/* Grid Layout Test */}
            <div class="space-y-2">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Grid Layout"
                </h2>
                <div class="grid grid-cols-3 gap-4">
                    <div class="bg-bsl-bg-1 p-4 rounded text-center">"Grid 1"</div>
                    <div class="bg-bsl-bg-2 p-4 rounded text-center">"Grid 2"</div>
                    <div class="bg-bsl-bg-3 p-4 rounded text-center">"Grid 3"</div>
                </div>
            </div>

            {/* Shadows Test */}
            <div class="space-y-2">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Shadows"
                </h2>
                <div class="flex gap-4">
                    <div class="p-4 bg-white shadow-xs rounded">"shadow-xs"</div>
                    <div class="p-4 bg-white shadow-sm rounded">"shadow-sm"</div>
                    <div class="p-4 bg-white shadow-md rounded">"shadow-md"</div>
                    <div class="p-4 bg-white shadow-lg rounded">"shadow-lg"</div>
                </div>
            </div>

            {/* Responsive Design Test */}
            <div class="space-y-2">
                <h2 class="text-xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100">
                    "Responsive Design"
                </h2>
                <div class="bg-bsl-teal-500/10 p-4 rounded">
                    <p class="text-sm text-bsl-slate-500">
                        "Измените размер окна браузера для проверки responsive классов"
                    </p>
                </div>
            </div>
        </div>
    }
}
