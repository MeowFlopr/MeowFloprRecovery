# MeowFloprRecovery (MrR)

[🇷🇺 Русский](#-русский) | [🇬🇧 English](#-english)

---

## 🇷🇺 Русский

MeowFloprRecovery (MrR) — кастомное рекавери нового поколения для смартфона Xiaomi 14T (кодовое имя `degas`, процессор MediaTek Dimensity 8300-Ultra / MT6897), написанное с нуля на языке Rust.

Проект создан как независимая альтернатива классическому TWRP. В отличие от стандартных AOSP C++ реализаций, MrR не зависит от многогигабайтных деревьев сборки Android, собирается в виде полностью статического бинарного файла и напрямую взаимодействует с подсистемами ядра Linux.

### Статус проекта

Статус: BETA.

Проект находится в стадии активной разработки. Стабильная и безошибочная работа всех компонентов не гарантируется, программное обеспечение предоставляется «как есть» (AS IS). Разработчик не несет ответственности за поврежденные устройства, утерю данных или окирпичивание в результате некорректного использования.

### История разработки

Рекавери было спроектировано и полностью написано с нуля на Rust примерно за 1 день с активным использованием AI-агентов (agentic coding). Вся архитектура — от низкоуровневой работы с Direct Rendering Manager (DRM KMS) и аппаратной обработки сенсорного ввода Goodix до графического движка на tiny-skia и модулей работы с криптографией ядра Linux — создавалась в цикле непрерывной верификации на реальном физическом устройстве.

### Архитектура и технические особенности

- Ядро и среда исполнения: чистый Rust (target: `aarch64-unknown-linux-musl`), статическая линковка, отсутствие зависимостей от libc Android (bionic) во внутреннем рантайме.
- Графика: прямое управление фреймбуфером через Linux Direct Rendering Manager (DRM / KMS), без промежуточных композиторов. Векторный рендеринг через tiny-skia с аппаратной ориентацией экрана 1220x2712 при 144 Гц.
- Сенсорный ввод: драйвер прямого чтения событий Linux evdev с динамической нормализацией координат под матрицу дисплея и поддержкой мультитач.
- Хранилище: поддержка динамических разделов (Android Dynamic Partitions via libdm / dm-linear), суперраздела `super` (erofs / ext4).
- Дешифрование: гибридная схема расшифровки FBE v2 (fscrypt) и dm-default-key без зависимостей от KeyMint HAL в режиме рекавери через связку с MrR Companion.
- Формат упаковки: GKI v4 `vendor_boot` (header version 4, раздельные ramdisk00 для платформенных драйверов и ramdisk01 для среды восстановления).

### Возможности

- Сенсорный графический интерфейс в стилистике Material 3 с темной темой.
- Полноценная файловая навигация и установка ZIP-пакетов прошивок (Magisk, KernelSU, модули, OTA).
- Расшифровка пользовательского раздела `/data` без необходимости отключения аппаратного шифрования.
- Выборочная очистка разделов (Wipe Cache, Dalvik, System, Vendor, Metadata, Data).
- Полное форматирование данных (Format Data) с подтверждением через клавиатуру.
- Резервное копирование и восстановление системных и сервисных разделов (`boot`, `vendor_boot`, `dtbo`, `super`, `persist`, `nvram`, `nvdata`).
- Переключение активного слота загрузки A/B через прямое управление структурой `bootloader_control` в разделе `misc`.
- Встроенная консоль с выводом сообщений ядра (`dmesg`) и ходом выполнения операций установки.
- Режим MTP для прямого доступа к файловой системе смартфона с ПК по USB-кабелю.

### Документация

- [Инструкция по установке](docs/INSTALL.md)
- [Технический анализ шифрования MT6897 и MrR Companion](docs/ENCRYPTION.md)
- [Руководство по сборке](docs/BUILD.md)

### Обратная связь и баги

При обнаружении проблем открывайте Issue в данном репозитории GitHub либо пишите напрямую автору в Telegram: https://t.me/kotvava (туда же направляйте идеи и предложения по развитию).

### Лицензия

Проект распространяется под лицензией Apache License 2.0. Подробности в файле [LICENSE](LICENSE).

---

## 🇬🇧 English

MeowFloprRecovery (MrR) is an open-source, next-generation custom recovery for the Xiaomi 14T (codename `degas`, MediaTek Dimensity 8300-Ultra / MT6897 processor), written completely from scratch in Rust.

The project was designed as a modern, lightweight alternative to legacy TWRP. Unlike traditional AOSP C++ implementations, MrR does not depend on massive multi-gigabyte Android build trees, compiles into a standalone static binary, and interfaces directly with the Linux kernel subsystems.

### Project Status

Status: BETA.

This project is under active development. Stable and bug-free operation across all components is not guaranteed; the software is provided "AS IS". The developer assumes no liability for bricked devices, lost data, or hardware damage resulting from the use of this software.

### Development History

The recovery was designed and completely written from scratch in Rust in approximately 1 day with active use of AI agents (agentic coding). The entire architecture — from low-level Direct Rendering Manager (DRM KMS) integration and Goodix touchscreen evdev handling to the custom tiny-skia UI engine and Linux kernel keyring ioctls — was developed with continuous real-device hardware verification.

### Architecture & Technical Highlights

- Core & Runtime: Pure Rust (`aarch64-unknown-linux-musl` target), statically linked, zero runtime dependency on Android bionic libc.
- Graphics: Direct framebuffer control via Linux Direct Rendering Manager (DRM / KMS) without display compositors. High-performance 2D vector rendering via tiny-skia tailored for 1220x2712 AMOLED at 144 Hz.
- Touch Input: Direct Linux evdev event parser with dynamic coordinate normalization and multi-touch support.
- Storage: Android Dynamic Partitions support via libdm / dm-linear, reading and mounting `super` subvolumes (erofs and ext4).
- Decryption: Hybrid FBE v2 (fscrypt) and dm-default-key decryption pipeline bypassing recovery KeyMint HAL limitations via the MrR Companion subsystem.
- Packaging: Android GKI v4 `vendor_boot` layout (header v4 with split vendor ramdisk00 for platform kernel modules and ramdisk01 for recovery userspace).

### Features

- Material 3 inspired dark touch user interface.
- Touch file manager and ZIP package installer (compatible with Magisk, KernelSU, system patches).
- Hardware-backed `/data` decryption without disabling storage encryption.
- Partition wipe menu (Dalvik / ART cache, Cache, System, Vendor, Metadata, Data).
- Full Factory Reset and Format Data with explicit typed confirmation.
- Partition Backup and Restore (`boot`, `vendor_boot`, `dtbo`, `super`, `persist`, `nvram`, `nvdata`).
- A/B active boot slot switching via direct modification of the `bootloader_control` struct in the `misc` partition.
- Action console and live kernel `dmesg` log inspector.
- USB MTP mode for direct file transfer between device and PC.

### Documentation

- [Installation Guide](docs/INSTALL.md)
- [MT6897 Encryption Architecture & MrR Companion](docs/ENCRYPTION.md)
- [Build Instructions](docs/BUILD.md)

### Feedback & Bug Reports

If you encounter bugs or hardware compatibility issues, please open a GitHub Issue in this repository or contact the author directly on Telegram: https://t.me/kotvava (feature suggestions and development contributions are also welcome).

### License

Distributed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
