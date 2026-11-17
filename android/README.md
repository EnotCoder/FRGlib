# Android

Общий Gradle-проект для игр на FRGLib. Какая игра собирается, задаёт
`game.properties`; сам проект переиспользуется без правок.

```bash
./build.sh                 # собрать игру из game.properties
./build.sh --install       # установить на устройство и запустить
./build.sh --template      # проверить тулчейн на встроенном smoke-тесте
./build.sh --abi x86_64    # одна ABI, например для эмулятора
```

APK появится в `app/build/outputs/apk/debug/app-debug.apk`.

Скопируйте этот каталог в корень другого репозитория и поменяйте `crate`,
`assets`, `package` и `app_name` в `game.properties`. Пути считаются от
родителя `android/`.

Подробности: [`../docs/11-android.md`](../docs/11-android.md).

Не коммитьте сгенерированные `local.properties`, `.gradle/`, `app/build/`,
`jniLibs/` и `assets/` — корневой `.gitignore` их уже исключает.
