use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("help");

    match subcommand {
        "build" => build(),
        "run" => run(&args[1..]),
        "test" => test(),
        "fmt-check" => fmt_check(),
        "clippy" => clippy(),
        _ => print_usage(),
    }
}

fn print_usage() {
    eprintln!("Usage: cargo xtask <command>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  build              Build kernel and assemble bootable ISO");
    eprintln!("  run [--debug]      Build and launch in QEMU");
    eprintln!("  test               Run host-side unit tests");
    eprintln!("  fmt-check          Check formatting");
    eprintln!("  clippy             Run clippy lints");
    process::exit(1);
}

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
        eprintln!("CARGO_MANIFEST_DIR not set");
        process::exit(1);
    }));
    manifest_dir
        .parent()
        .expect("xtask must be inside workspace")
        .to_path_buf()
}

fn target_spec_path(root: &Path) -> PathBuf {
    root.join("target-specs").join("x86_64-vain.json")
}

fn kernel_binary_path(root: &Path) -> PathBuf {
    root.join("target")
        .join("x86_64-vain")
        .join("debug")
        .join("vain-kernel")
}

fn run_checked(command: &mut Command) {
    let status = command.status().unwrap_or_else(|err| {
        eprintln!("Failed to execute {:?}: {}", command, err);
        process::exit(1);
    });
    if !status.success() {
        eprintln!("Command {:?} exited with {}", command, status);
        process::exit(status.code().unwrap_or(1));
    }
}

fn build() {
    let root = workspace_root();
    let target_spec = target_spec_path(&root);

    eprintln!("[xtask] Building kernel...");
    run_checked(
        Command::new("cargo")
            .args(["build", "--manifest-path"])
            .arg(root.join("kernel").join("Cargo.toml").to_str().unwrap())
            .arg("--target")
            .arg(target_spec.to_str().unwrap())
            .args([
                "-Zbuild-std=core,alloc,compiler_builtins",
                "-Zbuild-std-features=compiler-builtins-mem",
                "-Zjson-target-spec",
            ]),
    );

    let kernel_elf = kernel_binary_path(&root);
    if !kernel_elf.exists() {
        eprintln!("[xtask] Kernel binary not found at {:?}", kernel_elf);
        process::exit(1);
    }

    eprintln!("[xtask] Building userspace/init...");
    run_checked(
        Command::new("cargo")
            .env(
                "RUSTFLAGS",
                "-C link-arg=-Tuserspace/init/linker.ld -C relocation-model=static",
            )
            .args(["build", "--manifest-path"])
            .arg(
                root.join("userspace")
                    .join("init")
                    .join("Cargo.toml")
                    .to_str()
                    .unwrap(),
            )
            .arg("--target")
            .arg("x86_64-unknown-none"),
    );

    eprintln!("[xtask] Building userspace/drivers/ps2-keyboard...");
    run_checked(
        Command::new("cargo")
            .env(
                "RUSTFLAGS",
                "-C link-arg=-Tuserspace/init/linker.ld -C relocation-model=static",
            )
            .args(["build", "--manifest-path"])
            .arg(
                root.join("userspace")
                    .join("drivers")
                    .join("ps2-keyboard")
                    .join("Cargo.toml")
                    .to_str()
                    .unwrap(),
            )
            .arg("--target")
            .arg("x86_64-unknown-none"),
    );

    assemble_iso(
        &root,
        &kernel_elf,
        &root
            .join("target")
            .join("x86_64-unknown-none")
            .join("debug")
            .join("init"),
        &root
            .join("target")
            .join("x86_64-unknown-none")
            .join("debug")
            .join("ps2-keyboard"),
    );
}

fn assemble_iso(root: &Path, kernel_elf: &Path, init_elf: &Path, ps2_keyboard_elf: &Path) {
    eprintln!("[xtask] Assembling bootable ISO...");

    let iso_root = root.join("iso_root");
    let _ = fs::remove_dir_all(&iso_root);

    let boot_dir = iso_root.join("boot");
    let limine_dir = boot_dir.join("limine");
    let efi_boot_dir = iso_root.join("EFI").join("BOOT");

    fs::create_dir_all(&limine_dir).expect("create iso_root/boot/limine");
    fs::create_dir_all(&efi_boot_dir).expect("create iso_root/EFI/BOOT");

    fs::copy(kernel_elf, boot_dir.join("kernel")).expect("copy kernel binary");
    fs::copy(init_elf, boot_dir.join("init")).expect("copy init binary");
    fs::copy(ps2_keyboard_elf, boot_dir.join("ps2-keyboard")).expect("copy ps2-keyboard binary");
    fs::copy(root.join("limine.conf"), limine_dir.join("limine.conf")).expect("copy limine.conf");

    let limine_share = find_limine_share();

    let limine_files = [
        ("limine-bios-cd.bin", limine_dir.as_path()),
        ("limine-bios.sys", limine_dir.as_path()),
        ("limine-uefi-cd.bin", limine_dir.as_path()),
        ("BOOTX64.EFI", efi_boot_dir.as_path()),
    ];

    for (filename, destination) in &limine_files {
        let source = limine_share.join(filename);
        if !source.exists() {
            eprintln!("[xtask] Warning: Limine file not found: {:?}", source);
            continue;
        }
        fs::copy(&source, destination.join(filename))
            .unwrap_or_else(|_| panic!("copy {}", filename));
    }

    let iso_path = root.join("vain.iso");

    run_checked(
        Command::new("xorriso")
            .args(["-as", "mkisofs"])
            .arg("-b")
            .arg("boot/limine/limine-bios-cd.bin")
            .args([
                "-no-emul-boot",
                "-boot-load-size",
                "4",
                "-boot-info-table",
                "--efi-boot",
            ])
            .arg("boot/limine/limine-uefi-cd.bin")
            .args([
                "-efi-boot-part",
                "--efi-boot-image",
                "--protective-msdos-label",
            ])
            .arg(iso_root.to_str().unwrap())
            .arg("-o")
            .arg(iso_path.to_str().unwrap()),
    );

    run_checked(
        Command::new("limine")
            .args(["bios-install"])
            .arg(iso_path.to_str().unwrap()),
    );

    let _ = fs::remove_dir_all(&iso_root);

    eprintln!("[xtask] ISO ready: {:?}", iso_path);
}

fn find_limine_share() -> PathBuf {
    let candidates = [
        PathBuf::from("/opt/homebrew/share/limine"),
        PathBuf::from("/usr/local/share/limine"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }
    eprintln!("[xtask] Cannot find Limine share directory.");
    eprintln!("[xtask] Install via: brew install limine");
    process::exit(1);
}

fn find_ovmf_firmware() -> PathBuf {
    let candidates = [
        PathBuf::from("/opt/homebrew/share/qemu/edk2-x86_64-code.fd"),
        PathBuf::from("/opt/homebrew/Cellar/qemu/11.0.2/share/qemu/edk2-x86_64-code.fd"),
        PathBuf::from("/usr/local/share/qemu/edk2-x86_64-code.fd"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    if let Ok(output) = Command::new("find")
        .args([
            "/opt/homebrew",
            "-name",
            "edk2-x86_64-code.fd",
            "-type",
            "f",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            let path = PathBuf::from(line.trim());
            if path.exists() {
                return path;
            }
        }
    }

    eprintln!("[xtask] Cannot find OVMF firmware (edk2-x86_64-code.fd).");
    eprintln!("[xtask] Install via: brew install qemu");
    process::exit(1);
}

fn run(extra_args: &[String]) {
    build();

    let root = workspace_root();
    let iso_path = root.join("vain.iso");
    let ovmf = find_ovmf_firmware();
    let debug_mode = extra_args.iter().any(|a| a == "--debug");

    eprintln!("[xtask] Launching QEMU...");
    if debug_mode {
        eprintln!("[xtask] Debug mode: QEMU paused, waiting for GDB on :1234");
    }

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.args(["-M", "q35"])
        .arg("-drive")
        .arg(format!(
            "if=pflash,unit=0,format=raw,file={},readonly=on",
            ovmf.to_str().unwrap()
        ))
        .args(["-cdrom"])
        .arg(iso_path.to_str().unwrap())
        .args(["-m", "256M"])
        .args(["-serial", "stdio"])
        .args(["-no-reboot", "-no-shutdown"]);

    if debug_mode {
        qemu.args(["-s", "-S"]);
    }

    run_checked(&mut qemu);
}

fn test() {
    let root = workspace_root();
    eprintln!("[xtask] Running host-side unit tests...");
    run_checked(
        Command::new("cargo")
            .arg("test")
            .arg("--manifest-path")
            .arg(
                root.join("libs")
                    .join("abi")
                    .join("Cargo.toml")
                    .to_str()
                    .unwrap(),
            ),
    );
    run_checked(
        Command::new("cargo")
            .arg("test")
            .arg("--manifest-path")
            .arg(
                root.join("libs")
                    .join("elf")
                    .join("Cargo.toml")
                    .to_str()
                    .unwrap(),
            ),
    );
    run_checked(
        Command::new("cargo")
            .arg("test")
            .arg("--manifest-path")
            .arg(
                root.join("libs")
                    .join("allocator")
                    .join("Cargo.toml")
                    .to_str()
                    .unwrap(),
            ),
    );
    eprintln!("[xtask] All tests passed.");
}

fn fmt_check() {
    let root = workspace_root();
    eprintln!("[xtask] Checking formatting...");
    run_checked(
        Command::new("cargo")
            .arg("fmt")
            .arg("--all")
            .arg("--check")
            .current_dir(&root),
    );
    eprintln!("[xtask] Formatting OK.");
}

fn clippy() {
    let root = workspace_root();

    eprintln!("[xtask] Running clippy on kernel...");
    let target_spec = target_spec_path(&root);
    run_checked(
        Command::new("cargo")
            .arg("clippy")
            .arg("--manifest-path")
            .arg(root.join("kernel").join("Cargo.toml").to_str().unwrap())
            .arg("--target")
            .arg(target_spec.to_str().unwrap())
            .args([
                "-Zbuild-std=core,alloc,compiler_builtins",
                "-Zbuild-std-features=compiler-builtins-mem",
                "-Zjson-target-spec",
            ])
            .args(["--", "-D", "warnings"]),
    );

    eprintln!("[xtask] Running clippy on libs...");
    for lib_crate in &["abi", "elf", "allocator"] {
        run_checked(
            Command::new("cargo")
                .arg("clippy")
                .arg("--manifest-path")
                .arg(
                    root.join("libs")
                        .join(lib_crate)
                        .join("Cargo.toml")
                        .to_str()
                        .unwrap(),
                )
                .args(["--", "-D", "warnings"]),
        );
    }

    eprintln!("[xtask] Running clippy on xtask...");
    run_checked(
        Command::new("cargo")
            .arg("clippy")
            .arg("--manifest-path")
            .arg(root.join("xtask").join("Cargo.toml").to_str().unwrap())
            .args(["--", "-D", "warnings"]),
    );

    eprintln!("[xtask] Clippy clean.");
}
