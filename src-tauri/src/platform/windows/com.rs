use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

struct Apartment {
    owned: bool,
}

impl Drop for Apartment {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: `CoUninitialize` must balance a `CoInitializeEx` that succeeded *on this
            // thread*. `owned` is set only by the initializer below, which runs in this thread's
            // own `thread_local!` slot, and `Apartment` is neither `Send` nor reachable from
            // another thread, so this drop runs on the same thread that initialized. The guard
            // makes the call at most once: the value is dropped once at thread exit.
            unsafe { CoUninitialize() };
        }
    }
}

thread_local! {
    static APARTMENT: Apartment = Apartment {
        // SAFETY: `CoInitializeEx` takes no pointer from us (`None` reserved argument) and is
        // callable on any thread that has not already joined an incompatible apartment. A
        // `thread_local!` initializer runs at most once per thread, so this cannot double-
        // initialize, and its failure is recorded rather than ignored so `Drop` only unwinds an
        // apartment this thread actually owns.
        owned: unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() },
    };
}

pub(crate) fn ensure_initialized() {
    APARTMENT.with(|_| ());
}
