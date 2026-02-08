## 1. Specification
- [ ] 1.1 Утвердить capability `tpm-lease-licensing` как источник истины для UX, протокола и политики истечения.
- [ ] 1.2 Зафиксировать формат лицензии, требования к подписи и binding к TPM-публичному ключу.
- [ ] 1.3 Зафиксировать поведение offline/expiry: `30 дней lease + 7 дней grace + hard-fail`.
- [ ] 1.4 Зафиксировать обязательные CLI-операции: `license status`, `license activate`, `license renew`, `license diag`.
- [ ] 1.5 Зафиксировать поддерживаемый VM-сценарий: passthrough TPM (enterprise).
- [ ] 1.6 Зафиксировать процессы re-host и ротации вендорских ключей подписи.

## 2. Design And Security
- [ ] 2.1 Подготовить ADR по threat model: что защищается, какие атаки считаются out-of-scope.
- [ ] 2.2 Описать протокол renew с anti-replay (challenge, TTL, одноразовость) и правилами ошибок.
- [ ] 2.3 Описать политику логирования/диагностики без данных 1С и без чувствительных секретов.

## 3. Validation
- [ ] 3.1 `openspec validate add-tpm-lease-licensing --strict --no-interactive`.
- [ ] 3.2 Провести review спецификации с владельцами продукта и support (истечение, re-host, VM policy).
