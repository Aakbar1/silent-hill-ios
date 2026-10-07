// Copyright (C) 2026 shdecompilations; SPDX-License-Identifier: GPL-3.0-only
// PORT: Enemy hierarchies can reference later bones (BIRD bone 16). Validate
// bounds and acyclicity instead of requiring serialized topological order.
use crate::{
    assets::DecodeError,
    gameplay::{Animation, BindPose},
};
pub fn decode_enemy_animation(bytes: &[u8]) -> Result<Animation<'_>, DecodeError> {
    let bad = |s: &str| DecodeError(s.into());
    let header = bytes.get(..20).ok_or_else(|| bad("truncated ANM header"))?;
    let half = |i| u16::from_le_bytes(header[i..i + 2].try_into().unwrap());
    let word = |i| u32::from_le_bytes(header[i..i + 4].try_into().unwrap());
    let data_offset = half(0);
    let frame_size = half(4);
    let keyframe_count = half(16);
    let bone_count = header[6];
    let file_size = word(12);
    if bone_count == 0 || header[18] > 30 || file_size as usize > bytes.len() || file_size < 20 {
        return Err(bad("invalid ANM counts/scale/size"));
    }
    let pose_end = 20 + usize::from(bone_count) * 6;
    if usize::from(data_offset) < pose_end
        || usize::from(frame_size) < usize::from(header[2]) * 9 + usize::from(header[3]) * 3
    {
        return Err(bad("ANM poses overlap frames or frame stride is too short"));
    }
    let poses = bytes
        .get(20..pose_end)
        .ok_or_else(|| bad("truncated bind poses"))?
        .as_chunks::<6>()
        .0
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let pose = BindPose {
                parent: p[0] as i8,
                rotation: p[1] as i8,
                translation: p[2] as i8,
                initial: [p[3] as i8, p[4] as i8, p[5] as i8],
            };
            // Root pose is deliberately not consumed by original Anim_BoneInit.
            if i > 0
                && (pose.parent < 0
                    || pose.parent as usize >= usize::from(bone_count)
                    || (pose.rotation >= 0 && pose.rotation as usize >= usize::from(header[2]))
                    || (pose.translation >= 0
                        && pose.translation as usize >= usize::from(header[3])))
            {
                return Err(bad("ANM parent/channel outside hierarchy"));
            }
            Ok(pose)
        })
        .collect::<Result<Vec<_>, _>>()?;
    for start in 1..poses.len() {
        let mut seen = vec![false; poses.len()];
        let mut current = start;
        while current != 0 {
            if seen[current] {
                return Err(bad("ANM parent cycle"));
            }
            seen[current] = true;
            current = poses[current].parent as usize;
        }
    }
    let end = usize::from(data_offset) + usize::from(frame_size) * usize::from(keyframe_count);
    if end > file_size as usize {
        return Err(bad("ANM frames exceed declared size"));
    }
    let frames = bytes
        .get(usize::from(data_offset)..end)
        .ok_or_else(|| bad("truncated ANM frames"))?;
    Ok(Animation {
        data_offset,
        rotation_count: header[2],
        translation_count: header[3],
        frame_size,
        bone_count,
        active_bones: word(8),
        file_size,
        keyframe_count,
        scale: header[18],
        root_y: header[19],
        poses,
        frames,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut v = vec![0; 48];
        v[0..2].copy_from_slice(&38u16.to_le_bytes());
        v[6] = 3;
        v[12..16].copy_from_slice(&48u32.to_le_bytes());
        for (i, parent) in [0, 2, 0].into_iter().enumerate() {
            v[20 + i * 6] = parent;
            v[21 + i * 6] = 255;
            v[22 + i * 6] = 255;
        }
        v
    }
    #[test]
    fn forward_hierarchy_is_valid() {
        assert!(decode_enemy_animation(&fixture()).is_ok());
    }
    #[test]
    fn cycles_self_links_and_out_of_bounds_are_rejected() {
        for (a, b) in [(2, 1), (1, 0), (3, 0), (255, 0)] {
            let mut v = fixture();
            v[26] = a;
            v[32] = b;
            assert!(decode_enemy_animation(&v).is_err());
        }
    }
    #[test]
    fn truncated_frames_and_overlapping_poses_are_rejected() {
        let v = fixture();
        for n in 0..v.len() {
            assert!(decode_enemy_animation(&v[..n]).is_err());
        }
        let mut v = fixture();
        v[0] = 37;
        assert!(decode_enemy_animation(&v).is_err());
        let mut v = fixture();
        v[4] = 20;
        v[16] = 1;
        assert!(decode_enemy_animation(&v).is_err());
    }
}
