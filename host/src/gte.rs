// SPDX-License-Identifier: GPL-3.0-only
//! PsyQ/COP2 bridge. All integer math is the existing psxgpu native C GTE.
use psxgpu::gte::Gte;
use std::{cell::RefCell, collections::HashMap};

#[derive(Default)]
struct Context {
    exact: Gte,
    fifo: [Option<[f32; 2]>; 3],
    positions: HashMap<usize, (u32, [f32; 2])>,
}
thread_local! {static GTE: RefCell<Context> = RefCell::new(Context::default());}

#[unsafe(no_mangle)]
extern "C" fn port_gte_reset() {
    GTE.with(|c| *c.borrow_mut() = Context::default());
}
#[unsafe(no_mangle)]
extern "C" fn port_gte_read_data(reg: u32) -> u32 {
    GTE.with(|c| {
        c.borrow_mut()
            .exact
            .read_data(reg.try_into().expect("COP2 register"))
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_gte_read_control(reg: u32) -> u32 {
    GTE.with(|c| {
        c.borrow()
            .exact
            .read_control(reg.try_into().expect("COP2 register"))
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_gte_write_data(reg: u32, value: u32) {
    GTE.with(|c| {
        let mut c = c.borrow_mut();
        c.exact
            .write_data(reg.try_into().expect("COP2 register"), value);
        match reg {
            12..=14 => c.fifo[(reg - 12) as usize] = None,
            15 => c.fifo = [c.fifo[1], c.fifo[2], None],
            _ => {}
        }
    });
}
#[unsafe(no_mangle)]
extern "C" fn port_gte_write_control(reg: u32, value: u32) {
    GTE.with(|c| {
        c.borrow_mut()
            .exact
            .write_control(reg.try_into().expect("COP2 register"), value)
    });
}
#[unsafe(no_mangle)]
extern "C" fn port_gte_execute(opcode: u32) -> i32 {
    GTE.with(|c| {
        let mut c = c.borrow_mut();
        // PORT: Float positions are presentation metadata; never feed them into
        // integer COP2 registers, flags, collision or game logic.
        if cfg!(feature = "precise-vertices") {
            match opcode & 63 {
                1 => c.fifo = [c.fifo[1], c.fifo[2], c.exact.project_precise(0).ok()],
                48 => c.fifo = std::array::from_fn(|i| c.exact.project_precise(i).ok()),
                _ => {}
            }
        }
        i32::from(c.exact.execute(opcode).is_err())
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_gte_store_sxy(destination: *mut u8, reg: u32) {
    GTE.with(|c| {
        let mut c = c.borrow_mut();
        let word = c.exact.read_data(reg.try_into().expect("COP2 register"));
        // SAFETY: PsyQ's store helper supplies a writable four-byte coordinate.
        unsafe { destination.cast::<u32>().write_unaligned(word) };
        if cfg!(feature = "precise-vertices") {
            let index = (reg.min(14) - 12) as usize;
            if let Some(position) = c.fifo[index] {
                c.positions.insert(destination as usize, (word, position));
            } else {
                c.positions.remove(&(destination as usize));
            }
        }
    });
}

/// Consume metadata only at known polygon coordinate offsets. Reused packet
/// addresses with changed coordinate words cannot inherit stale metadata.
pub(crate) fn packet_positions(base: *const u32, words: &[u32]) -> Option<Vec<[f32; 2]>> {
    if !cfg!(feature = "precise-vertices") || words.len() < 2 {
        return None;
    }
    let command = words[1] >> 24;
    if command & 0xe0 != 0x20 {
        return None;
    }
    let count = if command & 8 != 0 { 4 } else { 3 };
    let stride = 1 + usize::from(command & 4 != 0) + usize::from(command & 16 != 0);
    GTE.with(|c| {
        let mut c = c.borrow_mut();
        (0..count)
            .map(|i| {
                let offset = 2 + i * stride;
                let word = *words.get(offset)?;
                let (expected, position) = c.positions.remove(&(base as usize + offset * 4))?;
                (expected == word).then_some(position)
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/native-source/gte_commands.rs"));
    unsafe extern "C" {
        fn port_gte_bridge_probe(output: *mut u32) -> i32;
        fn port_gte_command_probe(index: u32, output: *mut u32) -> i32;
    }
    #[test]
    fn psyq_macros_and_functions_reach_the_exact_engine() {
        let mut output = [0; 8];
        // SAFETY: Probe writes eight u32 words, no game/global state involved.
        assert_eq!(unsafe { port_gte_bridge_probe(output.as_mut_ptr()) }, 1);
        assert_eq!(output[..4], [0x0032_0064, 4096, 500, 0x1000]);
        assert_eq!(output[4..7], [1000, 500, 2000]);
        assert_eq!(output[7], 1); // Unsupported opcode rejected.
    }
    #[test]
    fn contexts_are_worker_local() {
        port_gte_reset();
        port_gte_write_control(5, 123);
        std::thread::spawn(|| {
            assert_eq!(port_gte_read_control(5), 0);
            port_gte_write_control(5, 456);
        })
        .join()
        .unwrap();
        assert_eq!(port_gte_read_control(5), 123);
    }
    #[test]
    fn every_sdk_command_matches_direct_cop2_execution() {
        let mut covered = std::collections::HashSet::new();
        for (index, opcode) in COMMANDS.iter().copied().enumerate() {
            covered.insert(opcode & 63);
            let mut exact = Gte::new();
            port_gte_reset();
            for reg in 0..32u8 {
                let value = 0x0300_0123u32.wrapping_mul(u32::from(reg) + 1);
                exact.write_data(reg, value);
                exact.write_control(reg, value);
                port_gte_write_data(u32::from(reg), value);
                port_gte_write_control(u32::from(reg), value);
            }
            exact.execute(opcode).unwrap();
            let mut output = [0; 64];
            // SAFETY: Only worker-local GTE state and this 64-word output are used.
            assert_eq!(
                unsafe { port_gte_command_probe(index as u32, output.as_mut_ptr()) },
                0
            );
            for reg in 0..32u8 {
                assert_eq!(
                    output[usize::from(reg)],
                    exact.read_data(reg),
                    "opcode {opcode:#x} data {reg}"
                );
                assert_eq!(
                    output[32 + usize::from(reg)],
                    exact.read_control(reg),
                    "opcode {opcode:#x} control {reg}"
                );
            }
        }
        assert_eq!(covered.len(), 22);
    }
}
