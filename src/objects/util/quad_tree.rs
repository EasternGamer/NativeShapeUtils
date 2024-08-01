use std::simd::Simd;
use std::ops::{Div, Sub};
use crate::objects::boundary::Boundary;
use crate::traits::Positional;
use crate::types::Pos;

const MAX_CAPACITY : usize = 1024;
const MAX_DEPTH : i8 = 32;

pub struct QuadTree<'life, T : Positional> {
    pub top_left : Box<Option<QuadTree<'life, T>>>,
    pub top_right : Box<Option<QuadTree<'life, T>>>,
    pub bottom_left : Box<Option<QuadTree<'life, T>>>,
    pub bottom_right : Box<Option<QuadTree<'life, T>>>,
    pub depth : i8,
    pub has_children : bool,
    pub boundary : Boundary,
    pub data : Vec<&'life T>
}

impl <'life, T : Positional> QuadTree<'life, T> {
    #[inline]
    pub fn new(boundary_simd: Boundary, depth : i8) -> QuadTree<'life, T> {
        QuadTree {
            top_left : Box::new(None),
            top_right : Box::new(None),
            bottom_left : Box::new(None),
            bottom_right : Box::new(None),
            has_children : false,
            boundary : boundary_simd,
            depth,
            data : Vec::new()
        }
    }
    
    #[inline]
    pub fn contains(&self, point : &Simd<Pos, 2>) -> bool {
        self.boundary.contains(point)
    }
    
    
    fn find_data_top_left(&self, point : &Simd<Pos, 2>) -> Option<&Vec<&T>> {
        match self.top_left.as_ref() { 
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }
    fn find_data_top_right(&self, point : &Simd<Pos, 2>) -> Option<&Vec<&T>> {
        match self.top_right.as_ref() {
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }

    fn find_data_bottom_right(&self, point : &Simd<Pos, 2>) -> Option<&Vec<&T>> {
        match self.bottom_right.as_ref() {
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }
    fn find_data_bottom_left(&self, point : &Simd<Pos, 2>) -> Option<&Vec<&T>> {
        match self.bottom_left.as_ref() {
            Some(child_node) => {
                child_node.find_data(point)
            }
            None => None
        }
    }
    
    pub fn find_data(&self, point : &Simd<Pos, 2>) -> Option<&Vec<&T>> {
        if self.contains(point) {
            return if self.has_children {
                match self.find_data_top_left(point) {
                    Some(data) => Some(data),
                    None => match self.find_data_top_right(point) {
                        Some(data) => Some(data),
                        None => match self.find_data_bottom_left(point) {
                            Some(data) => Some(data),
                            None => match self.find_data_bottom_right(point) {
                                Some(data) => Some(data),
                                None => panic!("Point was inside quad tree, but not inside any child of the quad tree. This should *never* happen.")
                            }
                        }
                    }
                }
            } else {
                Some(&self.data)
            }
        }
        None
    }
    
    #[inline]
    fn get_top_left_node(&mut self) -> &mut QuadTree<'life, T> {
        self.top_left.as_mut().as_mut().expect("Had Children, but no top left tree")
    }
    #[inline]
    fn get_top_right_node(&mut self) -> &mut QuadTree<'life, T> {
        self.top_right.as_mut().as_mut().expect("Had Children, but no top right tree")
    }
    #[inline]
    fn get_bottom_left_node(&mut self) -> &mut QuadTree<'life, T> {
        self.bottom_left.as_mut().as_mut().expect("Had Children, but no bottom left tree")
    }
    #[inline]
    fn get_bottom_right_node(&mut self) -> &mut QuadTree<'life, T> {
        self.bottom_right.as_mut().as_mut().expect("Had Children, but no bottom right tree")
    }

    pub fn add_data(&mut self, data : &'life T) -> bool {
        let position = data.position();
        if self.contains(position) {
            if self.has_children {
                let top_left = self.get_top_left_node();
                if top_left.contains(position) {
                    return top_left.add_data(data);
                }
                let top_right = self.get_top_right_node();
                if top_right.contains(position) {
                    return top_right.add_data(data);
                }
                let bottom_left = self.get_bottom_left_node();
                if bottom_left.contains(position) {
                    return bottom_left.add_data(data);
                }
                let bottom_right = self.get_bottom_right_node();
                if bottom_right.contains(position) {
                    return bottom_right.add_data(data)
                }
                panic!("Trying to add a point that was inside a tree that had children, but it was not inside any children!");
            } else {
                return if self.data.len() < MAX_CAPACITY {
                    self.data.push(data);
                    true
                } else if self.depth < MAX_DEPTH {
                    self.sub_divide();
                    self.add_data(data)
                } else {
                    false
                };
            }
        }
        false
    }

    pub fn sub_divide(&mut self) {
        let corner_min_simd = &self.boundary.corner_min;
        let corner_max_simd = &self.boundary.corner_max;
        let center_simd = &corner_max_simd.sub((corner_max_simd - corner_min_simd).div(Simd::from_array([2f64 as Pos, 2f64 as Pos]))) ;
        let corner_min_array = corner_min_simd.as_array();
        let center_array = center_simd.as_array();
        let corner_max_array = corner_max_simd.as_array();

        let new_depth = self.depth + 1;
        let top_left_boundary = Boundary {
            corner_min: Simd::from_array([corner_min_array[0], center_array[1]]),
            corner_max: Simd::from_array([center_array[0], corner_max_array[1]])
        };
        let bottom_right_boundary = Boundary {
            corner_min: Simd::from_array([center_array[0], corner_min_array[1]]),
            corner_max: Simd::from_array([corner_max_array[0], center_array[1]])
        };
        let top_right_boundary = Boundary {
            corner_min: *center_simd,
            corner_max: *corner_max_simd
        };
        let bottom_left_boundary = Boundary {
            corner_min: *corner_min_simd,
            corner_max: *center_simd
        };
        let mut top_left = QuadTree::new(top_left_boundary, new_depth);
        let mut top_right = QuadTree::new(top_right_boundary, new_depth);
        let mut bottom_left = QuadTree::new(bottom_left_boundary, new_depth);
        let mut bottom_right = QuadTree::new(bottom_right_boundary, new_depth);
        
        
        for data_object in &self.data {
            if top_left.add_data(*data_object) {
                continue;
            }
            if top_right.add_data(*data_object) {
                continue;
            }
            if bottom_left.add_data(*data_object) {
                continue;
            }
            if !bottom_right.add_data(*data_object) {
                panic!("Failed to add data to children during sub division!");
            }
        }

        self.top_left = Box::new(Some(top_left));
        self.top_right = Box::new(Some(top_right));
        self.bottom_left = Box::new(Some(bottom_left));
        self.bottom_right = Box::new(Some(bottom_right));
        
        self.has_children = true;
    }
}