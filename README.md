# nex_os

Nonica's EXperimental Operating System

Currently WIP.

## Run on Docker

> [!warning]
> The current configuration takes a long time to build.

```bash
docker build -t nex_os .
docker run --rm -it -v $(pwd):/work nex_os ./run.sh
```

## Implemented Features

- Memory
    - Page allocator (bump allocator)
    - SV39 paging
- Process
    - User mode process
    - Round-robbin scheduler
    - Context switch (Struct Based)
    - ELF loader
- Trap
    - S-mode Trap Handler
    - SBI console output
