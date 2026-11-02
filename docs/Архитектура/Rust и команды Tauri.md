---
tags: [nous, архитектура, rust]
---

# Rust и команды Tauri

Всё, что нельзя сделать в браузере (перехват трафика, файлы игры, процессы, долгие задачи), живёт на стороне Rust в `app/src-tauri/src/`. Фронт вызывает это через **команды Tauri**.

## Соглашение: одна система — один модуль

```
src-tauri/src/
├─ lib.rs          регистрация модулей и команд
├─ profile.rs      профиль игрока (get_profile)
└─ <система>.rs    например sniffer.rs, datamine.rs, hoyolab.rs
```

Если система разрастается, модуль превращается в папку `<система>/mod.rs` с подмодулями.

## Добавить команду

1. Функция с атрибутом `#[tauri::command]` в модуле системы:
   ```rust
   // src-tauri/src/warps.rs
   use serde::Serialize;

   #[derive(Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct Warp { pub item_id: u32, pub time: i64 }

   #[tauri::command]
   pub fn get_warps() -> Result<Vec<Warp>, String> {
       Ok(vec![])
   }
   ```
2. Подключить модуль и команду в `lib.rs`:
   ```rust
   mod warps;
   // …
   .invoke_handler(tauri::generate_handler![
       profile::get_profile,
       warps::get_warps,
   ])
   ```
3. Вызвать с фронта. Принято держать вызовы в `api.ts` своей фичи:
   ```ts
   // src/features/warps/api.ts
   import { invoke, isTauri } from '@tauri-apps/api/core'
   import type { Warp } from './types'

   export async function getWarps(): Promise<Warp[]> {
     if (!isTauri()) return []          // npm run dev в браузере, без Rust
     return invoke<Warp[]>('get_warps')
   }
   ```

> [!warning] Регистрация команд не автоматическая
> В отличие от экранов, команды Tauri собираются на этапе компиляции. Каждую нужно явно вписать в `generate_handler!`, иначе фронт получит ошибку «command not found».

## Правила

- **Имена:** команда `snake_case`, глагол + объект (`get_profile`, `start_capture`). На фронте вызывается той же строкой.
- **Поля структур** отдаём в camelCase: `#[serde(rename_all = "camelCase")]`. Тогда TS-тип совпадает один в один.
- **TS-тип** зеркалит Rust-структуру и лежит рядом с вызовом, с комментарием `// Mirrors X in src-tauri/src/…`.
- **Ошибки** возвращаем как `Result<T, String>`. Фронт показывает их через [[Компоненты#EmptyState|EmptyState]] или [[Компоненты#Badge|Badge]] `danger`.
- **Нет данных ≠ ошибка.** Если аккаунт не привязан, возвращаем пустые значения, а фронт показывает нули. Так сделан профиль, см. [[Данные профиля]].
- **Долгие процессы** (сниффер) запускаются в фоне. Данные отдаются через события Tauri (`app.emit`), фронт подписывается через `listen`, а не опрашивает.
- **Большие бинарные данные** отдаём через `tauri::ipc::Response`: придут как `ArrayBuffer` без JSON.

## Связанные заметки

- [[Обзор архитектуры]]
- [[Данные профиля]]
- [[Источники данных]]
