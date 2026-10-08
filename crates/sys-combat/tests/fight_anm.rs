// SPDX-License-Identifier: GPL-3.0-only
use std::ffi::c_void;
use sys_combat as _;

unsafe extern "C" {
    fn sh_fight_anm_decode(
        bytes: *const u8,
        size: usize,
        output: *mut c_void,
        poses: *mut u8,
        capacity: usize,
    ) -> bool;
}

#[test]
fn native_forward_parent_and_failure_atomicity() {
    let mut bytes = vec![0; 50];
    bytes[..2].copy_from_slice(&44u16.to_le_bytes());
    bytes[2] = 0;
    bytes[3] = 1;
    bytes[4..6].copy_from_slice(&3u16.to_le_bytes());
    bytes[6] = 4;
    bytes[12..16].copy_from_slice(&50u32.to_le_bytes());
    bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
    for i in 0..4 {
        bytes[20 + i * 6 + 1] = 255;
        bytes[20 + i * 6 + 2] = 255;
    }
    bytes[26] = 3; // Bone 1 refers forward, bone 3 refers to root.
    bytes[32] = 1;
    let mut header = [0u64; 5];
    let mut poses = [0u8; 24];
    // SAFETY: Aligned native descriptor and complete pose storage, all local.
    assert!(unsafe {
        sh_fight_anm_decode(
            bytes.as_ptr(),
            bytes.len(),
            header.as_mut_ptr().cast(),
            poses.as_mut_ptr(),
            4,
        )
    });
    assert_eq!(poses[6], 3);
    for case in 0..5 {
        let mut invalid = bytes.clone();
        match case {
            0 => invalid[38] = 1, // cycle
            1 => invalid[26] = 4, // outside graph
            2 => invalid[27] = 0, // missing rotation channel
            3 => invalid[16] = 3, // truncated frames
            _ => invalid[0] = 20, // pose/frame overlap
        }
        let saved_header = header;
        let saved_poses = poses;
        // SAFETY: Same local buffers; decoder validates before publishing.
        assert!(!unsafe {
            sh_fight_anm_decode(
                invalid.as_ptr(),
                invalid.len(),
                header.as_mut_ptr().cast(),
                poses.as_mut_ptr(),
                4,
            )
        });
        assert_eq!(header, saved_header);
        assert_eq!(poses, saved_poses);
    }
}
