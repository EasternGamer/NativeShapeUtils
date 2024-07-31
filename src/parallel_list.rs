use core::slice::SlicePattern;
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};

use rayon::slice::ParallelSliceMut;

use crate::struts::SuperCell;

pub struct ParallelList<T> {
    pub len : usize,
    pub data : SuperCell<Box<[MaybeUninit<SuperCell<T>>]>>
}
pub struct ParallelConstantList<T, const N: usize> {
    pub len : usize,
    pub data : [MaybeUninit<T>; N]
}

impl <T> ParallelList<T> {
    pub fn new(size : usize) -> Self {
        Self {
            len : 0,
            data: SuperCell::new(Box::new_uninit_slice(size))
        }
    }

    #[inline]
    pub fn insert(&self, value : T, index : usize) {
        unsafe { *self.data.get_mut().get_unchecked_mut(index) = MaybeUninit::new(SuperCell::new(value)); }
    }
    #[inline]
    pub fn get(&self, index : usize) -> &T {
        unsafe { self.data.get_mut().get_unchecked(index).assume_init_ref().get()}
    }
    #[inline]
    pub fn get_mut(&self, index : usize) -> &mut T {
        unsafe { self.data.get_mut().get_unchecked_mut(index).assume_init_mut().get_mut()}
    }
    #[inline]
    pub fn get_slice(&self) -> &[SuperCell<T>] {
        unsafe { MaybeUninit::slice_assume_init_ref(self.data.get_mut().as_slice()) }
    }
}

impl <T : Sync + Send> ParallelList<T> {
    #[inline]
    pub fn get_slice_mut(&self) -> &mut [SuperCell<T>] {
        unsafe { MaybeUninit::slice_assume_init_mut(self.data.get_mut().as_parallel_slice_mut()) }
    }
}

impl <T> Index<usize> for ParallelList<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}
impl <T> IndexMut<usize> for ParallelList<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
    }
}

unsafe impl <T> Send for ParallelList<T> {}
unsafe impl <T> Sync for ParallelList<T> {}


impl <T, const N: usize> ParallelConstantList<T, N> {
    // Taken from heapless::Vec
    const ELEM: MaybeUninit<T> = MaybeUninit::uninit();
    const INIT: [MaybeUninit<T>; N] = [Self::ELEM; N]; // important for optimization of `new`
    pub const fn new() -> Self {
        Self {
            len : 0,
            data : Self::INIT
        }
    }

    #[inline]
    pub fn insert(&mut self, value : T, index : usize) {
        unsafe { *self.data.get_unchecked_mut(index) = MaybeUninit::new(value); }
        self.len += 1;
    }
    #[inline]
    pub fn get(&self, index : usize) -> &T {
        unsafe { self.data.get_unchecked(index).assume_init_ref()}
    }
    #[inline]
    pub fn get_mut(&mut self, index : usize) -> &mut T {
        unsafe { self.data.get_unchecked_mut(index).assume_init_mut()}
    }
}
impl <T, const N: usize> Index<usize> for ParallelConstantList<T, N> {
    type Output = T;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}
impl <T, const N: usize> IndexMut<usize> for ParallelConstantList<T, N> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
    }
}

unsafe impl <T, const N: usize> Send for ParallelConstantList<T, N> {}
unsafe impl <T, const N: usize> Sync for ParallelConstantList<T, N> {}