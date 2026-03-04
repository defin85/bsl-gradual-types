{
  "pass": false,
  "tokenizer": "o200k_base",
  "limits": {
    "max_production_loc": 1000,
    "max_target_loc": 800,
    "max_target_bytes": 81920,
    "max_target_tokens": 12000
  },
  "counts": {
    "production_files_scanned": 429,
    "target_files_expected": 28,
    "target_files_missing": 0,
    "hard_loc_violations": 1,
    "target_budget_violations": 15
  },
  "violations": {
    "hard_loc": [
      {
        "path": "backend/src/bin/lsp_server/server/language_server/impl_completion.rs",
        "loc": 1569,
        "max_loc": 1000
      }
    ],
    "target_missing": [],
    "target_budget": [
      {
        "path": "bsl-runtime/src/application/type_system/services/completion_service.rs",
        "loc": 872,
        "bytes": 28724,
        "tokens": 5905,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-runtime/src/system/basic_observability.rs",
        "loc": 980,
        "bytes": 32473,
        "tokens": 7876,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "analysis-v2/src/type_inference_v2.rs",
        "loc": 881,
        "bytes": 33173,
        "tokens": 6555,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-runtime/src/system/system_coordinator/config_loader.rs",
        "loc": 857,
        "bytes": 33909,
        "tokens": 6222,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-runtime/src/system/disk_cache.rs",
        "loc": 995,
        "bytes": 32595,
        "tokens": 6892,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-agent/src/server/mod.rs",
        "loc": 925,
        "bytes": 33089,
        "tokens": 7046,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "backend/src/bin/lsp_server/handlers/completion.rs",
        "loc": 818,
        "bytes": 27206,
        "tokens": 5581,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-runtime/src/system/runtime_config.rs",
        "loc": 990,
        "bytes": 34514,
        "tokens": 7532,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-runtime/src/system/parser_coordinator.rs",
        "loc": 961,
        "bytes": 34954,
        "tokens": 7006,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-runtime/src/system/system_coordinator/lifecycle.rs",
        "loc": 972,
        "bytes": 42807,
        "tokens": 7548,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-repository/src/repository.rs",
        "loc": 885,
        "bytes": 35495,
        "tokens": 7174,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "backend/src/bin/lsp_server/commands/configuration.rs",
        "loc": 837,
        "bytes": 28512,
        "tokens": 5367,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "backend/src/presentation/web/handlers.rs",
        "loc": 871,
        "bytes": 32532,
        "tokens": 6559,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "bsl-runtime/src/data/loaders/config_metadata_parser/discovery.rs",
        "loc": 844,
        "bytes": 39336,
        "tokens": 6928,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      },
      {
        "path": "backend/src/bin/intellisense_perf.rs",
        "loc": 926,
        "bytes": 29709,
        "tokens": 6689,
        "breaches": [
          "loc"
        ],
        "limits": {
          "max_loc": 800,
          "max_bytes": 81920,
          "max_tokens": 12000
        }
      }
    ]
  }
}
