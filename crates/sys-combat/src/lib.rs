// SPDX-License-Identifier: GPL-3.0-only
// The unused core graph helpers are retained so the harness shares the same ABI.
#[allow(dead_code)]
pub mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}
#[allow(dead_code)]
pub mod gameplay {
    include!(concat!(env!("OUT_DIR"), "/gameplay.rs"));
}
pub mod animation;
#[allow(dead_code)]
pub mod maps {
    include!(concat!(env!("OUT_DIR"), "/maps.rs"));
    /// Owns aligned collision leaves independently of imported immutable bytes.
    pub struct CombatCollision {
        pub header: Box<NativeCollision>,
        _storage: CollisionStorage,
    }
    impl CombatCollision {
        pub fn new(collision: &crate::assets::Collision<'_>) -> Self {
            let mut storage = CollisionStorage::new(collision);
            let header = Box::new(storage.header(collision));
            Self {
                header,
                _storage: storage,
            }
        }
    }
}
static FIXTURE: std::sync::Mutex<()> = std::sync::Mutex::new(());
unsafe extern "C" {
    fn sh_combat_test_case(index: i32) -> i32;
}
/// Runs an isolated synthetic C fixture and returns zero or its failing C line.
pub fn run_fixture(index: i32) -> i32 {
    let _guard = FIXTURE.lock().expect("fixture mutex");
    // SAFETY: the fixture has no pointer arguments and exclusive global access.
    unsafe { sh_combat_test_case(index) }
}
