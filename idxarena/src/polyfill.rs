pub mod hint {
    #[rustversion::since(1.95)]
    #[cfg_attr(not(feature = "intern"), allow(unused))]
    pub use core::hint::cold_path;

    #[rustversion::before(1.95)]
    #[cold]
    #[inline(always)]
    #[cfg_attr(not(feature = "intern"), allow(unused))]
    pub fn cold_path() {}
}
