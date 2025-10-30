import * as vscode from 'vscode';
import { BslPlatformDocsProvider } from '../providers';
/**
 * Инициализирует модуль платформенной документации
 */
export declare function initializePlatformDocs(channel: vscode.OutputChannel): void;
/**
 * Добавляет платформенную документацию
 */
export declare function addPlatformDocumentation(provider: BslPlatformDocsProvider): Promise<void>;
/**
 * Удаляет платформенную документацию
 */
export declare function removePlatformDocumentation(version: string, provider: BslPlatformDocsProvider): Promise<void>;
/**
 * Перепарсит платформенную документацию
 */
export declare function parsePlatformDocumentation(version: string): Promise<void>;
//# sourceMappingURL=manager.d.ts.map