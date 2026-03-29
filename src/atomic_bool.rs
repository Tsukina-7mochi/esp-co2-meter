use core::cell::RefCell;
use critical_section::{CriticalSection, Mutex};

#[derive(Debug)]
pub struct MyAtomicBool {
    inner: Mutex<RefCell<bool>>,
}

impl MyAtomicBool {
    pub const fn new(value: bool) -> Self {
        Self {
            inner: Mutex::new(RefCell::new(value)),
        }
    }

    pub fn store(&self, value: bool, cs: CriticalSection) {
        *self.inner.borrow_ref_mut(cs) = value;
    }

    pub fn swap(&self, value: bool, cs: CriticalSection) -> bool {
        let mut flag = self.inner.borrow_ref_mut(cs);
        let val = *flag;
        *flag = value;
        val
    }

    pub fn swap_in_cs(&self, value: bool) -> bool {
        critical_section::with(|cs| self.swap(value, cs))
    }
}
