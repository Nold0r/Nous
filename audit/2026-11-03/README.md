# Аудит Nous — 2026-11-03

**Основной результат по выбранной пользователем версии — [Nous-sdk / nous-core, codex/sdk@9d49f65](sdk/README.md).** [Подробный отчёт](sdk/REPORT.md), [матрица сценариев](sdk/SCENARIOS.md).

Проверка первоначально началась в рабочем каталоге Nous, `main@114a8a8` (v0.1.10). После уточнения пользователя переключена на соседний worktree Nous-sdk. Файлы `rust/`, `results/` и исходный результат frontend в корне этого каталога относятся к **старой main**, а не SDK. Их нельзя смешивать с актуальным отчётом.

Предварительные main-only проверки: frontend hooks — 8 нарушений и 1 control, Rust — 17 нарушений и 2 control. Обычный cargo test main дал 16 passed / 1 failed / 3 ignored: replay ошибочно отвергал допустимые аккаунты без реликвий. При изолированной пустой папке replay — 17 passed / 3 ignored. В SDK эта ошибка replay уже исправлена, как и часть сохранения/UID/scene-group проблем.

В `frontend-repros.mjs` можно передать целевую копию через NOUS_AUDIT_ROOT, файл результата — через NOUS_AUDIT_FRONTEND_RESULT. Отдельные manifest тестов сохраняют собственные lock-файлы. Исходники обоих приложений не менялись.
