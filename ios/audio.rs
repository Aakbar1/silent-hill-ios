// SPDX-License-Identifier: GPL-3.0-only
//! Only this worker touches SpuCpal; UIKit publishes lifecycle requests.
use super::{STATE, is_running, wait_for_active};
use crate::{backend::SpuBackend, spu_cpal::SpuCpal};
use std::sync::{Arc, Mutex, atomic::Ordering};

pub(crate) struct IosSpu {
    inner: Arc<Mutex<SpuCpal>>,
}
pub(crate) fn open_audio() -> Result<IosSpu, String> {
    wait_for_active();
    let inner = Arc::new(Mutex::new(crate::spu_cpal::open_configured()?));
    *STATE
        .get()
        .expect("audio state")
        .audio
        .lock()
        .expect("audio handle") = Some(inner.clone());
    println!("IOS_AUDIO SpuCpal initialized with real device output");
    Ok(IosSpu { inner })
}
impl SpuBackend for IosSpu {
    fn reset(&mut self) {
        self.inner.lock().expect("SPU worker").reset();
    }
    fn advance_to(&mut self, sample: u64) -> Result<(), String> {
        wait_for_active();
        let state = STATE.get().expect("audio state");
        if let Some(error) = state.failure.lock().expect("audio failure").clone() {
            return Err(error);
        }
        let result = self.inner.lock().expect("SPU worker").advance_to(sample);
        // An interrupted AudioUnit can stop consuming while advance_to is in
        // flight. Recover only with a matching OS notification; ordinary device
        // errors remain failures. Partial mixing retains its absolute clock.
        if result.is_err() && (!is_running(state) || state.recover_audio.load(Ordering::Acquire)) {
            wait_for_active();
            if let Some(error) = state.failure.lock().expect("audio failure").clone() {
                return Err(error);
            }
            return self.inner.lock().expect("SPU worker").advance_to(sample);
        }
        result
    }
    fn finish(&mut self) -> Result<(), String> {
        self.inner.lock().expect("SPU worker").finish()
    }
    fn write_register(&mut self, o: u16, v: u16) -> Result<(), String> {
        self.inner.lock().expect("SPU worker").write_register(o, v)
    }
    fn read_register(&self, o: u16) -> Result<u16, String> {
        self.inner.lock().expect("SPU worker").read_register(o)
    }
    fn transfer_write(&mut self, a: u32, b: &[u8]) -> Result<(), String> {
        self.inner.lock().expect("SPU worker").transfer_write(a, b)
    }
    fn transfer_read(&self, a: u32, b: &mut [u8]) -> Result<(), String> {
        self.inner.lock().expect("SPU worker").transfer_read(a, b)
    }
    fn cd_input(&mut self, p: &[i16], r: u32, c: u8) -> Result<bool, String> {
        self.inner.lock().expect("SPU worker").cd_input(p, r, c)
    }
    fn cd_stop(&mut self) {
        self.inner.lock().expect("SPU worker").cd_stop();
    }
}
