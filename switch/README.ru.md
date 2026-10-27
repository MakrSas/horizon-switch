# Horizon для Nintendo Switch

[English version](README.md)

Неофициальный порт [Horizon](https://github.com/ludovic111/horizon) на Nintendo Switch в виде homebrew. Horizon — это Minecraft Java Edition, переписанный на Rust. Порт — полноценная одиночная игра: работают сохранения, игра ставится как NSP со своей иконкой на главном экране.

Проект не связан с Mojang, Microsoft или Nintendo. Файлов Minecraft в нём нет: их делаешь сам из своей копии Minecraft Java Edition.

## Состояние

Проверено на одной Switch v1 (Erista), прошивка 22.5.0, Atmosphère 1.12.1.

| Что | Состояние |
|---|---|
| Одиночная игра | Работает: создание мира, выживание и творчество, сохранения. |
| Частота кадров | До 60 fps при дальности 6 в портативном режиме; ниже на большей дальности и пока генерируются новые чанки. |
| Графика | Vulkan через NVK (открытый драйвер NVIDIA из Mesa, порт [mesa-switch](https://github.com/danfromtico/mesa-switch)). Запасной вариант — OpenGL через Mesa из devkitPro. |
| Управление | Joy-Con (в консоли и отдельно) и Pro Controller, раскладка как в Bedrock. Кнопки переназначаются в настройках контроллера (Controller Binds). Есть HD-вибрация с отдельным переключателем для ломания блоков. Сенсор работает в меню и инвентаре, а в мире только выбирает ячейку хотбара. |
| Запуск | Около 45 с до входа в мир: ~11 с до главного меню, остальное — генератор мира. Первые секунды экран чёрный. |
| Скины | Скин любого аккаунта Minecraft: Настройки → Настройки скина, ввести ник. Нужен интернет один раз, дальше скин хранится для игры без сети. |
| Звук | Пока нет. |
| Мультиплеер | Не поддерживается. |
| Режим в доке | Не проверялся. |

## Установка

Нужна Switch с [Atmosphère](https://github.com/Atmosphere-NX/Atmosphere) и установщик NSP, например [DBI](https://github.com/rashevskyv/dbi).

1. **Сделай файлы игры из своего Minecraft.** На ПК, где Minecraft Java Edition 26.3 установлен через официальный лаунчер, собери `horizon-prepare` из этого репозитория и запусти:

   ```bash
   cargo run --release -p horizon-prepare -- --prepare --root resources
   ```

   Он возьмёт файлы игры из твоей установки Minecraft (другую папку можно указать через `--minecraft-dir`) и запишет ресурсы Horizon в `resources/`.

2. **Упакуй их в один образ RomFS** утилитой `build_romfs` из devkitPro (входит в switch-tools):

   ```bash
   switch/scripts/make-romfs.sh resources resources.romfs
   ```

   Скопируй `resources.romfs` на карту в `sdmc:/switch/horizon/resources.romfs` (около 600 МБ). Обычная папка `sdmc:/switch/horizon/resources/` тоже работает, но грузится в несколько раз медленнее.

3. **Установи `Horizon.nsp`** со страницы [Releases](../../releases) через DBI (или любой другой установщик NSP) и запусти Horizon с главного экрана.

Миры, настройки и логи лежат в `sdmc:/switch/horizon/`. Лог: `sdmc:/switch/horizon/logs/latest.log`.

**Разгон (по желанию).** Через [sys-clk](https://github.com/retronx-team/sys-clk) или похожую утилиту можно поднять частоты для title ID `0100485952450000`. Сильнее всего игру ограничивает процессор.

## Управление

| Действие | Кнопка |
|---|---|
| Ходьба / обзор | Левый стик / правый стик |
| Прыжок | A |
| Выбросить предмет | B |
| Поменять руки | X |
| Инвентарь | Y |
| Атака и ломание / использование и установка | ZR / ZL |
| Хотбар | L, R |
| Бег / присесть | Нажатие левого / правого стика |
| Вид, взять блок, чат, достижения | Крестовина |
| Список игроков | − |
| Пауза | + |

Всё, кроме паузы, можно переназначить. Там же можно назначить кнопку на экран отладки (F3).

## Сборка

Сборка идёт на Linux (проверено на Ubuntu 24.04).

**Понадобится:**

- [devkitPro](https://devkitpro.org/wiki/Getting_Started) с `devkitA64`, `libnx`, `switch-mesa`, `switch-libexpat`, `switch-zlib`, `switch-zstd` и `switch-tools`.
- Nightly Rust с `rust-src` (`rustup toolchain install nightly --component rust-src`). Проверялось на `rustc 1.101.0-nightly (76456a688 2026-10-25)`; другой тулчейн задаётся через `RUST_TOOLCHAIN`.
- NVK, собранный один раз: `switch/scripts/build-nvk.sh`. Нужны Docker или Podman и около 20 ГБ места. Скрипт использует сборочные скрипты и патчи NVK из [bevy-horizon](https://github.com/JohnUzoka/bevy-horizon) и кладёт результат в `switch/build/nvk/lib/libvulkan_bh.a`.
- Для NSP: [hacbrewpack](https://github.com/rlaphoenix/hacBrewPack) и `prod.keys` твоей консоли (снимаются через Lockpick_RCM). Никогда не коммить и не выкладывай ключи.

**Дальше, из корня репозитория:**

```bash
switch/scripts/build.sh                                  # switch/build/horizon.elf
switch/scripts/make-nsp.sh ~/switch/prod.keys            # switch/build/Horizon.nsp
```

Сборка под ПК не меняется: всё из `switch/` используется, только когда запускаешь эти скрипты.

## Как устроен порт

- **Цель сборки.** `aarch64-nintendo-switch-newlib.json` — своя цель Rust (`target_os = "horizon"`, newlib); линкуется devkitA64 с libnx. `std` собирается из исходников через `-Z build-std`, из копии `rust-src` тулчейна с патчем `std/std.patch`.
- **Пропатченные крейты** (`patches/`, подключены через `[patch.crates-io]` воркспейса): `libc` (newlib на AArch64 Horizon, делается `patch_libc.py`), `getrandom`, `ring`, `winit` (бэкенд `platform_impl/nx`: окно, фокус и выход апплета, сенсор), `wgpu` и `wgpu-core` (бэкенды Vulkan и GLES на Horizon) и заглушка `arboard`. В `third_party/wgpu-hal` добавлены поверхность EGL для Horizon и поверхность VI для NVK.
- **nxshim** (`nxshim/nxshim.c`) — прослойка на C, которая линкуется в игру:
  - заглушки того, чего нет в newlib (`getrandom`, `sysconf`, `dlopen`);
  - ключи pthread сверх лимита newlib;
  - `rename` поверх существующего файла;
  - отчёты о падениях в `fatal.txt`;
  - монтирование RomFS;
  - раскладка потоков по ядрам;
  - указатель потока, который читает Rust-код внутри NVK.
- **`crates/horizon-nx`** — безопасные обёртки для игры: контроллеры и HD-вибрация, память, RomFS и глобальный аллокатор. Аллокатор — это кэш мелких блоков без блокировок из bevy-horizon поверх malloc из newlib: единственная блокировка malloc тормозила все потоки.
- **Производительность.** Приоритеты потоков на Horizon (`crates/horizon-threads`), кэш отрисовки предметов в HUD и настройки по умолчанию для Switch в `settings.rs`.

## Благодарности

- [Horizon](https://github.com/ludovic111/horizon) от Ludovic, MIT.
- [mesa-switch](https://github.com/danfromtico/mesa-switch) от danfromtico и NaGaa95 и [bevy-horizon](https://github.com/JohnUzoka/bevy-horizon) от JohnUzoka (MIT или Apache-2.0): NVK для Switch, его патчи и сборочные скрипты, изменения wgpu-hal для Vulkan на Horizon и кэш мелких блоков для аллокатора.
- [libnx](https://github.com/switchbrew/libnx) и [devkitPro](https://devkitpro.org).
- Пропатченные крейты сохраняют свои лицензии: libc, getrandom, winit, wgpu (MIT или Apache-2.0), ring (лицензия в стиле ISC).

Minecraft — торговая марка Mojang Synergies AB. Nintendo Switch — торговая марка Nintendo.
