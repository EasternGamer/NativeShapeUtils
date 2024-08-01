use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::Path;
use rayon::prelude::*;
use crate::traits::{ByteConvertable, Indexable};
use crate::objects::util::parallel_list::ParallelList;

#[inline]
pub fn read_f64(vector: &[u8], index: &mut usize) -> f64 {
    let current_index = *index;
    let new_size = current_index + 8;
    let bytes = &vector[current_index..new_size];
    *index = new_size;
    f64::from_be_bytes(bytes.try_into().expect(""))
}

#[inline]
pub fn read_i32(vector: &[u8], index: &mut usize) -> i32 {
    let current_index = *index;
    let new_size = current_index + 4;
    let bytes = &vector[current_index..new_size];
    *index = new_size;
    i32::from_be_bytes(bytes.try_into().expect(""))
}
#[inline]
pub fn read_string(vector: &[u8], index: &mut usize, string_len: usize) -> String {
    let current_index = *index;
    let new_size = current_index + string_len;
    let x = &vector[current_index..new_size];
    *index = new_size;
    String::from_utf8_lossy(x).to_string()
}

#[inline]
pub fn skip_f64(index: &mut usize) {
    *index += 8;
}

#[inline]
pub fn skip_i32(index: &mut usize) {
    *index += 4;
}

#[inline]
pub fn skip_string(index: &mut usize, string_len: usize) {
    *index += string_len;
}


pub fn load_from_bytes_parallel<T : Indexable + ByteConvertable>(bytes : &[u8]) -> ParallelList<T> {
    let mut index = 0;
    let size = read_i32(bytes, &mut index) as usize;
    let list = ParallelList::new(size);
    let mut byte_array = Vec::with_capacity(size);
    for _ in 0..size {
        let type_size = read_i32(bytes, &mut index) as usize;
        let type_bytes = &bytes[index..(index+type_size)];
        index += type_size;
        byte_array.push(type_bytes);
    }
    let block_size = size/12;
    byte_array.into_par_iter().by_uniform_blocks(block_size).for_each(|x| {
        let data = T::from_bytes(x);
        let index = data.index();
        list.insert(data, index);
    });
    list
}

pub fn load_from_bytes<T : Indexable + ByteConvertable>(bytes : &[u8]) -> ParallelList<T> {
    let mut index = 0;
    let size = read_i32(bytes, &mut index) as usize;
    let list = ParallelList::new(size);
    for _ in 0..size {
        let type_size = read_i32(bytes, &mut index) as usize;
        let type_bytes = &bytes[index..(index+type_size)];
        index += type_size;
        let data = T::from_bytes(type_bytes);
        let index = data.index();
        list.insert(data, index);
    }
    list
}

pub struct FileLoader<'loader, T : ByteConvertable + Indexable> {
    file_location : &'loader str,
    phantom_data: PhantomData<T>
}

impl <'loader, T : ByteConvertable + Indexable> FileLoader<'loader, T> {
    pub fn new(file_location : &'loader str) -> Self {
        Self {
            file_location,
            phantom_data : PhantomData
        }
    }

    pub fn load(&self) -> Result<ParallelList<T>, String> {
        let file_name = self.file_location;
        let path = Path::new(file_name);
        let possible_file = File::open(path);
        match possible_file {
            Ok(mut file) => {
                let mut data = Vec::new();
                file.read_to_end(&mut data).expect("");
                Ok(load_from_bytes(&data))
            },
            Err(e) => Err(e.to_string())
        }
    }

    pub fn load_parallel(&self) -> Result<ParallelList<T>, String> {
        let file_name = self.file_location;
        let path = Path::new(file_name);
        let possible_file = File::open(path);
        match possible_file {
            Ok(mut file) => {
                let mut data = Vec::new();
                file.read_to_end(&mut data).expect("");
                Ok(load_from_bytes_parallel(&data))
            },
            Err(e) => Err(e.to_string())
        }
    }
}