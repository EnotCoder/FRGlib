## FRGLib

библиотека для создания игр на Python, Rust и C++.

## Подготовка окружения

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install "maturin[patchelf]"
```

## Сборка и запуск

```bash
maturin develop
python examples/python/window.py
```

`maturin develop` пересобирает расширение и ставит его в активный venv. После
изменения в `src/` достаточно запустить его же — отдельная пересборка не нужна.

## Wheel без зависимости от системного SDL3

По умолчанию линкуется системная SDL3, и maturin кладёт её копию в wheel. Если
нужна полностью самодостаточная сборка, включите фичу `bundled`: SDL3
собирается из исходников и линкуется статически, и wheel получается примерно
1.9 МБ без всяких внешних библиотек.

```bash
pip install maturin cmake
maturin build --release --features bundled --out dist
```

Нужны `cmake` и C-компилятор. Проверено: `ldd` на установленном `.so` не
показывает `libSDL3` вообще.

Оговорка: раз SDL3 влинкована статически, maturin не может определить
manylinux-тег и ставит общий `linux_x86_64`. Для загрузки на PyPI такой wheel
не подойдёт — потребуется `auditwheel repair`.