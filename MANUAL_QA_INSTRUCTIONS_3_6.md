# Manual QA Instructions for MILESTONE 3.6 Phase 1
## Settings & Detail Levels for Hover

**Duration:** ~15-20 minutes
**Requirements:** VSCode with BSL Extension installed

---

## SETUP

### 1. Compile and Install Extension

```bash
# Build the extension
cd vscode-extension
npm run compile

# In VSCode: Press F5 to open Extension Development Host
# (or: Run → Start Debugging)
```

### 2. Open Test File

Create a test file `test_hover_detail_levels.bsl`:

```bsl
Процедура ТестОсновныхЛевелов()
    Перем МассивДанных;
    Перем ТаблицаЗначений;
    Перем СтроковыеДанные;

    МассивДанных = Новый Массив;
    ТаблицаЗначений = Новый ТаблицаЗначений;
    СтроковыеДанные = "Строка";
КонецПроцедуры
```

---

## TEST SCENARIOS

### SCENARIO 1: Compact Detail Level

**Goal:** Verify that Compact level shows only type information

#### Steps:
1. **Open Settings:** Ctrl+, (or Menu → File → Preferences → Settings)
2. **Search:** Type "BSL Hover Detail"
3. **Set:** Change `detailLevel` to `compact`
4. **Open test file:** Load `test_hover_detail_levels.bsl`
5. **Hover:** Position cursor on variable `МассивДанных` (any occurrence)
6. **Observe hover tooltip**

#### Expected Result:
```
✅ Shows: Type information (Массив)
✅ Shows: Certainty indicator (🟢 Known or 🟡 Inferred)
❌ Should NOT show: Method list (Методы section)
❌ Should NOT show: Property list (Свойства section)

Example hover:
┌─────────────────────────────────┐
│ Переменная: МассивДанных        │
│ Тип: Массив                     │
│ Уверенность: 🟢 Known (100%)    │
└─────────────────────────────────┘
```

#### Pass/Fail:
- **PASS** if hover shows only type + certainty
- **FAIL** if hover shows methods/properties in Compact mode

---

### SCENARIO 2: Full Detail Level (Default)

**Goal:** Verify Full level shows type + methods (up to limit)

#### Steps:
1. **Set:** Change `detailLevel` to `full` in Settings
2. **Set:** Verify `maxMethods` = 10 (default)
3. **Hover:** Same variable `МассивДанных`
4. **Observe:** Tooltip should change without reloading

#### Expected Result:
```
✅ Shows: Type (Массив)
✅ Shows: Certainty (🟢 Known)
✅ Shows: Methods section with up to 10 methods
❌ Should NOT show: Properties section

Example:
┌─────────────────────────────────────┐
│ Переменная: МассивДанных            │
│ Тип: Массив                         │
│ Уверенность: 🟢 Known (100%)        │
│                                     │
│ Методы (показано 10 из 20):         │
│ • Добавить(Значение, Позиция)       │
│ • Вставить(Позиция, Значение)       │
│ • Удалить(Позиция)                  │
│ • Получить(Позиция)                 │
│ ... и ещё 10 методов                │
└─────────────────────────────────────┘
```

#### Pass/Fail:
- **PASS** if methods shown up to limit, no properties
- **FAIL** if properties shown, or methods not shown

---

### SCENARIO 3: Detailed Level

**Goal:** Verify Detailed shows everything (methods + properties)

#### Steps:
1. **Set:** Change `detailLevel` to `detailed`
2. **Hover:** Same variable or use `ТаблицаЗначений` (has more properties)
3. **Observe:** Tooltip updates

#### Expected Result:
```
✅ Shows: Type
✅ Shows: Certainty
✅ Shows: Methods (up to maxMethods = 10)
✅ Shows: Properties (up to maxProperties = 5)

Example:
┌──────────────────────────────────────┐
│ Переменная: ТаблицаЗначений          │
│ Тип: ТаблицаЗначений                 │
│ Уверенность: 🟢 Known (100%)         │
│                                      │
│ Методы (показано 10 из 30):          │
│ • Добавить(ТипМассива)               │
│ • Вставить(Индекс, ТипМассива)       │
│ ... и ещё 20 методов                 │
│                                      │
│ Свойства (показано 5 из 8):          │
│ • Количество: Число                  │
│ • Колонки: КолекцияКолонок           │
│ ... и ещё 3 свойства                 │
└──────────────────────────────────────┘
```

#### Pass/Fail:
- **PASS** if both methods and properties shown
- **FAIL** if properties not shown, or limits not respected

---

### SCENARIO 4: Certainty Toggle

**Goal:** Verify showCertainty flag works correctly

#### Steps:
1. **Set:** `showCertainty` = `true` (default)
2. **Hover:** Any variable
3. **Observe:** Certainty indicator visible
4. **Set:** Change `showCertainty` to `false`
5. **Hover:** Same variable
6. **Observe:** Certainty line should disappear

#### Expected Result:
```
WITH showCertainty = true:
┌─────────────────────────────────┐
│ Переменная: МассивДанных        │
│ Тип: Массив                     │
│ Уверенность: 🟢 Known (100%)    │ ← VISIBLE
└─────────────────────────────────┘

WITH showCertainty = false:
┌─────────────────────────────────┐
│ Переменная: МассивДанных        │
│ Тип: Массив                     │
│ (no certainty line)             │ ← HIDDEN
└─────────────────────────────────┘
```

#### Pass/Fail:
- **PASS** if line appears/disappears correctly
- **FAIL** if certainty always shown or always hidden

---

### SCENARIO 5: Dynamic Configuration Updates

**Goal:** Verify hover updates without LSP restart

#### Steps:
1. **Open:** test_hover_detail_levels.bsl
2. **Hover:** On `МассивДанных` → note current detail level
3. **Change Setting:** In Settings, change `detailLevel` from "full" to "compact"
4. **DO NOT close file or restart**
5. **Hover:** Same variable again

#### Expected Result:
```
✅ Hover updates immediately (no LSP restart needed)
✅ Compact version shown without page reload
✅ All settings changes take effect instantly
```

#### Pass/Fail:
- **PASS** if hover changes without reopening file
- **FAIL** if needs page reload or LSP restart

---

### SCENARIO 6: Method Limits (maxMethods)

**Goal:** Verify method limit is enforced

#### Steps:
1. **Set:** `detailLevel` = "full"
2. **Set:** `maxMethods` = 3
3. **Set:** `maxProperties` = 2
4. **Hover:** On type with many methods
5. **Observe:** Only 3 methods shown + remainder message

#### Expected Result:
```
✅ Shows exactly 3 methods
✅ Shows message: "... и ещё N методов"
✅ Does not exceed limit

Example:
│ Методы (показано 3 из 20):
│ • Добавить(...)
│ • Вставить(...)
│ • Удалить(...)
│ ... и ещё 17 методов
```

#### Pass/Fail:
- **PASS** if limit respected and remainder shown
- **FAIL** if limit ignored or message missing

---

### SCENARIO 7: Multiline Method Formatting

**Goal:** Verify methods with 4+ parameters use multiline format

#### Steps:
1. **Set:** `detailLevel` = "full"
2. **Find:** Method with 4+ parameters
   - Best option: `ТаблицаЗначений.Вставить()` (has ~4 params)
3. **Hover:** On that method call location
4. **Observe:** Parameter formatting

#### Expected Result:
```
For method with 3 parameters (inline):
• ВставитьКолонку(Колонка: Строка, Позиция: Число, Ширина: Число = 0) → Число

For method with 4+ parameters (multiline):
• ВставитьДанные(
    Строка: Число,
    Колонка: Строка,
    Позиция: Число,
    Значение: Значение
  ) → Неопределено
```

#### Pass/Fail:
- **PASS** if 4+ param methods use multiline with indentation
- **FAIL** if all methods inline or indentation wrong

---

### SCENARIO 8: Default Fallback for Invalid Values

**Goal:** Verify system handles invalid settings gracefully

#### Steps:
1. **Edit** settings.json directly (advanced):
   ```json
   "bsl.hover": {
     "detailLevel": "INVALID_VALUE"
   }
   ```
2. **Hover:** On any variable
3. **Observe:** System should fall back to "full"

#### Expected Result:
```
✅ No errors in LSP logs
✅ Hover works with default (full) level
✅ No red errors in VSCode
```

#### Pass/Fail:
- **PASS** if fallback works silently
- **FAIL** if error messages appear

---

## REGRESSION TESTS

### Check That Nothing Broke

#### Test R1: Basic Hover Still Works
- [ ] Hover on primitive types (Число, Строка) shows type
- [ ] Hover on custom types shows appropriate detail level
- [ ] No exceptions in Developer Console (Help → Toggle Developer Tools)

#### Test R2: Method Signatures Preserved
- [ ] Method parameter names visible
- [ ] Return types shown
- [ ] Optional parameters marked with "?"
- [ ] Default values shown when present

#### Test R3: Performance Acceptable
- [ ] Hover appears within 100ms
- [ ] No noticeable lag when changing detail level
- [ ] Extension doesn't consume excess CPU while idle

---

## BUG REPORT TEMPLATE

If you find an issue, please report:

```
Title: [MILESTONE 3.6] Brief description

Severity: Critical | High | Medium | Low

Reproduction Steps:
1. Set detailLevel = "..."
2. Hover on variable/method "..."
3. Observe: (what you see)

Expected: (what should happen)

Actual: (what happens instead)

LSP Log: (paste relevant lines from Output → BSL Language Server)

Attachments: (screenshot if visual)
```

---

## TESTING COMPLETE

When all scenarios pass:
- [ ] SCENARIO 1: Compact level ✅
- [ ] SCENARIO 2: Full level ✅
- [ ] SCENARIO 3: Detailed level ✅
- [ ] SCENARIO 4: Certainty toggle ✅
- [ ] SCENARIO 5: Dynamic updates ✅
- [ ] SCENARIO 6: Method limits ✅
- [ ] SCENARIO 7: Multiline formatting ✅
- [ ] SCENARIO 8: Invalid values ✅
- [ ] REGRESSION: No breakage ✅

**Sign off:** Phase 1 Manual QA Complete ✅

---

## SUPPORT COMMANDS

### View LSP Logs
1. Help → Toggle Developer Tools
2. Output tab → Select "BSL Language Server" from dropdown
3. Scroll to see detailed logging

### Check Settings
```json
// .vscode/settings.json should contain:
{
  "bsl.hover": {
    "detailLevel": "full",        // compact | full | detailed
    "maxMethods": 10,
    "maxProperties": 5,
    "showCertainty": true
  }
}
```

### Force LSP Restart
- Command Palette (Ctrl+Shift+P)
- Type: "BSL: Restart Language Server"
- Enter

---

**END OF MANUAL QA INSTRUCTIONS**
