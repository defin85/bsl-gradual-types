# BSL Analyzer Binaries

This directory contains essential binaries for BSL Analyzer VSCode extension.

## Included tools:

- **bsl-analyzer.exe** - Main static analyzer
- **lsp_server.exe** - Language Server Protocol implementation  
- **syntaxcheck.exe** - Syntax validator
- **build_unified_index.exe** - Type system index builder
- **query_type.exe** - Type information queries
- **check_type_compatibility.exe** - Type compatibility checker
- **extract_platform_docs.exe** - Platform documentation extractor
- **extract_hybrid_docs.exe** - Hybrid documentation processor
- **incremental_update.exe** - Incremental analysis updates
- **bsl-mcp-server.exe** - MCP server for LLM integration

Total size optimized from 155+ MB to ~63.4 MB by excluding test and debug binaries.
