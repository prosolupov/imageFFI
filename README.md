# image_ffi

Проект демонстрирует обработку PNG-изображений через динамически загружаемые плагины (`cdylib`) в Rust.

## Состав проекта

- `image_processor` — бинарный крейт (CLI), который:
1. читает PNG;
2. декодирует в `RGBA8`;
3. загружает плагин из динамической библиотеки;
4. передает буфер пикселей в плагин;
5. сохраняет результат в PNG.
- `mirror_plugin` — плагин зеркального разворота.
- `blur_plugin` — плагин размытия.

## Сборка

```bash
cargo build
```

После сборки библиотеки плагинов находятся в `target/debug`:
- Linux: `libmirror_plugin.so`, `libblur_plugin.so`
- macOS: `libmirror_plugin.dylib`, `libblur_plugin.dylib`
- Windows: `mirror_plugin.dll`, `blur_plugin.dll`

## Запуск

```bash
cargo run -p image_processor -- \
  <input.png> <output.png> <plugin_name> <params_file> \
  --plugin-path <plugins_dir>
```

Логирование включено через `log + env_logger`. Уровень логов задаётся переменной `RUST_LOG`:

```bash
RUST_LOG=info cargo run -p image_processor -- \
  <input.png> <output.png> <plugin_name> <params_file> \
  --plugin-path <plugins_dir>
```

Аргументы:
- `input` — путь к исходному PNG;
- `output` — путь для сохранения результата;
- `plugin` — имя плагина без расширения (например, `mirror_plugin` или `blur_plugin`);
- `params` — путь к текстовому файлу с параметрами;
- `--plugin-path` — директория с библиотеками плагинов (обязательный аргумент).

## API плагина

Каждый плагин экспортирует:

```c
int32_t process_image(
    uint32_t width,
    uint32_t height,
    uint8_t* rgba_data,
    const char* params
);
```

- `rgba_data` — массив длиной `width * height * 4`.
- Плагин модифицирует буфер на месте.
- Возвращаемые коды: `0` — успех, `1` — некорректные параметры, `2` — некорректные входные данные.

## Формат params

Можно использовать простой текст вида `key=value`, `key:value`, разделители: запятая, `;` или новая строка.

Параметры валидируются строго:
- неизвестные ключи считаются ошибкой;
- некорректные значения считаются ошибкой;
- при ошибке плагин возвращает код ошибки, а `image_processor` завершает работу без записи выходного PNG.

Примеры:

### mirror_plugin

`params_mirror.txt`:

```txt
horizontal=true
vertical=false
```

Поддерживаемые значения `bool`: `true/false`, `1/0`, `yes/no`, `on/off`.

### blur_plugin

`params_blur.txt`:

```txt
radius=2
iterations=3
```

- `radius` — радиус размытия (`>= 1`);
- `iterations` — количество проходов (`>= 1`).

## Примеры запуска

Зеркало:

```bash
cargo run -p image_processor -- \
  input.png output_mirror.png mirror_plugin params_mirror.txt \
  --plugin-path <plugins_dir>
```

Размытие:

```bash
cargo run -p image_processor -- \
  input.png output_blur.png blur_plugin params_blur.txt \
  --plugin-path <plugins_dir>
```

## Готовые demo-файлы в репозитории

В репозитории уже есть готовые картинки и файлы параметров:

- картинки: `examples/images/input.png`;
- параметры зеркала: `examples/params/mirror_horizontal.txt`, `examples/params/mirror_vertical.txt`;
- параметры размытия: `examples/params/blur_soft.txt`, `examples/params/blur_strong.txt`.

Пример запуска с ними:

```bash
cargo build -p mirror_plugin -p blur_plugin

cargo run -p image_processor -- \
  examples/images/input.png examples/output_mirror.png \
  mirror_plugin examples/params/mirror_horizontal.txt \
  --plugin-path target/debug

cargo run -p image_processor -- \
  examples/images/input.png examples/output_blur.png \
  blur_plugin examples/params/blur_soft.txt \
  --plugin-path target/debug
```

## Обработка ошибок

`image_processor` проверяет:
- существование `input`, `params` и файла библиотеки плагина;
- что входной файл действительно PNG;
- ошибки декодирования/кодирования изображения;
- корректность длины RGBA-буфера (`width * height * 4`) перед вызовом плагина.

Ошибки описаны типизированно в `image_processor/src/error.rs` (`AppError`).
