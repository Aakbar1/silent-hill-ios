// SPDX-License-Identifier: GPL-3.0-only
use std::process::ExitCode;

unsafe extern "C" {
    fn native_toolchain_pointer_bits() -> i32;
}

fn pointer_bits() -> i32 {
    // SAFETY: This linked C function has no parameters, side effects, or pointers.
    unsafe { native_toolchain_pointer_bits() }
}

fn main() -> ExitCode {
    if std::env::args_os().len() != 1 {
        eprintln!(
            "This compiler/linker probe does not accept game arguments. \
             Run cargo run -p silent-hill-boot for native boot. See docs/boot/READINESS.md."
        );
        return ExitCode::FAILURE;
    }
    let bits = pointer_bits();
    println!("Native C linked into Rust: {bits}-bit pointers.");
    if bits == 64 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_c_uses_64_bit_pointers() {
        assert_eq!(super::pointer_bits(), 64);
    }
}
