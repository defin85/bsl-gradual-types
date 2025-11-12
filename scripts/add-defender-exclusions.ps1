# ============================================================================
# Скрипт для добавления BSL Gradual Types в исключения Windows Defender
# ============================================================================
# Требует прав администратора!
# Использование: .\scripts\add-defender-exclusions.ps1
# ============================================================================

# Проверка прав администратора
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "❌ Требуются права администратора!" -ForegroundColor Red
    Write-Host "Запустите PowerShell от имени администратора и повторите команду." -ForegroundColor Yellow
    exit 1
}

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "🛡️  BSL Gradual Types - Настройка Windows Defender" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

$projectRoot = "C:\1CProject\bsl-gradual-types"

# Папки для исключения
$excludedPaths = @(
    $projectRoot,
    "$projectRoot\examples\syntax_helper",
    "$projectRoot\target",
    "$projectRoot\vscode-extension",
    "$projectRoot\frontend\target"
)

# Процессы для исключения
$excludedProcesses = @(
    "$projectRoot\vscode-extension\bin\lsp-server.exe",
    "$projectRoot\target\release\bsl-lsp-server.exe",
    "$projectRoot\target\debug\bsl-lsp-server.exe"
)

# Добавляем исключения для папок
Write-Host "📁 Добавление папок в исключения:" -ForegroundColor Cyan
foreach ($path in $excludedPaths) {
    try {
        if (Test-Path $path) {
            Add-MpPreference -ExclusionPath $path -ErrorAction Stop
            Write-Host "  ✅ $path" -ForegroundColor Green
        } else {
            Write-Host "  ⏭️  $path (не существует)" -ForegroundColor Yellow
        }
    } catch {
        if ($_.Exception.Message -like "*already exists*") {
            Write-Host "  ⏭️  $path (уже в исключениях)" -ForegroundColor Yellow
        } else {
            Write-Host "  ❌ $path - ошибка: $($_.Exception.Message)" -ForegroundColor Red
        }
    }
}

Write-Host ""

# Добавляем исключения для процессов
Write-Host "⚙️  Добавление процессов в исключения:" -ForegroundColor Cyan
foreach ($process in $excludedProcesses) {
    try {
        if (Test-Path $process) {
            Add-MpPreference -ExclusionProcess $process -ErrorAction Stop
            Write-Host "  ✅ $process" -ForegroundColor Green
        } else {
            Write-Host "  ⏭️  $process (не существует)" -ForegroundColor Yellow
        }
    } catch {
        if ($_.Exception.Message -like "*already exists*") {
            Write-Host "  ⏭️  $process (уже в исключениях)" -ForegroundColor Yellow
        } else {
            Write-Host "  ❌ $process - ошибка: $($_.Exception.Message)" -ForegroundColor Red
        }
    }
}

Write-Host ""

# Отключаем сканирование архивов (опционально, снижает безопасность!)
Write-Host "📦 Настройки сканирования архивов:" -ForegroundColor Cyan
Write-Host "  Отключить сканирование ZIP архивов? (снижает безопасность!)" -ForegroundColor Yellow
$response = Read-Host "  Введите 'да' для отключения, Enter для пропуска"

if ($response -eq "да" -or $response -eq "yes") {
    try {
        Set-MpPreference -DisableArchiveScanning $true -ErrorAction Stop
        Write-Host "  ✅ Сканирование архивов отключено" -ForegroundColor Green
    } catch {
        Write-Host "  ❌ Ошибка: $($_.Exception.Message)" -ForegroundColor Red
    }
} else {
    Write-Host "  ⏭️  Пропущено (рекомендуется для безопасности)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan

# Показываем текущие исключения
Write-Host "📊 Текущие исключения Windows Defender:" -ForegroundColor Cyan
Write-Host ""

Write-Host "📁 Исключённые папки:" -ForegroundColor Cyan
$currentPaths = (Get-MpPreference).ExclusionPath
if ($currentPaths) {
    $currentPaths | Where-Object { $_ -like "*bsl-gradual-types*" } | ForEach-Object {
        Write-Host "  • $_" -ForegroundColor Green
    }
} else {
    Write-Host "  (нет исключений)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "⚙️  Исключённые процессы:" -ForegroundColor Cyan
$currentProcesses = (Get-MpPreference).ExclusionProcess
if ($currentProcesses) {
    $currentProcesses | Where-Object { $_ -like "*bsl*" -or $_ -like "*lsp*" } | ForEach-Object {
        Write-Host "  • $_" -ForegroundColor Green
    }
} else {
    Write-Host "  (нет исключений)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "📦 Сканирование архивов:" -ForegroundColor Cyan
$archiveScanning = (Get-MpPreference).DisableArchiveScanning
if ($archiveScanning) {
    Write-Host "  ⚠️  ОТКЛЮЧЕНО (снижена безопасность!)" -ForegroundColor Yellow
} else {
    Write-Host "  ✅ ВКЛЮЧЕНО (рекомендуется)" -ForegroundColor Green
}

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "✅ Настройка завершена!" -ForegroundColor Green
Write-Host ""
Write-Host "🚀 Следующие шаги:" -ForegroundColor Cyan
Write-Host "  1. Перезапустите VSCode для применения изменений"
Write-Host "  2. Запустите расширение (F5)"
Write-Host "  3. HBK Recovery должен работать быстрее без Defender"
Write-Host ""
