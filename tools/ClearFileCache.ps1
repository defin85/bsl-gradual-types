# ClearFileCache.ps1
# PowerShell скрипт для очистки Windows File System Cache
# Альтернатива EmptyStandbyList.exe (не требует внешних утилит)
#
# Требует запуска от имени администратора!

param(
    [switch]$Force
)

# Проверка прав администратора
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "❌ Требуются права администратора!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Запусти скрипт от имени администратора:" -ForegroundColor Yellow
    Write-Host "   powershell -ExecutionPolicy Bypass -File tools\ClearFileCache.ps1" -ForegroundColor Cyan
    exit 1
}

Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "🧹 Очистка Windows File System Cache" -ForegroundColor White
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Импортируем WinAPI функции
Add-Type @"
using System;
using System.Runtime.InteropServices;

public class MemoryManagement
{
    [DllImport("psapi.dll")]
    public static extern int EmptyWorkingSet(IntPtr hwProc);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool SetSystemFileCacheSize(
        IntPtr MinimumFileCacheSize,
        IntPtr MaximumFileCacheSize,
        int Flags
    );

    public const int FILE_CACHE_MAX_HARD_ENABLE = 0x00000001;
    public const int FILE_CACHE_MIN_HARD_ENABLE = 0x00000004;
}
"@

try {
    Write-Host "🔄 Очистка File System Cache..." -ForegroundColor Yellow

    # Устанавливаем минимальный кэш (эффективно очищая его)
    $result = [MemoryManagement]::SetSystemFileCacheSize(
        [IntPtr]::new(0x80000),    # 512 KB минимум
        [IntPtr]::new(0x100000),   # 1 MB максимум
        [MemoryManagement]::FILE_CACHE_MAX_HARD_ENABLE -bor [MemoryManagement]::FILE_CACHE_MIN_HARD_ENABLE
    )

    if ($result) {
        Write-Host "   Кэш уменьшен до минимума..." -ForegroundColor Gray
    } else {
        Write-Host "   ⚠️ Не удалось уменьшить кэш (ошибка WinAPI)" -ForegroundColor Yellow
    }

    # Даём системе время для освобождения памяти
    Start-Sleep -Milliseconds 1000

    # Возвращаем управление кэшем системе
    $result = [MemoryManagement]::SetSystemFileCacheSize(
        [IntPtr]::new(-1),
        [IntPtr]::new(-1),
        0
    )

    if ($result) {
        Write-Host "   Управление кэшем возвращено системе" -ForegroundColor Gray
    }

    Write-Host ""
    Write-Host "✅ File System Cache очищен!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📊 Теперь можешь тестировать прогресс парсинга:" -ForegroundColor Cyan
    Write-Host "   1. Открой VSCode Extension (F5)" -ForegroundColor White
    Write-Host "   2. Открой конфигурацию 1С" -ForegroundColor White
    Write-Host "   3. Наблюдай прогресс в статус-баре (1-2 минуты)" -ForegroundColor White
    Write-Host ""

    # Дополнительно: очистка Working Sets (опционально)
    if ($Force) {
        Write-Host "🔄 Очистка Working Sets всех процессов (может занять время)..." -ForegroundColor Yellow
        Get-Process | ForEach-Object {
            try {
                $null = [MemoryManagement]::EmptyWorkingSet($_.Handle)
            } catch {
                # Игнорируем процессы, к которым нет доступа
            }
        }
        Write-Host "✅ Working Sets очищены" -ForegroundColor Green
        Write-Host ""
    }

} catch {
    Write-Host ""
    Write-Host "❌ Ошибка при очистке кэша:" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ""
    exit 1
}

Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
