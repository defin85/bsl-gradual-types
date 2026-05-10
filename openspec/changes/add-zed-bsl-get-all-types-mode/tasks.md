## 1. Approval and baseline
- [x] 1.1 Get approval for this OpenSpec change before production implementation
- [x] 1.2 Confirm `add-zed-symbol-browser-panel` baseline exists in the working Zed fork
- [x] 1.3 Confirm the active BSL server advertises `bsl.getAllTypes` through `execute_command_provider`

## 2. Backend contract
- [x] 2.1 Add focused backend coverage for `workspace/executeCommand` command `bsl.getAllTypes` pagination and response shape
- [x] 2.2 Keep direct `bsl/getAllTypes` alias deferred unless explicitly approved; if implemented, add backend and Zed tests proving identical response semantics
- [x] 2.3 Document unsupported/empty-domain behavior for getAllTypes as an empty, well-formed `AnalysisResultDto`
- [x] 2.4 Add coverage that initialization advertises `bsl.getAllTypes` through `execute_command_provider.commands`
- [x] 2.5 Document that item grouping consumers must use each returned type's `source` and `category`; response-level `categories` is advisory metadata

## 3. Zed project/LSP data source
- [x] 3.1 Add typed DTOs for BSL getAllTypes request/response in the Zed fork
- [x] 3.2 Add a narrow project-layer Zed helper to find every BSL-capable language server and fetch paged getAllTypes results
- [x] 3.3 Preserve generic `project.symbols("", cx)` as fallback for non-BSL or unsupported servers
- [x] 3.4 Define checked-in page-size, max-page, and max-item constants and test the cap path
- [x] 3.5 Add focused tests for BSL capability detection, multi-server deterministic merge, pagination stop conditions, and error propagation
- [x] 3.6 Ensure any page failure after BSL mode selection returns the BSL error state instead of falling back to generic symbols or merging stale partial data

## 4. Symbol Browser UI
- [x] 4.1 Add a data-source mode to `symbol_browser`: generic `workspace/symbol` vs BSL TypeRepository
- [x] 4.2 Render BSL groups by source/category with deterministic ordering and counts
- [x] 4.3 Keep loading/empty/error states honest for both modes
- [x] 4.4 Build BSL groups from returned item-level `source` and `category`, not from response-level `categories`
- [x] 4.5 Add focused tests for BSL grouping and fallback to generic grouping

## 5. Verification
- [x] 5.1 Run backend focused getAllTypes tests
- [x] 5.2 Run `cargo test -p project` focused Zed tests for the new helper
- [x] 5.3 Run `cargo test -p symbol_browser`
- [x] 5.4 Run `cargo check -p zed`
- [ ] 5.5 Run BSL X11 live-smoke with the dev extension installed: Symbol Browser shows TypeRepository source/category groups
- [ ] 5.6 Run Rust X11 live-smoke: generic `workspace/symbol` behavior remains intact
- [x] 5.7 Run `openspec validate add-zed-bsl-get-all-types-mode --strict --no-interactive`
