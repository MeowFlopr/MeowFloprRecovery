# Building MeowFloprRecovery / Сборка MeowFloprRecovery

[🇬🇧 English](#-english) | [🇷🇺 Русский](#-русский)

---

## 🇬🇧 English

This guide provides technical instructions for compiling the MeowFloprRecovery (MrR) pure Rust codebase for the `aarch64-unknown-linux-musl` target and assembling the final `vendor_boot.img` partition image.

### Prerequisites

- Host Operating System: Linux (Ubuntu 22.04 LTS+, Debian 12+, Arch Linux) or WSL2 on Windows 10/11.
- Rust toolchain (Rust 1.75 or later via `rustup`).
- Python 3 with the `lz4` compression module.
- Standard packaging utilities: `cpio`, `git`.
- Note: No C compiler or C library toolchain is required for the recovery core; it is 100% pure Rust.

### 1. Build Environment Setup

Install the required host packages (Debian / Ubuntu):

```bash
sudo apt-get update
sudo apt-get install -y build-essential python3 python3-pip git cpio
pip3 install lz4
```

Add the static musl AArch64 compilation target via `rustup`:

```bash
rustup target add aarch64-unknown-linux-musl
```

### 2. Compiling the Recovery Core

Execute the release compilation directly with Cargo:

```bash
cargo build --target aarch64-unknown-linux-musl --release
```

During compilation:
1. The native Direct Rendering Manager (DRM KMS) atomic modesetting engine implemented entirely in Rust is compiled.
2. The complete Rust graphics engine (tiny-skia 2D canvas, fontdue font rasterizer) and recovery state machines are compiled.
3. Release profile optimizations defined in `Cargo.toml` are applied:
   - `opt-level = "z"` (aggressive binary size minimization).
   - `lto = true` (link-time optimization).
   - `codegen-units = 1` (monolithic compilation unit for maximum dead code elimination).
   - `strip = true` (automatic symbol stripping).

The compiled static binary is generated at:
`target/aarch64-unknown-linux-musl/release/mrr`
Binary footprint is approximately 1.48 MB.

### 3. Assembling the GKI v4 vendor_boot Image

The `vendor_boot` partition on the MediaTek MT6897 architecture has a strict physical ceiling: exactly 64 MiB (67,108,864 bytes). Exceeding this boundary will result in fastboot flash rejection.

The composite image comprises:
- GKI v4 header (4096-byte page alignment).
- `vendor_ramdisk00`: platform vendor kernel modules (LZ4 compressed).
- `vendor_ramdisk01`: recovery userspace ramdisk containing the `mrr` binary and `/system/bin/recovery` wrapper.
- Device Tree Blob (`dtb.img`).
- Vendor ramdisk table describing entry offsets.

Execute the packager script `scripts/pack_vendor_boot.py`:

```bash
python3 scripts/pack_vendor_boot.py \
  --orig-img path/to/base_vendor_boot.img \
  --ramdisk-cpio path/to/recovery_ramdisk.cpio \
  --rd00 path/to/vendor_ramdisk00_platform.lz4 \
  --dtb path/to/dtb.img \
  --rust-bin target/aarch64-unknown-linux-musl/release/mrr \
  --output MRR-degas-vendor_boot.img
```

The script automatically validates the final byte size against the 64 MiB hardware boundary.

### 4. Feedback & Support

For compilation questions or build failure reports, open an Issue on GitHub or contact the developer on Telegram: https://t.me/kotvava.

---

## 🇷🇺 Русский

В данном руководстве приведена инструкция по компиляции ядра MeowFloprRecovery (MrR) на чистом языке Rust под архитектуру `aarch64-unknown-linux-musl` и сборке итогового образа `vendor_boot.img`.

### Системные требования

- Операционная система: Linux (Ubuntu 22.04 LTS+, Debian 12+, Arch Linux) либо WSL2 под управлением Windows 10/11.
- Установленный инструментарий Rust (версия 1.75 или новее через `rustup`).
- Интерпретатор Python 3 с установленным пакетом `lz4`.
- Стандартные утилиты сборки: `cpio`, `git`.
- Примечание: Для сборки ядра рекавери не требуются Си-компиляторы; проект написан на 100% чистом Rust.

### 1. Подготовка окружения сборки

Установите необходимые пакеты в системе (на примере Ubuntu / Debian):

```bash
sudo apt-get update
sudo apt-get install -y build-essential python3 python3-pip git cpio
pip3 install lz4
```

Установите целевую платформу для компиляции статических бинарных файлов через `rustup`:

```bash
rustup target add aarch64-unknown-linux-musl
```

### 2. Компиляция ядра рекавери

Для запуска процесса сборки выполните:

```bash
cargo build --target aarch64-unknown-linux-musl --release
```

В процессе сборки:
1. Компилируется нативный модуль взаимодействия с подсистемой ядра DRM KMS atomic modesetting, написанный полностью на Rust.
2. Компилируются все Rust-модули интерфейса (tiny-skia 2D canvas, шрифтовой движок fontdue).
3. Применяются оптимизации размера из профиля `release`:
   - `opt-level = "z"` (агрессивная оптимизация по размеру исполняемого кода).
   - `lto = true` (межмодульная оптимизация на этапе линковки).
   - `codegen-units = 1` (максимальная монолитность кода).
   - `strip = true` (удаление отладочных символов).

Итоговый бинарный файл располагается по пути:
`target/aarch64-unknown-linux-musl/release/mrr`
Его размер составляет приблизительно 1.48 МБ.

### 3. Сборка итогового образа vendor_boot

Раздел `vendor_boot` на платформе MediaTek MT6897 с ядром GKI v4 имеет жесткий физический лимит: 64 МБ (67 108 864 байт). Превышение данного размера приведет к ошибке fastboot при прошивке.

Структура образа включает:
- Заголовок GKI v4 (размер страницы: 4096 байт).
- `vendor_ramdisk00`: платформенные драйверы и модули ядра MediaTek (сжатие LZ4).
- `vendor_ramdisk01`: ramdisk среды рекавери с внедренным бинарником `mrr` и скриптом-оберткой `/system/bin/recovery`.
- Таблица дерева устройств (`dtb.img`).
- Таблица смещений ramdisk (ramdisk table).

Для упаковки используется скрипт `scripts/pack_vendor_boot.py`:

```bash
python3 scripts/pack_vendor_boot.py \
  --orig-img path/to/base_vendor_boot.img \
  --ramdisk-cpio path/to/recovery_ramdisk.cpio \
  --rd00 path/to/vendor_ramdisk00_platform.lz4 \
  --dtb path/to/dtb.img \
  --rust-bin target/aarch64-unknown-linux-musl/release/mrr \
  --output MRR-degas-vendor_boot.img
```

После завершения скрипт проверит размер файла и выведет запас свободного пространства в разделе.

### 4. Обратная связь

При возникновении ошибок компиляции или компоновки создавайте Issue в репозитории либо пишите в Telegram: https://t.me/kotvava.
