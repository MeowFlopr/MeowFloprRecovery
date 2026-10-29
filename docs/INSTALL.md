# Инструкция по установке / Installation Guide

[🇷🇺 Русский](#-русский) | [🇬🇧 English](#-english)

---

## 🇷🇺 Русский

Данный документ описывает пошаговый процесс прошивки MeowFloprRecovery (MrR) на смартфон Xiaomi 14T (`degas`).

### Требования перед началом

1. Разблокированный загрузчик (Bootloader Unlocked). Прошивка кастомных образов на заблокированный загрузчик невозможна.
2. Компьютер под управлением Windows, Linux или macOS с установленными утилитами `adb` и `fastboot` (Android Platform Tools).
3. Рабочий кабель USB Type-C и уровень заряда аккумулятора не менее 50%.
4. Резервная копия оригинального раздела `vendor_boot` вашей текущей сборки HyperOS.

### Архитектура раздела vendor_boot в Android 14+ (GKI v4)

В смартфонах на базе ядра Linux 6.1 (GKI v4) ядро операционной системы находится в разделе `boot`, а ramdisk рекавери, дерево устройств (DTB) и платформенные модули ядра размещаются в разделе `vendor_boot`. Команда `fastboot boot recovery.img` не поддерживается загрузчиками MediaTek MT6897, поэтому образ прошивается напрямую в раздел `vendor_boot`.

### Пошаговая прошивка

#### Шаг 1: Перевод устройства в режим Fastboot

1. Выключите смартфон.
2. Зажмите и удерживайте комбинацию кнопок: «Громкость вниз» + «Питание» до появления надписи FASTBOOT на экране.
3. Подключите смартфон к компьютеру через USB.
4. Проверьте обнаружение устройства в терминале:

```bash
fastboot devices
```

Если серийный номер отображается, устройство готово к прошивке.

#### Шаг 2: Создание резервной копии текущего vendor_boot (рекомендуется)

Перед перезаписью сохраните образ заводского раздела:

```bash
fastboot fetch vendor_boot stock_vendor_boot.img
```

Если команда `fetch` не поддерживается загрузчиком вашего региона, извлеките `vendor_boot.img` из официальной fastboot-прошивки вашей версии HyperOS.

#### Шаг 3: Прошивка образа MeowFloprRecovery

Выполните команду записи образа в раздел `vendor_boot`:

```bash
fastboot flash vendor_boot MRR-degas-vendor_boot.img
```

При использовании явного указания слотов можно прошить образ в конкретный активный слот:

```bash
fastboot flash vendor_boot_a MRR-degas-vendor_boot.img
# или
fastboot flash vendor_boot_b MRR-degas-vendor_boot.img
```

#### Шаг 4: Загрузка в рекавери

Перезагрузите смартфон в режим восстановления:

```bash
fastboot reboot recovery
```

Либо зажмите физическую комбинацию: «Громкость вверх» + «Питание» сразу после перезагрузки.

### Первый запуск и дешифрование

1. При первом старте рекавери инициализирует экран AMOLED 1220x2712 и загрузит драйвер сенсорного слоя Goodix.
2. Если раздел `/data` зашифрован и ключи еще не были синхронизированы через MrR Companion, интерфейс предложит варианты:
   - Пропустить дешифрование (кнопка «Отмена / Cancel») для работы с другими разделами или прошивки ZIP.
   - Ввести PIN-код блокировки экрана или ручной 128-значный hex-ключ.
   - Экспортировать модуль MrR Companion для установки в запущенную систему.
3. Подробная инструкция по настройке автоматической расшифровки приведена в документе [ENCRYPTION.md](ENCRYPTION.md).

### Восстановление при сбоях (Rollback)

Если после прошивки устройство не загружается в рекавери или зависает на заставке:

1. Переведите смартфон обратно в режим Fastboot («Громкость вниз» + «Питание»).
2. Восстановите оригинальный образ:

```bash
fastboot flash vendor_boot stock_vendor_boot.img
fastboot reboot
```

3. Для решения проблем обратитесь в Issue репозитория либо в Telegram: https://t.me/kotvava.

---

## 🇬🇧 English

This guide provides step-by-step instructions for flashing MeowFloprRecovery (MrR) on the Xiaomi 14T (`degas`).

### Prerequisites

1. Unlocked Bootloader. Flashing third-party partition images is not permitted on locked devices.
2. A PC running Windows, Linux, or macOS with modern `adb` and `fastboot` utilities (Android SDK Platform-Tools).
3. Reliable USB Type-C cable and at least 50% battery level.
4. Backup copy of the stock `vendor_boot` partition matching your currently installed HyperOS build.

### Android 14+ GKI v4 vendor_boot Layout

On devices running Linux kernel 6.1 (Generic Kernel Image v4), the generic kernel binary is stored in `boot`, while the recovery ramdisk, device tree blob (DTB), and SoC vendor modules reside in `vendor_boot`. The legacy `fastboot boot recovery.img` command is not supported by MediaTek MT6897 bootloaders; the image must be flashed directly into the `vendor_boot` partition.

### Installation Steps

#### Step 1: Boot Device into Fastboot Mode

1. Power off the smartphone completely.
2. Press and hold Volume Down + Power buttons until the FASTBOOT logo appears.
3. Connect the device to your PC with the USB cable.
4. Verify host connectivity in terminal:

```bash
fastboot devices
```

The device serial number should appear in the output.

#### Step 2: Backup Stock vendor_boot (Recommended)

Save a copy of your factory partition before writing:

```bash
fastboot fetch vendor_boot stock_vendor_boot.img
```

If the `fetch` command is disabled on your device firmware, extract `vendor_boot.img` from the official fastboot recovery package of your current HyperOS build.

#### Step 3: Flash MeowFloprRecovery Image

Flash the recovery image into the `vendor_boot` partition:

```bash
fastboot flash vendor_boot MRR-degas-vendor_boot.img
```

Alternatively, write directly to a specific slot:

```bash
fastboot flash vendor_boot_a MRR-degas-vendor_boot.img
# or
fastboot flash vendor_boot_b MRR-degas-vendor_boot.img
```

#### Step 4: Boot into Recovery

Reboot the phone into recovery mode:

```bash
fastboot reboot recovery
```

Alternatively, hold Volume Up + Power during reboot.

### Initial Boot & Storage Handling

1. During initialization, MrR configures the 1220x2712 AMOLED panel and loads the Goodix touch driver.
2. If `/data` is encrypted and companion keys have not been synced yet, the unlock screen will present several options:
   - Skip decryption ("Отмена / Cancel") to access unencrypted partitions or flash standalone packages.
   - Enter your screen lock PIN or a manual 128-character hexadecimal master key.
   - Export the MrR Companion application and Magisk module for installation inside booted HyperOS.
3. Refer to [ENCRYPTION.md](ENCRYPTION.md) for full technical details on seamless automated decryption.

### Recovery and Rollback

If the device fails to boot or loops at the splash screen:

1. Put the smartphone back into Fastboot mode (Volume Down + Power).
2. Restore the original vendor boot image:

```bash
fastboot flash vendor_boot stock_vendor_boot.img
fastboot reboot
```

3. For support or bug reports, open a GitHub Issue or reach out via Telegram: https://t.me/kotvava.
