// SPDX-License-Identifier: GPL-3.0-only
// Appended to native.rs in OUT_DIR for Apple targets only. Keep the shared C
// callbacks, asset reads, movie decoder, raster and save payloads unchanged.
fn ios_clock() {
    let paused = crate::platform_ios::wait_for_active();
    // Called while port_present already holds the HOST RefCell borrow: use
    // independent clock storage, never re-enter host() from that callback.
    thread_local! { static NEXT: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) }; }
    NEXT.with(|cell| {
        let now = Instant::now();
        let next = if paused {
            now
        } else {
            cell.get().unwrap_or(now)
        };
        let next = next + Duration::from_secs_f64(1.0 / 60.0);
        if let Some(wait) = next.checked_duration_since(now) {
            std::thread::sleep(wait);
        }
        cell.set(Some(next));
    });
}

pub(crate) fn run_ios_worker(
    disc: GameDisc<DiscImage<File>>,
    saves: SaveStore,
    backends: Backends,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let now = Instant::now();
    HOST.with(|cell| {
        *cell.borrow_mut() = Some(Host {
            disc,
            gpu: backends.gpu,
            spu: backends.spu,
            pad: backends.pad,
            assets: AssetStore::default(),
            saves,
            proxy: None,
            frames: 0,
            limit: u64::MAX,
            screenshot: None,
            start: now,
            next: now,
            cancel,
            error: None,
            first_logo: None,
            last_frame: None,
            movie: None,
            state: 0,
            step: 0,
            movie_frames: 0,
            movie_skips: 0,
        });
    });
    // SAFETY: The single C worker is separate from UIKit. The existing C exit
    // jump crosses C frames only, after all Rust callbacks have returned.
    let code = unsafe { port_run_game() };
    let result = host(|h| {
        if let Some(movie) = h.movie.take() {
            movie.end(h.spu.as_mut(), false);
        }
        h.error.take().map_or_else(
            || {
                if code == 0 {
                    Ok(())
                } else {
                    Err(format!(
                        "Native game stopped (code {code}, tick {}).",
                        h.frames
                    ))
                }
            },
            Err,
        )
    });
    HOST.with(|cell| *cell.borrow_mut() = None);
    result
}
