//! Основные типы IR (Intermediate Representation)
//!
//! Содержит SemanticNode, SemanticNodeKind, Parameter, FunctionSignature,
//! VariableState, MemberAccessKind.

use serde::{Deserialize, Serialize};

use crate::domain::types::TypeResolution;

use super::span::Span;
use super::symbol_table::ScopeId;

/// Упрощённое семантическое представление элементов программы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNode {
    /// Тип узла
    pub kind: SemanticNodeKind,

    /// Позиция в исходном коде (для diagnostics и hover)
    pub span: Span,

    /// ID scope, к которому принадлежит узел
    pub scope_id: ScopeId,
}

/// Тип доступа к члену объекта
///
/// Расширяемый enum для представления различных видов обращения к членам объекта.
/// Поддерживает вызовы методов, доступ к свойствам и индексированный доступ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberAccessKind {
    /// Вызов метода: `obj.Method()`
    Method,
    /// Доступ к свойству: `obj.Property`
    Property,
    /// Индексированный доступ: `obj[index]` (для будущего расширения)
    Indexer,
}

impl MemberAccessKind {
    /// Проверяет, является ли это вызовом метода
    pub fn is_method(&self) -> bool {
        matches!(self, MemberAccessKind::Method)
    }

    /// Проверяет, является ли это доступом к свойству
    pub fn is_property(&self) -> bool {
        matches!(self, MemberAccessKind::Property)
    }

    /// Проверяет, является ли это индексированным доступом
    pub fn is_indexer(&self) -> bool {
        matches!(self, MemberAccessKind::Indexer)
    }
}

/// Виды семантических узлов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticNodeKind {
    // === Базовые объявления ===
    /// Объявление переменной: `Перем x: Число;`
    ///
    /// # Phase 3: TypeResolution для type_hint и initial_value_type
    ///
    /// - `type_hint` - явная аннотация типа из исходного кода (Explicit source)
    /// - `initial_value_type` - выведенный тип из инициализирующего значения (Inferred source)
    VariableDeclaration {
        name: String,
        /// Phase 3: TypeResolution вместо String для явной аннотации типа
        type_hint: Option<TypeResolution>,
        is_export: bool,
        /// Phase 3: TypeResolution вместо String для типа начального значения
        initial_value_type: Option<TypeResolution>,
    },

    /// Доступ к переменной в выражении: `x`
    VariableAccess {
        name: String,
    },

    /// Присваивание: `x = 42;`
    ///
    /// # Phase 3: TypeResolution для value_type
    ///
    /// `value_type` теперь содержит полную информацию о выведенном типе:
    /// - Certainty (уверенность в типе)
    /// - ResolutionSource (откуда пришёл тип)
    /// - Metadata (дополнительная информация)
    Assignment {
        variable: String,
        /// Phase 3: TypeResolution вместо String для полной информации о типе
        value_type: TypeResolution,
        value_node: Option<usize>, // MILESTONE 3.5: индекс узла value expression (для hover)
    },

    /// Объявление функции
    ///
    /// # Phase 3: TypeResolution для return_type
    ///
    /// `return_type` теперь содержит полную информацию о возвращаемом типе:
    /// - Explicit если явно указан в коде
    /// - Inferred если выведен из return statements
    ///
    /// # Context-Aware Validation
    ///
    /// `compiler_directive` содержит директиву компилятора из исходного кода
    /// для context-aware валидации (например, &НаСервере, &НаКлиенте).
    FunctionDeclaration {
        name: String,
        params: Vec<Parameter>,
        /// Phase 3: TypeResolution вместо String для возвращаемого типа
        return_type: Option<TypeResolution>,
        body_scope: ScopeId,
        body: Vec<usize>, // индексы узлов тела функции
        /// Директива компилятора для context-aware валидации
        compiler_directive: Option<crate::domain::CompilerDirective>,
    },

    /// Объявление процедуры
    ///
    /// # Context-Aware Validation
    ///
    /// `compiler_directive` содержит директиву компилятора из исходного кода
    /// для context-aware валидации (например, &НаСервере, &НаКлиенте).
    ProcedureDeclaration {
        name: String,
        params: Vec<Parameter>,
        body_scope: ScopeId,
        body: Vec<usize>, // индексы узлов тела процедуры
        /// Директива компилятора для context-aware валидации
        compiler_directive: Option<crate::domain::CompilerDirective>,
    },

    // === Control Flow (КРИТИЧНО для Milestone 2.3 flow-sensitive) ===
    /// Условный оператор: `Если условие Тогда ... Иначе ... КонецЕсли`
    IfStatement {
        /// Phase 3: TypeResolution для условия (обычно Булево)
        condition_type: TypeResolution,
        then_branch: Vec<usize>, // Индексы SemanticNode в then ветке
        else_branch: Option<Vec<usize>>,
    },

    /// Цикл While: `Пока условие Цикл ... КонецЦикла`
    WhileLoop {
        /// Phase 3: TypeResolution для условия (обычно Булево)
        condition_type: TypeResolution,
        body: Vec<usize>,
    },

    /// Цикл For: `Для i = 1 По 10 Цикл ... КонецЦикла`
    ForLoop {
        variable: String,
        /// Phase 3: TypeResolution для диапазона (всегда Число для For loop)
        range_type: TypeResolution,
        body: Vec<usize>,
    },

    /// Цикл ForEach: `Для Каждого элемент Из коллекция Цикл ... КонецЦикла`
    ForEachLoop {
        variable: String,
        /// Phase 3: TypeResolution для коллекции (массив, соответствие и т.д.)
        collection_type: TypeResolution,
        body: Vec<usize>,
    },

    /// Возврат из функции: `Возврат значение;`
    Return {
        /// Phase 3: TypeResolution для возвращаемого значения
        value_type: Option<TypeResolution>,
    },

    /// Прерывание цикла: `Прервать;`
    Break,

    /// Продолжение цикла: `Продолжить;`
    Continue,

    /// Обработка исключений: `Попытка ... Исключение ... КонецПопытки`
    TryExcept {
        try_body: Vec<usize>,
        except_body: Vec<usize>,
    },

    // === Global Property Access (платформенные менеджеры) ===
    /// Доступ к глобальному свойству платформы: `Справочники`, `Документы`, `РегистрыСведений`
    ///
    /// # Семантика
    ///
    /// Глобальные свойства платформы — это точки входа к менеджерам объектов метаданных.
    /// Они всегда доступны в глобальной области видимости и возвращают Manager-типы.
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// Справочники          // GlobalPropertyAccess { name: "Справочники", result_type: СправочникиМенеджер }
    /// Документы            // GlobalPropertyAccess { name: "Документы", result_type: ДокументыМенеджер }
    /// РегистрыСведений     // GlobalPropertyAccess { name: "РегистрыСведений", result_type: РегистрыСведенийМенеджер }
    /// ```
    GlobalPropertyAccess {
        /// Имя глобального свойства: "Справочники", "Документы", "РегистрыСведений"
        name: String,
        /// Тип результата: СправочникиМенеджер, ДокументыМенеджер, etc.
        result_type: TypeResolution,
    },

    // === Member Access (КРИТИЧНО для LSP hover) ===
    /// Доступ к члену объекта: `объект.свойство` или `объект.Метод()`
    ///
    /// # Семантика полей
    ///
    /// - `object_node`: **Индекс узла-объекта** для цепочек доступа
    ///   - Some(index) для вложенных выражений (GlobalPropertyAccess, MemberAccess, FunctionCall)
    ///   - None для простых переменных (используется object_name)
    /// - `object_name`: **Имя переменной** из исходного кода (Some("МассивДанных"))
    ///   - Some(name) для простых переменных (Identifier)
    ///   - None для сложных выражений (PropertyAccess, Call, New, etc.)
    /// - `object_type`: **Тип объекта СЛЕВА от точки** (результат type inference)
    /// - `member_name`: Имя свойства или метода (например, "Добавить")
    /// - `access_kind`: Тип доступа (Method, Property, Indexer)
    /// - `result_type`: **Тип РЕЗУЛЬТАТА доступа** (тип значения после разрешения члена)
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// МассивДанных = Новый Массив();
    /// МассивДанных.Добавить("текст");  // object_name=Some("МассивДанных"), access_kind=Method
    ///
    /// obj.prop1.prop2.Метод();  // object_name=None, object_node=Some(...), access_kind=Method
    /// obj.Свойство;             // access_kind=Property
    ///
    /// // Цепочка: Справочники.Контрагенты
    /// // GlobalPropertyAccess(Справочники) → MemberAccess(object_node=GlobalPropertyAccess, member_name=Контрагенты)
    /// ```
    MemberAccess {
        /// НОВОЕ: Индекс узла-объекта для цепочек (GlobalPropertyAccess, MemberAccess, FunctionCall)
        object_node: Option<usize>,
        /// Имя переменной (для простых переменных, deprecated для цепочек)
        object_name: Option<String>,
        /// Phase 3: TypeResolution для типа объекта СЛЕВА от точки
        object_type: TypeResolution,
        member_name: String,
        /// Тип доступа к члену: метод, свойство или индексатор
        access_kind: MemberAccessKind,
        /// НОВОЕ: Тип РЕЗУЛЬТАТА доступа (тип значения справа от точки)
        result_type: TypeResolution,
    },

    /// Вызов функции или метода: `Функция()` или `объект.Метод(args)`
    ///
    /// # Семантика полей
    ///
    /// - `function_name`: Имя функции/метода
    /// - `object_name`: **Имя переменной-объекта** для вызовов методов
    ///   - Some(name) для методов переменных: `МассивДанных.Добавить("x")`
    ///   - None для глобальных функций: `Сообщить("текст")`
    ///   - None для сложных выражений: `ПолучитьОбъект().Метод()`
    /// - `object_type`: **Тип объекта** для вызовов методов (Phase 3: TypeResolution)
    ///   - Some(TypeResolution) для методов
    ///   - None для глобальных функций
    /// - `arg_types`: Типы аргументов вызова (Phase 3: Vec<TypeResolution>)
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// Сообщить("текст");  // object_name=None, object_type=None
    /// МассивДанных.Добавить("x");  // object_name=Some("МассивДанных"), object_type=Some(TypeResolution)
    /// ```
    ///
    /// # Phase 3: TypeResolution для object_type и arg_types
    ///
    /// - `object_type` и `arg_types` теперь содержат полную информацию о типах
    /// - Для Unknown типов валидация пропускается (graceful degradation)
    /// - `object_name` по-прежнему String для flow-sensitive анализа (lookup в SymbolTable)
    FunctionCall {
        function_name: String,
        object_name: Option<String>,
        /// Phase 3: TypeResolution вместо String для полной информации о типе объекта
        object_type: Option<TypeResolution>,
        /// Phase 3: TypeResolution вместо String для полной информации о типах аргументов
        arg_types: Vec<TypeResolution>,
        /// Индекс вложенного узла (для цепочек методов)
        /// Например: Справочники.Контрагенты.НайтиПоКоду().ПолучитьОбъект()
        /// ПолучитьОбъект будет иметь object_node указывающий на НайтиПоКоду
        object_node: Option<usize>,
        /// НОВОЕ: Тип возвращаемого значения функции/метода
        ///
        /// # Примеры
        /// - `Справочники.Контрагенты.НайтиПоКоду("001")` → result_type: СправочникСсылка.Контрагенты
        /// - `Массив.Добавить("x")` → result_type: Неопределено (процедура)
        /// - `Строка(123)` → result_type: Строка
        result_type: TypeResolution,
    },

    // === Scope tracking ===
    /// Блок scope
    BlockScope {
        statements: Vec<usize>, // Индексы SemanticNode
        scope_id: ScopeId,
    },

    // === Конструкторы (Constructor Support) ===
    /// Выражение конструктора: `Новый Массив`, `Новый Массив(10)`, `Новый("Строка")`
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// // Конструктор без параметров
    /// МассивДанных = Новый Массив;
    ///
    /// // Конструктор с параметрами
    /// МассивФиксированный = Новый Массив(10);
    ///
    /// // Динамический конструктор через строку
    /// Ссылка = Новый("СправочникСсылка.Номенклатура");
    /// ```
    ///
    /// # Семантика
    ///
    /// - `type_name` - имя типа для создания ("Массив", "ТаблицаЗначений", etc.)
    /// - `arg_types` - типы аргументов конструктора (для валидации и inference)
    /// - `is_dynamic` - `true` для динамических конструкторов `Новый("Тип")`
    /// - `result_type` - результирующий тип (обычно равен type_name, для Generic может быть "Массив<T>")
    /// - `generic_params` - параметры Generic типов для коллекций (None если не Generic)
    NewExpression {
        /// Имя типа для создания ("Массив", "ТаблицаЗначений", "СправочникСсылка.Номенклатура")
        type_name: String,

        /// Phase 3: TypeResolution для результирующего типа конструктора
        ///
        /// Обычно равен `type_name`, но для Generic коллекций может включать параметры:
        /// - `Массив` → TypeResolution::generic("Массив", ["?"])
        /// - `Массив<Число>` → TypeResolution::generic("Массив", ["Число"])
        result_type: TypeResolution,

        /// Phase 3: TypeResolution для типов аргументов конструктора
        ///
        /// # Примеры
        /// - `Новый Массив(10)` → `vec![TypeResolution::primitive("Число")]`
        /// - `Новый ТаблицаЗначений` → `vec![]` (без аргументов)
        arg_types: Vec<TypeResolution>,

        /// Generic параметры для коллекций (None если тип не Generic)
        ///
        /// # Примеры
        /// - `Массив` → None (inference будет выполнен позже из использования)
        /// - `Массив<Число>` → Some(vec!["Число"]) (явный параметр)
        /// - `Соответствие<Строка, Число>` → Some(vec!["Строка", "Число"])
        generic_params: Option<Vec<String>>,

        /// Динамический конструктор через строку
        ///
        /// `true` для выражений вида `Новый("СправочникСсылка.Номенклатура")`
        is_dynamic: bool,
    },
}

/// Параметр функции/процедуры
///
/// # Phase 3: TypeResolution для type_hint
///
/// `type_hint` теперь содержит полную информацию о типе параметра:
/// - Explicit если явно указан в коде (`Параметр: Строка`)
/// - None если тип не указан (динамический параметр)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    /// Phase 3: TypeResolution вместо String для типа параметра
    pub type_hint: Option<TypeResolution>,
    pub default_value: Option<String>,
    pub is_val: bool, // ByVal параметр
}

/// Сигнатура функции/процедуры
///
/// # Phase 3: TypeResolution для return_type
///
/// `return_type` теперь содержит полную информацию о возвращаемом типе:
/// - Explicit если явно указан в коде
/// - Inferred если выведен из анализа тела функции
/// - None для процедур (не возвращают значение)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<Parameter>,
    /// Phase 3: TypeResolution вместо String для возвращаемого типа
    pub return_type: Option<TypeResolution>,
    pub is_export: bool,
}

/// Состояние переменной в таблице символов
///
/// Содержит полную информацию о переменной: тип, позицию и флаг инициализации.
/// Используется для отслеживания flow-sensitive состояния переменных.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableState {
    /// Тип переменной (TypeResolution с certainty и metadata)
    pub resolution: TypeResolution,
    /// Инициализирована ли переменная (присвоено значение)
    pub initialized: bool,
    /// Позиция объявления в исходном коде
    pub declaration_span: Span,
}

impl VariableState {
    /// Создать состояние переменной
    pub fn new(resolution: TypeResolution, span: Span, initialized: bool) -> Self {
        Self {
            resolution,
            initialized,
            declaration_span: span,
        }
    }

    /// Создать для объявления без инициализации (Перем X;)
    pub fn declared(resolution: TypeResolution, span: Span) -> Self {
        Self::new(resolution, span, false)
    }

    /// Создать для объявления с инициализацией (X = 5; или параметр функции)
    pub fn initialized(resolution: TypeResolution, span: Span) -> Self {
        Self::new(resolution, span, true)
    }

    /// Пометить как инициализированную
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }
}
