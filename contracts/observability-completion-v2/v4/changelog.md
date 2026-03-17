# observability-completion-v2 v4

## 4.0.0

- Adds bounded completion route counters for split-prepare routing:
  - `head_hit`
  - `exact_hit`
- Adds bounded completion fail-closed cause counters for split-prepare attribution:
  - `prepare_timeout`
  - `exact_deadline`
- Adds `completion_head_to_exact_upgrade` counter+histogram so dashboards can distinguish first response from later exact readiness on the same revision.

Migration note: dashboards and tooling that want authoritative split-prepare attribution
must switch from `v3` to `v4` and read the new bounded route/cause/upgrade keys instead of
inferring them from generic completion outcomes alone.
