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
