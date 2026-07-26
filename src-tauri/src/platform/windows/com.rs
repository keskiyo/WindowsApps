//! Per-thread COM apartment.
//!
//! Shortcut resolution and icon extraction both need an apartment-threaded COM apartment, and
//! both used to bracket *every single call* with `CoInitializeEx` / `CoUninitialize`. Each time
//! the reference count reached zero the apartment was torn down and the in-process shell
//! servers unloaded, only to be reloaded microseconds later for the next shortcut — hundreds of
//! times per scan. `AGENTS_backend.md` §10 asks for initialization *per thread*, which is what
//! this provides: the apartment is created on first use and released when the worker thread
//! ends.

use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

struct Apartment {
    owned: bool,
}

impl Drop for Apartment {
    fn drop(&mut self) {
        if self.owned {
            unsafe { CoUninitialize() };
        }
    }
}

thread_local! {
    /// `owned` is false when the thread already had an incompatible apartment (`CoInitializeEx`
    /// answers `RPC_E_CHANGED_MODE`). Callers still proceed — exactly as before — but this
    /// thread must not call `CoUninitialize` for an apartment it did not create.
    static APARTMENT: Apartment = Apartment {
        owned: unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() },
    };
}

/// Make sure this thread has a COM apartment. Cheap after the first call.
pub(crate) fn ensure_initialized() {
    APARTMENT.with(|_| ());
}
