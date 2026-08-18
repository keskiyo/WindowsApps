use std::ffi::c_void;
use windows::core::PWSTR;
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
};

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

pub(crate) struct CoTaskString(PWSTR);

impl CoTaskString {
    pub(crate) fn own(value: PWSTR) -> Self {
        Self(value)
    }

    pub(crate) fn to_trimmed(&self) -> Option<String> {
        if self.0.is_null() {
            return None;
        }
        // SAFETY: the pointer came from a shell API documented to return a null-terminated UTF-16
        // buffer, and `own` took it before anything else could free it, so the buffer is still
        // mapped for as long as this guard lives — which covers this call. The null check above
        // rules out the one value those APIs hand back without allocating, so the read below always
        // starts inside the allocation and stops at the terminator the API promises.
        let value = unsafe { self.0.to_string() }.ok()?;
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }
}

impl Drop for CoTaskString {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        // SAFETY: the pointer came from a shell API whose documented deallocator is `CoTaskMemFree`
        // and which transfers ownership to the caller. This type is the only owner — it is neither
        // `Clone` nor `Copy` and never hands the raw pointer out — so the buffer is freed exactly
        // once, here, and nothing can read it afterwards.
        unsafe { CoTaskMemFree(Some(self.0 .0 as *const c_void)) };
    }
}
