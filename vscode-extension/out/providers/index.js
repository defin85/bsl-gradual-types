"use strict";
/**
 * Экспорт всех провайдеров
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.BslActionsWebviewProvider = exports.HierarchicalTypeIndexProvider = exports.BslTypeIndexProvider = exports.BslPlatformDocsProvider = exports.BslDiagnosticsProvider = exports.BslOverviewProvider = exports.PlatformDocItem = exports.BslTypeItem = exports.BslDiagnosticItem = exports.BslOverviewItem = void 0;
// Tree Item классы
var items_1 = require("./items");
Object.defineProperty(exports, "BslOverviewItem", { enumerable: true, get: function () { return items_1.BslOverviewItem; } });
Object.defineProperty(exports, "BslDiagnosticItem", { enumerable: true, get: function () { return items_1.BslDiagnosticItem; } });
Object.defineProperty(exports, "BslTypeItem", { enumerable: true, get: function () { return items_1.BslTypeItem; } });
Object.defineProperty(exports, "PlatformDocItem", { enumerable: true, get: function () { return items_1.PlatformDocItem; } });
// Провайдеры для sidebar
var overviewProvider_1 = require("./overviewProvider");
Object.defineProperty(exports, "BslOverviewProvider", { enumerable: true, get: function () { return overviewProvider_1.BslOverviewProvider; } });
var diagnosticsProvider_1 = require("./diagnosticsProvider");
Object.defineProperty(exports, "BslDiagnosticsProvider", { enumerable: true, get: function () { return diagnosticsProvider_1.BslDiagnosticsProvider; } });
var platformDocs_1 = require("./platformDocs");
Object.defineProperty(exports, "BslPlatformDocsProvider", { enumerable: true, get: function () { return platformDocs_1.BslPlatformDocsProvider; } });
var typeIndexProvider_1 = require("./typeIndexProvider");
Object.defineProperty(exports, "BslTypeIndexProvider", { enumerable: true, get: function () { return typeIndexProvider_1.BslTypeIndexProvider; } });
var hierarchicalTypeProvider_1 = require("./hierarchicalTypeProvider");
Object.defineProperty(exports, "HierarchicalTypeIndexProvider", { enumerable: true, get: function () { return hierarchicalTypeProvider_1.HierarchicalTypeIndexProvider; } });
var actionsWebview_1 = require("./actionsWebview");
Object.defineProperty(exports, "BslActionsWebviewProvider", { enumerable: true, get: function () { return actionsWebview_1.BslActionsWebviewProvider; } });
//# sourceMappingURL=index.js.map