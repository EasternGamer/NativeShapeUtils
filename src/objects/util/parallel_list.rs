use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};
use core::slice::SlicePattern;
use rayon::prelude::ParallelSliceMut;
use crate::objects::util::super_cell::SuperCell;

pub struct ParallelList<T> {
    pub data : SuperCell<Box<[MaybeUninit<SuperCell<T>>]>>
}

impl <T> ParallelList<T> {
    pub fn new(size : usize) -> Self {
        Self {
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

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe {  &*(MaybeUninit::slice_assume_init_ref(self.data.get_mut().as_slice()) as *const [SuperCell<T>] as *const [T]) }
    }
}

impl <T : Sync + Send> ParallelList<T> {
    #[inline]
    pub fn get_slice_mut(&self) -> &mut [SuperCell<T>] {
        unsafe { MaybeUninit::slice_assume_init_mut(self.data.get_mut().as_parallel_slice_mut()) }
    }

    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { &mut *(MaybeUninit::slice_assume_init_mut(self.data.get_mut().as_parallel_slice_mut()) as *mut [SuperCell<T>] as *mut [T])}
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