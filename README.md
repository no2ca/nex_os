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
    - Global allocator
- Process
    - User mode process
    - Round-robbin scheduler
    - Context switch (Struct Based)
    - ELF loader
    - Idle process
    - Process states (Running)
    - Process listing (ps)
    - Process creation/yield/exit syscalls
- Trap
    - S-mode Trap Handler
    - SBI console output
- Shell
    - Built-in commands (help, exit, yield, ps, sh)
    - Command history navigation (up/down)
    - Backspace handling and ASCII input validation
- VFS
    - In-memory filesystem (MemoryFs/MemoryNode)
- Timer
    - read_time helpers
- Logging
    - log macros
- Test
    - Basic shell test runner
