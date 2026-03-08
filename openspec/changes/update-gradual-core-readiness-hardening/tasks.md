## 1. Contract
- [ ] 1.1 Зафиксировать, что readiness gate MUST выводить или fail-closed сверять review/traceability verdict по содержимому `review_ref` и `traceability_ref`, а не доверять только self-reported JSON полям.
- [ ] 1.2 Зафиксировать policy для `superseding_delivery_path`: когда он может легитимно разрешить `declared_status=complete`, и какие доказательства для этого обязательны.
- [ ] 1.3 Зафиксировать exit criteria для bootstrap-only implicit module-context fallback в completion member resolution.

## 2. Validation First
- [ ] 2.1 Добавить failing regression tests для mismatch между `readiness_status.json` и содержимым `review_ref`.
- [ ] 2.2 Добавить failing regression tests для mismatch между `readiness_status.json` и содержимым `traceability_ref`.
- [ ] 2.3 Добавить regression tests для positive/negative сценариев `superseding_delivery_path`.
- [ ] 2.4 Добавить failing tests, которые pin down границу bootstrap-only implicit module-context fallback.

## 3. Implementation
- [ ] 3.1 Ужесточить `scripts/check-openspec-change-governance.py` так, чтобы optimistic verdict валился fail-closed при конфликте declared status и referenced evidence.
- [ ] 3.2 Доставить runtime/test changes для bootstrap-only implicit module-context fallback: либо direct automated evidence bounded behaviour, либо removal of the fallback path.

## 4. Closure
- [ ] 4.1 Обновить `openspec/changes/update-gradual-core-production-readiness/**` артефакты так, чтобы они ссылались на hardened gate semantics и не опирались на слабую self-reported модель.
- [ ] 4.2 Прогнать `openspec validate update-gradual-core-readiness-hardening --strict --no-interactive`.

## Dependencies / Parallelism
- [ ] D1 Пункты 1.1 и 1.2 блокируют 2.1, 2.2 и 2.3.
- [ ] D2 Пункт 1.3 блокирует 2.4 и 3.2.
- [ ] D3 Пункты 2.1, 2.2 и 2.3 блокируют 3.1.
- [ ] D4 Пункты 3.1 и 3.2 блокируют 4.1 и 4.2.
