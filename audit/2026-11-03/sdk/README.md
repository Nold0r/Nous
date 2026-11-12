# Воспроизведение аудита SDK

Цель: `D:/Documents/ClodeCode/Nous-sdk`, `codex/sdk@9d49f65`. Эти тесты расположены в соседнем репозитории Nous и используют **оригинальные crates SDK как path dependencies**. В `audit_tests.rs` тонкий wrapper направляет GameCommand через `nous_hsr::decode` в публичный `nous_core::GameState`; он не исправляет и не дублирует предметную логику.

- [Отчёт, приоритеты, причины, рекомендации](REPORT.md).
- [Матрица сценариев и границы покрытия](SCENARIOS.md).
- [Исходники Rust-проверок](rust/src/audit_tests.rs), [специальные проверки SDK/Veritas](rust/src/sdk_specific.rs).
- [Результат Rust](results/rust.txt): **25 tests: 19 FAIL, 6 PASS**.
- [Результат hooks](results/frontend.json): **9 tests: 8 FAIL, 1 PASS**.
- [Результат module runtime / origin predicate](results/modules.json): **4 tests: 3 FAIL, 1 PASS**.

Всего **38 проверок аудита: 30 нарушений безопасного ожидаемого поведения, 8 положительных контролей**. Это не 30 независимых CVE и не 30 доказанных удалённых атак. Для воспроизведения тесты намеренно завершаются ненулевым кодом. Формат отчёта различает проверенный API, проверенный предикат и анализ кода.

Запуск из `D:/Documents/ClodeCode/Nous` (Node 24 использован для stripTypeScriptTypes; Rust — установленный stable):

```powershell
$env:NOUS_AUDIT_ROOT = 'D:\Documents\ClodeCode\Nous-sdk'
$env:NOUS_AUDIT_FRONTEND_RESULT = 'sdk/results/frontend.json'
node audit/2026-11-03/frontend-repros.mjs
node audit/2026-11-03/sdk/modules-repros.mjs
$env:CARGO_TARGET_DIR = 'D:\Documents\ClodeCode\Nous-sdk\target'
cargo test --manifest-path audit/2026-11-03/sdk/rust/Cargo.toml --offline --lib -- --test-threads=1
```

Rust создаёт только искусственные аккаунты/кеши в `sdk/results/rust-fixtures/<pid>-<case>`; эта папка исключена из git. Она оставлена для изучения. Ничего не удаляется рекурсивно из пользовательских каталогов. Fake network frames разбираются в памяти, не отправляются в сеть.

Штатные проверки, выполненные отдельно:

```powershell
# Из Nous-sdk
cargo check -p nous --offline
# NOUS_RECORDINGS для штатного replay был направлен в отдельную пустую папку.
cargo test -p nous-core -p nous-hsr -p nous-veritas --offline --lib --tests

# Из Nous-sdk/app
npm run lint
npm run build
npm run test:modules
npm audit --json
# При отдельном Vite dev-сервере на 127.0.0.1:5187:
$env:NOUS_UI_URL = 'http://127.0.0.1:5187/'
npm run test:ui
node --experimental-strip-types --test src/veritas/presentation.test.mjs src/veritas/updates.test.mjs src/veritas/browser.test.mjs scripts/tests/veritas-server.test.mjs
```

Результаты: `cargo check` успешен; Rust — **50 passed, 2 ignored**; modules — **3 passed**; smoke — успешен; Veritas JS/browser/server — **13 passed**; npm audit — **0**. Штатный replay с пустой папкой не означает проверку реальных login-записей; специальная синтетическая проверка пустого аккаунта в SDK при этом выполнялась.

`scan-deps.py` уровнем выше отправляет в OSV только публичные имена и версии registry-зависимостей. [Результат](results/rust-osv.json): 580 проверенных версий, 8 с advisories, из них часть informational/другой target. Команды не исправляют lock-файлы приложения.

Безопасный [PowerShell repro exit code](ci-exit-repro.ps1) запускает только два `cmd /c exit`, не создаёт tasks и не вызывает UAC. [Вывод](results/ci-exit.txt): первая ошибка 7, итоговый код 0. На этом ПК Windows PowerShell требует `-ExecutionPolicy Bypass` для выполнения локального файла; это параметр одного процесса, системная policy не менялась.

Первый широкий cargo-запуск дополнительно начал дорогую линковку example `events`; она была остановлена, затем проверка выполнена с `--lib --tests`. Её остановка не является результатом теста приложения. Первый запуск Veritas browser tests без NOUS_UI_URL получил connection refused на их стандартном 5174; с указанным изолированным сервером все 13 проверок прошли.

Программа не исправлялась, не запускалась поверх настоящей игры и не публиковалась. Тесты DLL, UAC, Task Scheduler, сетевой топологии и native replay перечислены как оставшиеся проверки, а не как пройденные.
