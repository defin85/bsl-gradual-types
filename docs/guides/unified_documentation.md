# Unified Documentation for BSL Gradual Type System

> Внимание: этот документ служит агрегатором ссылок. Единственным актуальным источником является раздел [reference/target_architecture](../reference/target_architecture/). См. его README и связанные документы.

## Введение
Проект BSL Gradual Type System направлен на создание системы постепенной типизации для языка 1С:Предприятие. Цель - улучшение качества кода, интеграция с LSP, поддержка различных типов (платформенных, конфигурационных, пользовательских), с акцентом на производительность, расширяемость и двуязычность.

## Архитектура
См. раздел [target_architecture](../reference/target_architecture/) — обзор слоёв и компонентов в актуальной архитектуре.

## Дизайн-решения
Ключевые решения и контракты отражены в документах раздела [target_architecture](../reference/target_architecture/), например [central_type_system.md](../reference/target_architecture/central_type_system.md) и [overview.md](../reference/target_architecture/overview.md).

## Фасетная система
Описание и контракты представлены в [central_type_system.md](../reference/target_architecture/central_type_system.md) и смежных документах раздела [target_architecture](../reference/target_architecture/).

## Реализация
Актуальные детали реализации отражены в разделе [target_architecture](../reference/target_architecture/).

## Целевая Архитектура
На основе target_architecture (источник истины для системы):
- Слоистая архитектура: Data, Domain, Application, Presentation.
- Центральная система типов на Rust.
- План миграции с Strangler-pattern и фича-флагами.
- Дорожная карта с 6 фазами внедрения.

## План Миграции
См. [migration_plan.md](../reference/target_architecture/migration_plan.md): поэтапный переход на Tree-sitter, меры по снижению рисков.