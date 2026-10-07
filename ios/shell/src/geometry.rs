// SPDX-License-Identifier: GPL-3.0-only
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
}

pub fn circles(points: &[[f32; 2]], size: [f32; 2], radius: f32) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(points.len() * 48 * 3);
    let ndc = |p: [f32; 2]| Vertex {
        position: [p[0] / size[0] * 2.0 - 1.0, 1.0 - p[1] / size[1] * 2.0],
    };
    for &center in points {
        for i in 0..48 {
            let point = |n: u32| {
                let angle = n as f32 * std::f32::consts::TAU / 48.0;
                [
                    center[0] + radius * angle.cos(),
                    center[1] + radius * angle.sin(),
                ]
            };
            vertices.extend([ndc(center), ndc(point(i)), ndc(point(i + 1))]);
        }
    }
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_point_has_a_circle_and_pixel_radius_is_preserved() {
        let v = circles(&[[400.0, 200.0], [40.0, 80.0]], [800.0, 400.0], 20.0);
        assert_eq!(v.len(), 2 * 48 * 3);
        assert_eq!(v[0].position, [0.0, 0.0]);
        assert!((v[1].position[0] - 0.05).abs() < 1e-6);
        assert!((v[48 * 3].position[0] + 0.9).abs() < 1e-6);
        assert!(circles(&[], [800.0, 400.0], 20.0).is_empty());
    }
}
