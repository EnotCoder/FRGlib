## FRGLIB

![FRGLib](./logo.svg)

Крутая библиотека для Python, Rust и C++. Она вам может пригодиться для создания игры или программ с графикой.



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

## Запуск тестов (Python)

```bash
python3 examples/python/window.py 
```