/**
 * Вспомогательный класс для работы с конфигурацией BSL Analyzer
 * Использует плоскую структуру настроек, организованную в категории
 */
export declare class BslAnalyzerConfig {
    private static getConfig;
    static get enabled(): boolean;
    static get enableRealTimeAnalysis(): boolean;
    static get maxFileSize(): number;
    static get serverMode(): string;
    static get serverTcpPort(): number;
    static get serverTrace(): string;
    static get useBundledBinaries(): boolean;
    static get binaryPath(): string;
    static get configurationPath(): string;
    static get platformVersion(): string;
    static get platformDocsArchive(): string;
    static get autoIndexBuild(): boolean;
    static get rulesConfig(): string;
    static get enableMetrics(): boolean;
    static isValid(): boolean;
    static summary(): any;
}
/**
 * Мигрирует старые настройки на новые имена
 */
export declare function migrateLegacySettings(): Promise<void>;
//# sourceMappingURL=configHelper.d.ts.map