# isolate-completion-pre-dispatch-ingress real-module readiness gate

- profile: `p39_real_conf_big_document_symbol_mixed_load_gate_live`
- profile title: `documentSymbol mixed-load isolation`
- report: `/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-real-conf-big-document-symbol-mixed-load-live.json`
- report change_id: `isolate-completion-pre-dispatch-ingress`
- measured samples: `10`
- head_hit traces: `10`
- exact_hit traces: `0`
- prepare_timeout delta: `0`
- exact_deadline delta: `0`
- documentSymbol latest_ready delta: `40`
- documentSymbol current_ready delta: `0`
- documentSymbol unavailable delta: `0`
- documentSymbol superseded delta: `0`
- documentSymbol present responses: `40`
- documentSymbol null responses: `0`
- legacy ingress-regression samples: `0`
- pre-dispatch samples over budget: `0`
- pre-dispatch samples over hard cap: `0`
`p95(adapter_to_dispatch_wait_ms)=1ms`
`max(adapter_to_dispatch_wait_ms)=1ms`
`p95(service_future_to_first_poll_wait_ms)=3ms`
`max(service_future_to_first_poll_wait_ms)=3ms`
`p95(transport_to_handler_wait_ms)=3ms`
`max(transport_to_handler_wait_ms)=3ms`
