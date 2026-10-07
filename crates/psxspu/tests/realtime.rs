//! Allocation instrumentation lives in this test executable, not the mixer.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
struct Counted;
thread_local! {static ENABLED:Cell<bool>=const {Cell::new(false)};static ALLOCATIONS:Cell<usize>=const {Cell::new(0)};}
// SAFETY: every allocation is delegated to the system allocator with the
// original pointer/layout. Thread-local counters neither allocate nor alias.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ENABLED.with(|e| {
            if e.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        // SAFETY: preserve GlobalAlloc's layout contract unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: preserve GlobalAlloc's pointer/layout contract unchanged.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ENABLED.with(|e| {
            if e.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        // SAFETY: preserve GlobalAlloc's pointer/layout/size contract unchanged.
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;
fn tracked(work: impl FnOnce()) {
    ALLOCATIONS.with(|n| n.set(0));
    ENABLED.with(|e| e.set(true));
    work();
    ENABLED.with(|e| e.set(false));
    assert_eq!(
        ALLOCATIONS.with(Cell::get),
        0,
        "allocation on the realtime path"
    );
}
#[test]
fn mixer_allocates_nothing_with_all_voices_and_reverb() {
    use psxspu::{Spu, reverb::Preset, sdk::VoiceAttr};
    let mut s = Spu::new();
    s.init();
    s.set_reverb_preset(Preset::Hall, true);
    s.set_reverb(true);
    s.set_reverb_voice(true, 0xffffff);
    let mut block = [0x73; 16];
    block[0] = 0x24;
    block[1] = 7;
    s.upload(0x1010, &block).unwrap();
    for i in 0..24 {
        s.key_on_with_attr(&VoiceAttr {
            voices: 1 << i,
            mask: 0x6009f,
            volume: [512; 2],
            pitch: 3000 + i * 10,
            address: 0x1010,
            adsr1: 15,
            adsr2: 0x1fc0,
            ..VoiceAttr::default()
        })
        .unwrap();
    }
    let mut output = [[0; 2]; 1024];
    tracked(|| {
        for _ in 0..10 {
            s.render(&mut output);
        }
    });
    assert_eq!(s.decode_errors(), 0);
}
#[cfg(feature = "output")]
#[test]
fn device_callback_allocation_free_resampling_and_underrun() {
    use psxspu::output::{Callback, Statistics};
    use std::sync::{Arc, atomic::Ordering};
    let (mut producer, consumer) = rtrb::RingBuffer::new(1024);
    for _ in 0..1024 {
        producer.push([1000, -1000]).unwrap();
    }
    let statistics = Arc::new(Statistics::default());
    let mut c = Callback::new(consumer, Arc::clone(&statistics), 48000).unwrap();
    let mut output = [0f32; 1024];
    tracked(|| {
        for _ in 0..8 {
            c.fill(&mut output, 2);
        }
    });
    assert_eq!(statistics.callbacks.load(Ordering::Relaxed), 8);
    assert!(statistics.underrun_frames.load(Ordering::Relaxed) > 0);
    assert!(output.iter().all(|x| *x == 0.0));
}
#[cfg(feature = "output")]
#[test]
fn device_native_rate_preserves_pcm_and_unsigned_silence() {
    use psxspu::output::{Callback, Statistics};
    use std::sync::Arc;
    let (mut p, c) = rtrb::RingBuffer::new(4);
    p.push([-32768, 32767]).unwrap();
    let mut c = Callback::new(c, Arc::new(Statistics::default()), 44100).unwrap();
    let mut out = [0i16; 4];
    c.fill(&mut out, 2);
    assert_eq!(out, [-32768, 32767, 0, 0]);
    let mut out = [0u16; 4];
    tracked(|| c.fill(&mut out, 2));
    assert_eq!(out, [32768; 4]);
}
