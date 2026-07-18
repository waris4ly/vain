# Vain OS

An experimental, from-scratch x86_64 operating system kernel written in Rust.

**Note:** This is a work-in-progress, experimental project. Expect instability and rapid changes.

## Features
- Custom physical and virtual memory manager
- APIC (Local APIC & IOAPIC) integration
- Hardware interrupts (Timer & PS/2 Keyboard)
- Ring 3 userspace transition and early syscalls

## Build and Run
Requires Rust Nightly, QEMU, and xorriso.
```bash
cargo xtask run
```
