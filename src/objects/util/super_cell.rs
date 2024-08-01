use std::cell::UnsafeCell;
use std::simd::Simd;
use crate::traits::Positional;
use crate::types::Pos;
/// Modified version of `Cell`
#[repr(transparent)]
pub struct SuperCell<T : ?Sized> {
    value : UnsafeCell<T>
}

impl <T> SuperCell<T> {
    #[inline]
    pub const fn new(value : T) -> Self {
        Self {
            value : UnsafeCell::new(value)
        }
    }
    #[inline]
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut (*self.value.get()) }
    }
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &(*self.value.get()) }
    }
}

impl<T> SuperCell<[T]> {
    /// Returns a `&[SuperCell<T>]` from a `&SuperCell<[T]>`
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// let slice: &mut [i32] = &mut [1, 2, 3];
    /// let cell_slice: &SuperCell<[i32]> = SuperCell::from_mut(slice);
    /// let slice_cell: &[SuperCell<i32>] = cell_slice.as_slice_of_cells();
    ///
    /// assert_eq!(slice_cell.len(), 3);
    /// ```
    pub fn as_slice_of_cells(&self) -> &[SuperCell<T>] {
        // SAFETY: `SuperCell<T>` has the same memory layout as `T`.
        unsafe { &*(self as *const SuperCell<[T]> as *const [SuperCell<T>]) }
    }
}

impl<T, const N: usize> SuperCell<[T; N]> {
    /// Returns a `&[SuperCell<T>; N]` from a `&SuperCell<[T; N]>`
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// let mut array: [i32; 3] = [1, 2, 3];
    /// let cell_array: &SuperCell<[i32; 3]> = SuperCell::from_mut(&mut array);
    /// let array_cell: &[SuperCell<i32>; 3] = cell_array.as_array_of_cells();
    /// ```
    pub fn as_array_of_cells(&self) -> &[SuperCell<T>; N] {
        // SAFETY: `Cell<T>` has the same memory layout as `T`.
        unsafe { &*(self as *const SuperCell<[T; N]> as *const [SuperCell<T>; N]) }
    }
}

impl <T : Positional> Positional for SuperCell<T> {
    #[inline]
    fn position(&self) -> &Simd<Pos, 2> {
        self.get().position()
    }
}

unsafe impl <T> Sync for SuperCell<T> {}

unsafe impl <T> Send for SuperCell<T> {}