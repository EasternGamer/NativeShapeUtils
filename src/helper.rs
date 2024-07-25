use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::Path;
use std::slice::SliceIndex;
use crate::struts::HasIndex;
use crate::parallel_list::ParallelList;

pub trait ByteConvertable {
    fn from_bytes(byte_array : &[u8]) -> Self;
}

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
    *index = *index + 8;
}

#[inline]
pub fn skip_i32(index: &mut usize) {
    *index = *index + 4;
}

#[inline]
pub fn skip_string(index: &mut usize, string_len: usize) {
    *index = *index + string_len;
}

pub struct Loader<'loader, T : ByteConvertable + HasIndex> {
    file_location : &'loader str,
    phantom_data: PhantomData<T>
}
impl <'loader, T : ByteConvertable + HasIndex> Loader<'loader, T> {
    pub fn new(file_location : &'loader str) -> Self {
        Self {
            file_location,
            phantom_data : PhantomData
        }
    }
    
    pub fn load_from_bytes(bytes : &[u8]) -> ParallelList<T> {
        let mut index = 0;
        let size = read_i32(bytes, &mut index) as usize;
        let mut list = ParallelList::new(size);
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
    pub fn load(&self) -> Result<ParallelList<T>, String> {
        let file_name = self.file_location;
        let path = Path::new(file_name);
        let possible_file = File::open(path);
        match possible_file { 
            Ok(mut file) => {
                let mut data = Vec::new();
                file.read_to_end(&mut data).expect("");
                Ok(Self::load_from_bytes(&data))
            }, 
            Err(e) => Err(e.to_string()) 
        }
    }
}