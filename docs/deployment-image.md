# Server container image

`Dockerfile` собирает Landfall в два этапа:

1. `builder` использует pinned Rust Bookworm image и выполняет
   `cargo build --locked --release --bin landfall-server`.
2. `runtime` использует pinned `debian:bookworm-slim`, копирует только готовый
   binary и license-файл, а процесс запускается как UID/GID `65532`.

Так build tools, Cargo registry и исходники не попадают в runtime image. Это
уменьшает размер и поверхность атаки, а `--locked` не позволяет сборке
незаметно изменить версии зависимостей.

## Проверка

```bash
docker build --check .
docker build --tag landfall-server:local .
```

`docker build --check` проверяет синтаксис и общие ошибки Dockerfile. Полная
сборка дополнительно доказывает, что workspace компилируется внутри clean
builder stage. Образы pinned digest должны обновляться только отдельным
reviewed release change.
