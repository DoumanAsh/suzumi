#[doc(hidden)]
#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! unreach {
    () => ({
        unsafe {
            std::hint::unreachable_unchecked();
        }
    })
}

#[doc(hidden)]
#[macro_export]
#[cfg(debug_assertions)]
macro_rules! unreach {
    () => ({
        unreachable!()
    })
}

pub trait OptionExt<T> {
    fn unwrap_certain(self) -> T;
}

impl<T> OptionExt<T> for Option<T> {
    #[inline(always)]
    fn unwrap_certain(self) -> T {
        match self {
            Some(res) => res,
            None => unreach!(),
        }
    }
}

pub struct DropAsync;
pub struct DropSync;
pub trait DropRunner {
    #[inline(always)]
    fn run<F: FnOnce() + Send + 'static>(f: F) {
        f();
    }
}

impl DropRunner for DropSync {}
impl DropRunner for DropAsync {
    #[inline(always)]
    fn run<F: FnOnce() + Send + 'static>(f: F) {
        tokio::task::spawn_blocking(f);
    }
}

pub struct DropGuard<F: FnOnce() + Send + 'static, T: DropRunner> {
    inner: Option<F>,
    _runner: T,
}

impl<F: FnOnce() + Send + 'static, T: DropRunner> DropGuard<F, T> {
    #[inline(always)]
    pub fn new(inner: F, _runner: T) -> Self {
        Self {
            inner: Some(inner),
            _runner,
        }
    }

    #[inline(always)]
    pub fn forget(self) {
        core::mem::forget(self)
    }
}

impl<F: FnOnce() + Send + 'static, T: DropRunner> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        if let Some(f) = self.inner.take() {
            T::run(f)
        }
    }
}
