## Сборка и запуск

В папке проекта выполните:

```bash
cmake -S . -B build
cmake --build build -j
./build/frglib
```

После изменения исходного кода пересоберите и запустите проект:

```bash
cmake --build build -j
./build/frglib
```