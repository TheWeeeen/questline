# questline

[![CI](https://github.com/TheWeeeen/questline/actions/workflows/ci.yml/badge.svg)](https://github.com/TheWeeeen/questline/actions/workflows/ci.yml)

Запуск игровой активности для Discord: квесты «поиграть N минут», фарм часов, значки профиля. Rust-ядро без ОС внутри, тонкий UI, тесты на всё, что можно проверить без Discord.

Статус: **M0 готов** — домен, машина состояний, санитайзер путей, планировщик батчей, CLI `plan` и CI. Движок запуска процессов — M1.

## Зачем ещё один

[`mimic`](https://github.com/Timon2306/mimic-discord) делает то же самое, но вся логика прогона живёт в одном файле на 914 строк с `AtomicBool` и `RwLock`, тестов нет вообще, состояния прогона хранит UI, а каталог от Discord парсится без валидации. Здесь те же задачи разложены по слоям, у каждого слоя есть контракт и тест.

## Что уже есть

```
crates/core   домен, машина состояний, санитайзер путей, планировщик — 0 I/O
crates/cli    questline plan: читает каталог, печатает реальный план батчей
fixtures/     каталог-фикстура, включая заведомо плохие записи
```

Пример (реальный вывод, фикстура в репозитории):

```
$ cargo run -q -p questline-cli -- plan --catalogue fixtures/games.json --platform windows
questline 0.1.0 — platform windows, batch 8, 4 game(s) usable
catalogue fixtures/games.json: 8 entries, 1 without an executable for this platform
  rejected "Traversal Demo": path traversal in executable name: "../../evil.exe"
  rejected "Bad Id": invalid application id "notnumeric"
  rejected "Reserved Device Name": "CON.exe" is a reserved device name on Windows
batch 1/2:
  105600. Terraria                         Terraria.exe
  730. Counter-Strike 2                 cs2.exe
  999002. Launcher Only                    launcher.exe
batch 2/2:
  1056000. Terraria (same exe name)         terraria.exe
```

## Правила, которых придерживаемся

- **Каталог — недоверенные данные.** `../../evil.exe` не «чинится», а отклоняется с причиной: молча переписать путь в `evil.exe` хуже, чем пропустить игру.
- **Явные состояния.** `Idle → Preparing → Running{batch} → … → Finished{batches}`; недопустимый переход возвращает ошибку с названием действия, а не молча ничего не делает.
- **Две игры с одинаковым `.exe` не идут в одном батче** — планировщик откладывает вторую, а не роняет запуск.
- **Ядро ничего не делает с ОС.** Процессы, время и сеть приходят через трейты из `ports`, поэтому весь сценарий прогоняется на фейках в тестах.
- **Никаких `unwrap` в продакшен-коде** — это ошибка линта, в тестах разрешено явно.

## Сборка и проверки

```bash
cargo test --all          # 44 теста
cargo clippy --all-targets -- -D warnings -A clippy::missing_errors_doc
cargo fmt --all --check
cargo deny check          # advisories, лицензии, источники
```

CI гоняет всё это на каждый push: `.github/workflows/ci.yml` — работа `fmt, clippy, test` и `advisories and licenses` (cargo-deny).

## План

- [x] **M0** — каркас, домен, машина состояний, санитайзер, планировщик, CLI `plan`, CI
- [ ] **M1** — движок прогонов: спавн, реестр PID, корректный cleanup (Job Object на Windows, PDEATHSIG на Linux), `questline run`
- [ ] **M2** — Discord-слой: ETag-кэш каталога, валидация схемы, RPC-провайдер, обработка 429
- [ ] **M3** — квесты: режим одного процесса с таймером, история в SQLite
- [ ] **M4** — UI на Tauri: прогон, квесты, база, логи, настройки; светлая тема, клавиатура, aria
- [ ] **M5** — релизы: подписанные артефакты, автообновление, portable/AppImage/deb, winget
- [ ] **M6** — документация: threat model, ADR на каждое решение

## Честно о границах

- На Linux Discord не начисляет награды за квесты — цель только Windows и macOS.
- Квесты, которые проверяют путь установки или Steam (GOALS, Marathon), этим способом не закрываются.
- С 2026 Discord применяет санкции за автоматизацию квестов, вплоть до записи в account standing. Инструмент делает то, что просит пользователь; решение и риск — его.
- Токен Discord не читается и не запрашивается.

## Лицензия

MIT, см. [LICENSE](LICENSE).
