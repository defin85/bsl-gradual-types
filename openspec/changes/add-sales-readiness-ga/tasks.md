## 1. Specification
- [ ] 1.1 Добавить новую capability `sales-readiness` с нормативными требованиями к коммерческому пакету, onboarding и GA-checklist.
- [ ] 1.2 Зафиксировать в `bsl-runtime-config`, что Runtime Overrides обязаны быть discoverable в UI настроек VS Code extension.
- [ ] 1.3 Зафиксировать в `bsl-runtime-config` синхронизацию ключей/описаний между `package.json` и документацией.
- [ ] 1.4 Зафиксировать в `dev-workflow` tag-driven release pipeline для VSIX с integrity артефактами.
- [ ] 1.5 Зафиксировать в `dev-workflow` обязательные release-checks на консистентность docs/settings.

## 2. Design And Rollout
- [ ] 2.1 Утвердить минимальный enterprise trust/legal пакет: `EULA`, `PRIVACY`, `SUPPORT`, `SECURITY`.
- [ ] 2.2 Определить миграционную стратегию настроек расширения (legacy aliases vs canonical runtime keys).
- [ ] 2.3 Зафиксировать ownership по релизному чеклисту (Engineering + Product + Support).

## 3. Validation
- [ ] 3.1 `openspec validate add-sales-readiness-ga --strict --no-interactive`.
- [ ] 3.2 Провести review change с владельцами продукта (go-to-market), extension и runtime-config.
- [ ] 3.3 Подтвердить, что change согласован с активным `add-tpm-lease-licensing` (политика hard-fail/lease/grace).
