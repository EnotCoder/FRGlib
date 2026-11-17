# 11. Android

FRGLib можно собрать под Android без отдельной ветки кода. Игра остаётся
обычным Rust-циклом по `Window`, а `android/` содержит один переиспользуемый
Gradle-проект: какой игре его собрать, решает `android/game.properties`.

Поддерживается путь **SDL3 + общие `.so`**:

```text
Rust cdylib (libmain.so) -> libSDL3.so + libSDL3_ttf.so -> Gradle -> APK
```

Статическая линковка SDL в этом сценарии не подходит: Java-слой SDL вызывает
`System.loadLibrary("SDL3")` до загрузки игры, поэтому SDL3 и SDL3_ttf должны
лежать в APK отдельными библиотеками.

## Что нужно

| Компонент | Зачем |
|---|---|
| Rust + `cargo-ndk` | `rustup target add aarch64-linux-android x86_64-linux-android`, затем `cargo install cargo-ndk` |
| Android SDK | `platform-tools`, `build-tools`, `platforms;android-35` |
| Android NDK | r28+; скрипт ищет установленную версию сам |
| JDK 17 | Gradle и AGP 8.7.3 |

`build.sh` ищет SDK в `ANDROID_HOME`, `ANDROID_SDK_ROOT` и стандартных путях.
NDK можно задать через `ANDROID_NDK_HOME`; иначе берётся самая новая версия из
`$ANDROID_HOME/ndk`.

## Быстрый старт

```bash
# Собрать игру из android/game.properties
./android/build.sh

# То же и сразу установить на подключённое устройство/эмулятор
./android/build.sh --install

# Только под эмулятор, в debug-профиле
./android/build.sh --abi x86_64 --debug

# Проверить саму цепочку на встроенном smoke-тесте
./android/build.sh --template --install
```

Готовый APK: `android/app/build/outputs/apk/debug/app-debug.apk`. Он подписан
debug-ключом; настройка release-подписи и AAB в этот гайд не входят.

`--template` игнорирует `crate`, `assets`, `package`, `app_name` и
`orientation` из `game.properties`, поэтому полезен как первая проверка
тулчейна: если шаблон запускается, проблема в игре, а не в Android-сборке.

## `game.properties`

```properties
crate=examples/2d_games/space_android
assets=examples/2d_games/assets
package=org.frglib.space
app_name=Space
orientation=landscape
abis=arm64-v8a,x86_64
min_sdk=24
ndk_version=auto
```

| Ключ | Смысл |
|---|---|
| `crate` | Rust-крейт с `crate-type = ["cdylib"]`; путь от корня репозитория |
| `assets` | Каталог, который целиком копируется в `assets/` APK |
| `package` | `applicationId` и Java-пакет приложения |
| `app_name` | Подпись под иконкой |
| `orientation` | `landscape`, `portrait` или `unspecified` |
| `abis` | Список ABI через запятую |
| `min_sdk` | `minSdk`; SDL3 требует минимум 21, здесь по умолчанию 24 |
| `ndk_version` | `auto` или конкретная версия из `$ANDROID_HOME/ndk` |

`build.sh` дополнительно принимает `--abi`, `--debug`, `--release`,
`--install`, `--template` и переменную `GRADLE_TASK`.

## Как подключить свою игру

1. Добавьте в крейт игры cdylib с именем `main` и экспортом `SDL_main`:

   ```toml
   [lib]
   name = "main"
   crate-type = ["cdylib"]

   [dependencies]
   sdl3 = { version = "0.20", features = ["ttf", "build-from-source"] }
   # Java-обвязка в android/app/src/main/java рассчитана на SDL 3.4.20.
   sdl3-sys = "=0.7.3"

   [profile.release]
   panic = "abort"
   ```

   ```rust
   #[no_mangle]
   pub unsafe extern "C" fn SDL_main(_argc: i32, _argv: *mut *mut c_char) -> i32 {
       match game::run() {
           Ok(()) => 0,
           Err(err) => 1,
       }
   }
   ```

   `#[sdl3_main::main]` не подходит: макрос создаёт локальный `main`, а
   `SDLActivity` ищет символ именно с именем `SDL_main` через `dlsym`.

2. Скопируйте каталог `android/` в корень своего репозитория — `build.sh`
   считает пути `crate` и `assets` от родителя `android/`.

3. Отредактируйте `android/game.properties` и, если нужно,
   `android/app/src/main/res/` с иконкой и темой.

4. Соберите:

   ```bash
   ./android/build.sh --install
   ```

Пример `examples/2d_games/space_android` показывает минимальную обёртку: он не
дублирует игру, а подключает те же файлы через `#[path]` и вызывает
`game::run()`.

## Пути к ассетам

На десктопе примеры строят путь от `CARGO_MANIFEST_DIR`. В APK такого пути
нет, зато SDL I/O умеет читать файлы из `assets/`. Поэтому `space` использует
разные корни:

```rust
#[cfg(target_os = "android")]
const ASSET_ROOT: &str = ""; // путь внутри APK, например "space/planets/earth.png"

#[cfg(not(target_os = "android"))]
const ASSET_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/2d_games/assets");
```

`Image::from_file` и `TtfContext::load_font` идут через SDL I/O, поэтому
относительные пути работают и в APK. Обычный `std::fs` — нет.

## Касания

`Window::poll()` переводит SDL-события пальцев в те же координаты, что и мышь:

```rust
Event::FingerDown { id, x, y, pressure }
Event::FingerMotion { id, x, y, dx, dy, pressure }
Event::FingerUp { id, x, y, pressure }
Event::FingerCanceled { id, x, y }
```

- `id` постоянен, пока палец касается экрана;
- координаты — в пикселях окна, не нормализованные `0..1`;
- `FingerCanceled` приходит, когда система забрала жест (уведомление, звонок,
  потеря фокуса). Его нужно обрабатывать как отпускание **без клика**.

По умолчанию SDL синтезирует из касаний мышь. Если игра разбирает `Finger*`
сама, синтез лучше выключить, иначе один тап придёт дважды:

```rust
let mut window = Window::builder("game")
    .touch_as_mouse(false)
    .build()?;
```

Мышь при этом продолжает работать: на Android это актуально для эмулятора,
Chromebook и планшетов с внешней мышью.

## Плотность экрана

Для интерфейса используйте `Window::display_scale()`, а не
`Window::pixel_density()`:

```rust
let scale = window.display_scale().clamp(1.0, 3.0);
let font = ttf.load_font(path, FONT_SIZE * scale)?;
```

На Android `pixel_density()` всегда возвращает `1.0`: размер окна и drawable
совпадают, а реальная плотность приходит из системы. `display_scale()`
учитывает её и на эмуляторе с `wm density 240` даёт `1.5`.

Масштабируйте шрифты, отступы, панели и радиусы попадания, но **не**
координаты событий: они уже в пикселях окна.

## Ориентация и кнопка «назад»

Манифест получает ориентацию из `game.properties`, но SDLActivity для
resizable-окна переопределяет её runtime-хинтом. Чтобы значения не
разъехались, `build.sh` экспортирует `FRGLIB_ORIENTATION`, а Android-крейт
ставит хинт до создания окна:

```rust
sdl3::hint::set("SDL_ORIENTATIONS", "LandscapeLeft LandscapeRight");
```

Кнопка «назад» по умолчанию сразу закрывает Activity. Хинт
`SDL_ANDROID_TRAP_BACK_BUTTON=1` доставляет её игре как
`Keycode::AcBack` — тогда игра сама решает, закрыть панель, снять выбор или
выйти. В `space` `AcBack` ведёт себя как `Escape`.

## 16 КБ страницы

Android 15 требует, чтобы нативные библиотеки были выровнены на 16 КБ.
`build.sh` передаёт линкеру `-Wl,-z,max-page-size=16384`; NDK r28+ делает это
и сам. Проверка:

```bash
llvm-readelf -l android/app/src/main/jniLibs/arm64-v8a/libmain.so | grep LOAD
# выравнивание должно быть 0x4000
```

## Ограничения

| Ограничение | Детали |
|---|---|
| Только SDL-окно | `WgpuWindow` под Android в этой сборке не подключён |
| Нет release-подписи | APK подписан debug-ключом; keystore и AAB настраиваются отдельно |
| Нет CI | Сборка рассчитана на локальный SDK/NDK |
| Только Android | iOS и другие платформы вне этого гайда |
| Общие SDL-библиотеки | Статическая линковка SDL несовместима с `System.loadLibrary("SDL3")` |

## Отладка

```bash
adb logcat | grep -E "SDL|rust|panic"
adb shell wm size
adb shell wm density        # 240 -> display_scale 1.5
adb shell wm density reset
```

Если приложение сразу закрывается, чаще всего причина одна из двух:

- версия `sdl3-sys` не совпадает с Java-обвязкой (SDLActivity проверяет
  версию при старте);
- в `libmain.so` нет символа `SDL_main`.
