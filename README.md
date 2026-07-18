# Vain OS

Vain is a custom, experimental operating system kernel built entirely from scratch in Rust. It serves as a deep dive into low-level systems programming, exploring modern operating system design, x86_64 architecture, and the boundaries of memory safety in a kernel environment.

## Project Status

**Note:** This is a work-in-progress, experimental project. 

Vain is not a production-ready operating system and is not intended for daily use. It is a living experiment that undergoes frequent refactoring and rapid architectural shifts. Expect bugs, incomplete features, and breaking changes as development progresses.

## Core Features

While still in its early stages, Vain currently supports several fundamental OS capabilities:

- **Bootstrapping:** Integration with the Limine bootloader for early system initialization and Higher Half Direct Map (HHDM) memory mapping.
- **Memory Management:** Custom physical frame allocation and virtual memory paging.
- **Hardware Interrupts:** Full support for the Advanced Programmable Interrupt Controller (APIC), including the Local APIC and IOAPIC.
- **Device Drivers:** Early support for system timers and PS/2 Keyboard input via hardware interrupts.
- **Userspace Transition:** The ability to load ELF binaries and successfully transition from kernel mode (Ring 0) to userspace (Ring 3).
- **System Calls:** A rudimentary system call interface allowing userspace programs to communicate with the kernel.

## System Architecture

The project is broken down into modular components to keep the codebase clean and maintainable:

- **`kernel`**: The core x86_64 operating system kernel. It manages memory, handles hardware interrupts, and provides system call routing.
- **`userspace/init`**: The first userspace program executed by the kernel. It acts as the initial process (PID 1) to test Ring 3 execution and system calls.
- **`libs`**: A collection of shared libraries and utilities used across the project, including a custom ELF loader and a memory allocator.
- **`xtask`**: A custom build system wrapper. It automates compiling the kernel, building userspace applications, assembling the bootable ISO, and launching the emulator.

## Prerequisites

To build and run Vain, you will need the following tools installed on your system:

1. **Rust (Nightly):** The project relies on several unstable Rust features.
2. **QEMU:** Required to emulate the x86_64 hardware.
3. **xorriso:** Used by the build system to generate the bootable ISO image.

## Building and Running

The project uses `xtask` to simplify the build and run process. 

To compile the entire operating system, generate the ISO, and launch it inside QEMU, run the following command in the root of the repository:

```bash
cargo xtask run
```

If you only want to build the ISO without launching the emulator, you can run:

```bash
cargo xtask build
```

## Contributing

Because this is a personal, experimental project, contributions are currently not being actively solicited. However, the code is open for educational purposes, and you are welcome to fork the repository to experiment with your own kernel design.
