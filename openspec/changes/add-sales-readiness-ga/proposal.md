# Change: Sales-readiness baseline для коммерческого GA

## Why
Продукт уже функционально силен для инженерного использования, но для стабильных продаж не хватает формального коммерческого контура: согласованных настроек расширения и документации, воспроизводимого release-пайплайна, а также минимального trust/legal пакета для enterprise-покупателей.

Сейчас это создает риски:
- покупатель не находит/не понимает Runtime Overrides в VS Code settings;
- документация и фактические ключи/поведение расходятся;
- релизы и артефакты поставки недостаточно формализованы для закупки и security-review.

## What Changes
- Добавить новую capability `sales-readiness` с требованиями к:
  - обязательному коммерческому пакету документов (EULA/Privacy/Support/Security),
  - воспроизводимому onboarding и GA-checklist,
  - release integrity артефактам (checksums + SBOM).
- Добавить дельту в `bsl-runtime-config`:
  - Runtime Overrides MUST быть явно доступны в UI настроек VS Code extension;
  - документация MUST быть синхронизирована с фактической схемой настроек.
- Добавить дельту в `dev-workflow`:
  - tag-driven release pipeline для VS Code extension;
  - обязательные release-проверки консистентности docs/settings.

## Impact
- Affected specs:
  - `sales-readiness` (new)
  - `bsl-runtime-config` (modified via delta)
  - `dev-workflow` (modified via delta)
- Affected code/docs (implementation follow-up):
  - `vscode-extension/package.json` (contributes.configuration)
  - `vscode-extension/README.md`
  - `.github/workflows/*` (release/check workflows)
  - legal/security docs в корне репозитория

## Non-Goals
- Реализация самой TPM-lease криптографии и протоколов (это покрывается `add-tpm-lease-licensing`).
- Изменение функциональной семантики IntelliSense/LSP поверх уже согласованных v2 change.
- Полная автоматизация enterprise procurement-процессов вне репозитория (договоры, биллинг, CRM).
