# Аппаратное шифрование MT6897 и MrR Companion / MT6897 Hardware Encryption & MrR Companion

[🇷🇺 Русский](#-русский) | [🇬🇧 English](#-english)

---

## 🇷🇺 Русский

В данном документе подробно разбирается архитектура шифрования пользовательских данных на платформе MediaTek Dimensity 8300-Ultra (MT6897) в операционной системе HyperOS (Android 14+), причины неработоспособности классических методов TWRP и принцип работы подсистемы MrR Companion.

### 1. Проблема аппаратного TEE и KeyMint 3.0

Начиная с Android 13/14 на современных чипсетах MediaTek, ключи шифрования пользовательского хранилища защищены аппаратной изолированной средой исполнения Microtrust Mitee (Trusted Execution Environment, TEE) и модулем KeyMint 3.0.

#### Механизм аппаратной привязки (Root of Trust)

1. При запуске основной системы (Android OS) загрузчик передает в TEE криптографические измерения: хеш ключа верификации ядра, версию ОС, уровень патча безопасности и статус разблокировки загрузчика.
2. TEE проверяет параметры Root of Trust (RoT) и разрешает сервису `android.hardware.security.keymint` дешифровать защищенные мастер-ключи FBE (File-Based Encryption).
3. При загрузке в режим Recovery параметры RoT кардинально отличаются: ядро работает в другом контексте, отсутствуют подписанные системные сервисы framework и vold.
4. В результате TEE Microtrust Mitee блокирует любые запросы на расшифровку мастер-ключей из кастомного рекавери. Попытки запуска оригинальных демонов KeyMint и vold в окружении recovery приводят к таймаутам, падению HAL и отказу в выдаче ключа.

### 2. Уровни шифрования накопителя в Xiaomi 14T

В архитектуре HyperOS на `degas` применяется двухслойная защита:

1. Блочный уровень (Block-level):
   - Раздел `/dev/block/by-name/userdata` оборачивается в виртуальное блочное устройство `dm-default-key` (драйвер device-mapper ядра Linux).
   - Алгоритм: AES-256-XTS с аппаратным ускорением криптографического движка MediaTek.
   - Без инициализации данного слоя файловая система `f2fs` на разделе `userdata` не может быть смонтирована.

2. Файловый уровень (Filesystem-level, FBE v2):
   - Файловая система F2FS использует встроенный механизм `fscrypt` v2.
   - Каждый файл и каталог шифруется отдельным производным ключом от мастер-ключа CE (Credential Encrypted) или DE (Device Encrypted).
   - Если в keyring ядра не добавлен мастер-ключ через вызов `FS_IOC_ADD_ENCRYPTION_KEY`, имена файлов и каталогов отображаются в виде зашифрованных хэшей (например, `0cvusgAAAAD2hKZ...`), а доступ к содержимому файлов блокируется.

### 3. Архитектура и решение MrR Companion

Чтобы обойти аппаратную блокировку TEE без компрометации защиты и без необходимости обязательного форматирования данных, в MeowFloprRecovery разработана архитектура MrR Companion.

#### Принцип работы

1. Захват ключей в легитимном контексте:
   - В запущенной системе HyperOS аппаратная среда TEE уже успешно проверила RoT и разблокировала ключи.
   - Фоновый сервис MrR Companion, работающий с правами суперпользователя (root), считывает актуальный 128-значный hex-ключ блочного шифрования через `dmsetup table --showkeys userdata` и мастер-ключ CE.

2. Сохранение в защищенный раздел `/persist`:
   - Извлеченные ключи сохраняются в файл `/mnt/vendor/persist/mrr/userdata.key` с правами доступа `0600`.
   - Раздел `persist` (`/dev/block/by-name/persist`) выбран намеренно: это энергонезависимый сервисный раздел чипсета, где хранятся калибровки радиомодуля, сенсоров и Wi-Fi. Этот раздел никогда не затирается при сбросах к заводским настройкам (Factory Reset) и доступен для чтения на самых ранних этапах загрузки.

3. Автономная дешифрация в MeowFloprRecovery:
   - При старте MrR монтирует раздел `persist` в `/mnt/vendor/persist`.
   - Считывает сохраненный `userdata.key`.
   - Настраивает таблицу device-mapper через системный вызов `ioctl(DM_TABLE_LOAD)` с параметром `default-key`.
   - Монтирует результирующее блочное устройство `/dev/block/mapper/userdata` в каталог `/data`.
   - Считывает мастер-ключ CE и регистрирует его в keyring ядра Linux через вызов:
     ```c
     ioctl(fd, FS_IOC_ADD_ENCRYPTION_KEY, &key_spec);
     ```
   - Ядро Linux мгновенно дешифрует файловую структуру: все каталоги `/sdcard` (`Download`, `DCIM`, `Documents`) становятся доступными в открытом виде.

### 4. Компоненты MrR Companion

Подсистема состоит из трех взаимодополняющих компонентов:

1. `MrRCompanion.apk`:
   - Нативное легковесное Android-приложение (написано без сторонних тяжелых библиотек).
   - Позволяет в один клик синхронизировать ключи, проверить статус разделов и установить системную службу автозапуска.

2. Модуль Magisk / KernelSU / APatch (`mrr-companion.zip`):
   - Модуль размещает скрипт `service.sh` в `/data/adb/modules/mrr-companion/`.
   - При каждой перезагрузке смартфона после завершения загрузки системы (`sys.boot_completed=1`) скрипт автоматически обновляет ключи в `/mnt/vendor/persist/mrr/`.
   - Даже если пользователь сменил PIN-код или обновил прошивку, ключи обновляются полностью прозрачно.

3. Встроенные prebuilt-ресурсы в MrR:
   - Исходный APK и ZIP-модуль скомпилированы и встроены прямо в бинарник `mrr` через `include_bytes!`.
   - В меню расшифровки пользователь может одной кнопкой распаковать модуль и APK на внутреннюю память (`/sdcard/MRR/`) или в раздел `/persist` для последующей установки.

### 5. Ручной ввод ключа

Если система была сброшена, а модуль еще не устанавливался, MeowFloprRecovery предоставляет альтернативные варианты:

- Ввод цифрового PIN-кода блокировки экрана для программной деривации ключа через Scrypt/PBKDF2.
- Ввод сырого 128-значного шестнадцатеричного мастер-ключа вручную с экранной клавиатуры.
- Пропуск дешифрования (кнопка «Отмена / Cancel») для выполнения независимых задач (прошивка ядра, бэкап системных разделов `boot`, `super`).

### Обратная связь

При вопросах по работе криптографического движка открывайте Issue на GitHub либо обращайтесь в Telegram: https://t.me/kotvava.

---

## 🇬🇧 English

This document provides a detailed breakdown of user data encryption on the MediaTek Dimensity 8300-Ultra (MT6897) SoC running HyperOS (Android 14+), explaining why classic TWRP approaches fail and how the MrR Companion subsystem resolves this challenge.

### 1. The TEE and KeyMint 3.0 Hardware Barrier

Starting with Android 13/14 on recent MediaTek platforms, user data encryption keys are sealed inside a hardware-isolated Trusted Execution Environment powered by Microtrust Mitee and KeyMint 3.0.

#### Root of Trust (RoT) Sealing Mechanism

1. During standard system boot (HyperOS), the bootloader measures the boot state and passes signed parameters to the TEE: OS version, security patch level, boot key hash, and bootloader lock state.
2. The TEE validates this Root of Trust (RoT) state and permits the `android.hardware.security.keymint` HAL to unseal the master Credential Encrypted (CE) and Device Encrypted (DE) keys.
3. In recovery mode, the verified boot environment is fundamentally different: the kernel runs in a separate context, and vendor framework services are offline.
4. Consequently, the Microtrust Mitee TEE rejects any decryption requests originating from custom recovery. Spawning stock KeyMint and vold binaries inside a recovery ramdisk results in binder timeouts, service crashes, and authentication failure.

### 2. Multi-Layer Storage Encryption in Xiaomi 14T

Storage security on `degas` uses a two-tier encryption stack:

1. Block-Level Encryption:
   - The underlying partition `/dev/block/by-name/userdata` is managed via the Linux device-mapper `dm-default-key` target.
   - Algorithm: AES-256-XTS with MediaTek hardware cryptographic engine offload.
   - Without active dm mapping, the `f2fs` filesystem on `userdata` cannot be mounted.

2. Filesystem-Level Encryption (FBE v2):
   - F2FS leverages the Linux kernel `fscrypt` v2 interface.
   - Individual inodes and directories are encrypted with keys derived from master credentials.
   - If the corresponding master key is not present in the Linux kernel keyring via `FS_IOC_ADD_ENCRYPTION_KEY`, filenames appear as base64 ciphertext hashes (e.g., `0cvusgAAAAD2hKZ...`), and file read operations fail with `EKEYREVOKED` or return ciphertext.

### 3. MrR Companion Architecture

To overcome the TEE RoT restriction without disabling encryption or requiring data formatting, MeowFloprRecovery implements the MrR Companion subsystem.

#### Operating Sequence

1. Key Acquisition in Authorized Context:
   - While booted into HyperOS, the hardware TEE has already validated the RoT and released master keys into authorized kernel data structures.
   - The MrR Companion service, executing with root privileges, extracts the active 128-character hex block key via `dmsetup table --showkeys userdata` and the CE master key descriptor.

2. Persistence in Non-Volatile Storage:
   - Extracted keys are written to `/mnt/vendor/persist/mrr/userdata.key` with restrictive permissions (`0600`).
   - The `persist` partition (`/dev/block/by-name/persist`) is an ext4 partition dedicated to hardware calibration, sensor data, and vendor flags. It is never wiped during Factory Reset / Wipe Data operations and is readily accessible during early recovery startup.

3. Autonomous Decryption in MeowFloprRecovery:
   - On startup, MrR mounts `/dev/block/by-name/persist` to `/mnt/vendor/persist`.
   - It reads `/mnt/vendor/persist/mrr/userdata.key`.
   - Configures the device-mapper target using `ioctl(DM_TABLE_LOAD)` with the `default-key` module.
   - Mounts the mapped block device `/dev/block/mapper/userdata` to `/data`.
   - Reads the CE master key and registers it in the Linux kernel keyring using:
     ```c
     ioctl(fd, FS_IOC_ADD_ENCRYPTION_KEY, &key_spec);
     ```
   - The Linux kernel instantly resolves directory trees: `/sdcard` folders (`Download`, `DCIM`, `Documents`) become readable as plaintext.

### 4. Subsystem Components

1. `MrRCompanion.apk`:
   - Lightweight standalone native Android application.
   - Provides a one-tap key synchronization button, hardware security state inspector, and boot service installer.

2. Magisk / KernelSU / APatch Module (`mrr-companion.zip`):
   - Installs a startup script at `/data/adb/modules/mrr-companion/service.sh`.
   - Automatically synchronizes updated keys to `/mnt/vendor/persist/mrr/` on every boot once `sys.boot_completed=1`.
   - Ensures transparent key updates across firmware updates and PIN modifications.

3. Embedded Assets in MrR Binary:
   - The Companion APK and Magisk module are embedded directly inside the compiled `mrr` binary via `include_bytes!`.
   - Can be exported from the recovery UI to `/sdcard/MRR/` or `/persist/` at any time with a single tap.

### 5. Manual Key Entry & Skip Fallbacks

If the device was formatted and the companion module was not yet installed:

- Enter the screen lock PIN for local key derivation via Scrypt/PBKDF2.
- Input a raw 128-character hexadecimal key via the on-screen keyboard.
- Skip decryption entirely ("Отмена / Cancel") to perform system partition repairs, flashing, or raw block backups.

### Feedback

For technical discussions or questions regarding storage cryptography, open an Issue or contact the developer on Telegram: https://t.me/kotvava.
