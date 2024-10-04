use crate::objects::pathing::connection::Connection;
use crate::objects::pathing::node::Node;
use crate::objects::pathing::node_type::{NodeType, SearchMethod};
use crate::objects::util::parallel_list::ParallelList;
use crate::objects::util::super_cell::SuperCell;
use crate::types::{Cost, Flag, Index, Pos};
use chrono::{Timelike, Utc};
use radix_heap::RadixHeapMap;
use rayon::prelude::*;
use std::simd::Simd;

const HOUR_TO_MIN : f64 = 60f64;
const HOUR_TO_SEC : f64 = HOUR_TO_MIN*60f64;
const HOUR_TO_MS : f64 = HOUR_TO_SEC*1000f64;
const MAX_TIME_HOUR : u32 = 16;
const MAX_TIME_MIN : u32 = MAX_TIME_HOUR*60;
const MAX_TIME_S : u32 = MAX_TIME_MIN*60;
const MAX_TIME_MS : u32 = MAX_TIME_S*1000;
const MAX_TIME : u32 = MAX_TIME_S;
const CONVERSION_FACTOR : Cost = HOUR_TO_SEC as Cost;

pub struct Solver<'solver> {
    start_node : Index,
    end_node : Index,
    total_iterations : u32,
    costs : ParallelList<Cost>,
    previous_indices: ParallelList<Index>,
    previous_distances : ParallelList<Cost>,
    connection_lens : ParallelList<u16>,
    path : Option<(Box<[Index]>, Cost, Cost)>,
    heap : RadixHeapMap<u32, Index>,
    backup_heap : RadixHeapMap<u32, Index>,
    current_iteration : u32,
    max_iterations : u32,
    pub search_method : SearchMethod,
    nodes: &'solver [SuperCell<Node>]
}


impl <'solver> Solver<'solver>  {
    pub fn new(nodes : &'solver [SuperCell<Node>], start_node_index : usize, end_node_index : usize, max_iterations : u32, search_method: SearchMethod) -> Self {
         let mut new = Self {
             heap : RadixHeapMap::new(),
             backup_heap : RadixHeapMap::new(),
             path : None,
             start_node : start_node_index as Index,
             end_node : end_node_index as Index,
             current_iteration : 0u32,
             total_iterations : 0u32,
             costs: ParallelList::new(nodes.len()),
             previous_indices: ParallelList::new(nodes.len()),
             previous_distances: ParallelList::new(nodes.len()),
             connection_lens: ParallelList::new(nodes.len()),
             max_iterations,
             nodes,
             search_method
         };
        new.start();
        new
    }

    #[inline]
    fn calculate_weight(&self, connection : &Connection, node_type: NodeType, flag: Flag, time_offset : Cost) -> Cost {
        
        match self.search_method {
            SearchMethod::FASTEST => {
                let connection_cost = connection.cost / (connection.speed as Cost);
                match node_type {
                    NodeType::Normal => connection_cost,
                    NodeType::NearTrafficLight => connection_cost + connection_cost * (3 as Cost) * Self::is_load_shedding(flag, time_offset),
                    NodeType::AtTrafficLight => connection_cost + connection_cost * (5 as Cost) * Self::is_load_shedding(flag, time_offset)
                }
            },
            SearchMethod::SHORTEST => connection.cost / 60.0,
            SearchMethod::AVOID => {
                let connection_cost = connection.cost / (connection.speed as Cost);
                match node_type {
                    NodeType::Normal => connection_cost,
                    NodeType::NearTrafficLight => connection_cost + connection_cost * (100 as Cost) * Self::is_load_shedding(flag, time_offset),
                    NodeType::AtTrafficLight => connection_cost + connection_cost * (200 as Cost) * Self::is_load_shedding(flag, time_offset)
                }
            }
        }
    }
    
    #[inline(always)]
    pub fn get_connection_len(&self, index: Index) -> u16 {
        self.connection_lens[index as usize]
    }

    #[inline(always)]
    pub fn check_updated_and_save(&mut self, index_source : Index, new_cost : Cost, connection_cost: Cost, previous : usize, length : u16) -> bool {
        if self.costs[index_source as usize] > new_cost {
            self.costs[index_source as usize] = new_cost;
            self.previous_indices[index_source as usize] = previous as u32;
            self.connection_lens[index_source as usize] = length;
            self.previous_distances[index_source as usize] = connection_cost;
            return true;
        }
        false
    }
    #[inline(always)]
    pub fn get_cost(&self, index : Index) -> Cost {
        self.costs[index as usize]
    }
    #[inline(always)]
    pub fn is_lower_cost(&self, index: Index, new_cost : Cost) -> bool {
        self.get_cost(index) < new_cost
    }
    #[inline(always)]
    pub fn has_visited(&self, index: Index) -> bool {
        self.costs[index as usize] != Cost::MAX
    }
    #[inline(always)]
    pub fn get_previous(&self, index: Index) -> u32 {
        self.previous_indices[index as usize]
    }
    
    pub fn start(&mut self) {
        self.reset();
        self.heap.push(MAX_TIME, self.start_node);
        self.costs[self.start_node as usize] = 0f64 as Cost;
    }

    #[inline(always)]
    pub fn get_nodes(&'solver self) -> &'solver [SuperCell<Node>] {
        self.nodes
    }

    #[inline(always)]
    pub fn update_search_speed(&mut self, new_speed : u32) {
        self.max_iterations = new_speed;
    }

    pub fn update_search(&mut self, start_node_index : Index, end_node_index : Index) {
        self.start_node = start_node_index;
        self.end_node = end_node_index;
        self.reset();
        println!("Finding search between {start_node_index} to {end_node_index}");
    }
    #[inline(always)]
    pub fn fully_searched(&self) -> bool {
        self.heap.is_empty()
    }

    fn merge(&mut self) {
        if !self.backup_heap.is_empty() {
            let mut new_radix = RadixHeapMap::new();
            let left_over_values = self.heap.iter();
            let bad_values = self.backup_heap.iter();
            for (key, value) in left_over_values {
                new_radix.push(*key, *value);
            }
            for (key, value) in bad_values {
                new_radix.push(*key, *value);
            }
            self.heap = new_radix;
            self.backup_heap.clear();
        }
    }

    pub const fn is_load_shedding(flag : u32, current_cost_time : Cost) -> Cost {
        (flag << (31 - current_cost_time as u32) >> 31) as Cost
    }
    
    fn reset_index(&mut self, index: Index) {
        self.costs[index as usize] = Cost::MAX;
        self.previous_indices[index as usize] = u32::MAX;
        self.connection_lens[index as usize] = 0u16;
    }

    fn compute_radix(&mut self) {
        let end_node_index = self.end_node;
        let time_in_hour = (Utc::now().time().minute() as f64/60f64) as Cost;
        while !self.heap.is_empty() && self.current_iteration < self.max_iterations {
            self.current_iteration += 1;
            self.total_iterations += 1;
            let pop = self.heap.pop().expect("Heap was not empty, but had nothing to pop.");
            let current_node_index = pop.1;
            let local_cost = self.costs[current_node_index as usize];
            if self.is_lower_cost(self.end_node, local_cost) {
                continue;
            }
            let connected_node = self.nodes[current_node_index as usize].get();
            let node_type = connected_node.node_type;
            let flag = connected_node.flag;
            let pop_cost = pop.0;
            let new_node_length = self.get_connection_len(current_node_index) + 1;
            let time_offset_cost = time_in_hour + local_cost;
            for connection in connected_node.get_connections() {
                let connection_distance = connection.cost;
                let connection_cost = self.calculate_weight(connection, node_type, flag, time_offset_cost);
                let connection_index = connection.index;
                let new_local_cost = local_cost + connection_cost;
                if self.check_updated_and_save(connection_index, new_local_cost, connection_distance, current_node_index as usize, new_node_length) && connection_index != end_node_index {
                    let tmp = (new_local_cost*CONVERSION_FACTOR) as u32;
                    let push_cost = MAX_TIME - tmp;
                    if push_cost <= pop_cost {
                        self.heap.push(push_cost, connection.index);
                    } else {
                        self.backup_heap.push(push_cost, connection.index);
                    }
                }
            }
        }
    }
    
    pub fn get_start_node_index(&self) -> usize {
        self.start_node as usize
    }

    pub fn get_end_node_index(&self) -> usize {
        self.end_node as usize
    }
    
    pub fn get_path_as_indices(&self) -> &Option<(Box<[Index]>, Cost, Cost)> {
        &self.path
    }

    pub fn get_path_as_positions(&self) -> Option<(Box<[Simd<Pos, 2>]>, Cost, Cost)> {
        self.path.as_ref().map(|(indices, time, distance)| {
            let positions : Vec<Simd<Pos, 2>> = indices.iter().map(|index| {self.nodes[*index as usize].get().position}).collect();
            (positions.into_boxed_slice(), *time, *distance)
        })
    }

    pub fn compute(&mut self) {
        self.compute_radix();
        self.merge();
        self.current_iteration = 0;
        let end_index = self.end_node;
        if self.has_visited(end_index) {
            let (path, distance, time) = self.backtrack();
            let path_len = path.len();
            self.path = Some((path, time, distance));
            println!("Found path of length {path_len}");
        }
        println!("No Path Found");
    }

    fn get_distance(&self, index: Index) -> Cost {
        self.previous_distances[index as usize]
    }
    
    pub fn backtrack(&self) -> (Box<[Index]>, Cost, Cost) {
        let length = self.get_connection_len(self.end_node) as usize;
        let mut path = Vec::with_capacity(length);
        let mut previous_node = self.end_node;
        let mut distance = 0.0;
        let mut time = 0.0;
        for _ in 0..length {
            path.push(previous_node);
            let previous_index = self.get_previous(previous_node);
            let mut connection_cost = self.get_distance(previous_index);
            let time_offset = self.get_cost(previous_index);
            let node = self.nodes[previous_index as usize].get();
            distance += connection_cost;
            connection_cost /= (node.connections[0].speed as Cost);
            time += match node.node_type {
                NodeType::Normal => connection_cost,
                NodeType::NearTrafficLight => connection_cost + connection_cost * (3 as Cost) * Self::is_load_shedding(node.flag, time_offset),
                NodeType::AtTrafficLight => connection_cost + connection_cost * (5 as Cost) * Self::is_load_shedding(node.flag, time_offset)
            };
            if previous_index != u32::MAX {
                previous_node = previous_index as Index;
            } else {
                break;
            }
        }
        path.push(previous_node);
        (path.into_boxed_slice(), distance, time)
    }

    pub fn reset(&mut self) {
        self.heap.clear();
        self.backup_heap.clear();
        self.path = None;
        self.costs.as_slice_mut().par_iter_mut().for_each(|x| {*x = Cost::MAX});
        self.previous_indices.as_slice_mut().par_iter_mut().for_each(|x| {*x = u32::MAX});
        self.connection_lens.as_slice_mut().par_iter_mut().for_each(|x| {*x = 0u16});
        self.previous_distances.as_slice_mut().par_iter_mut().for_each(|x| {*x = 0.0});
        self.current_iteration = 0;
        self.total_iterations = 0;
        self.heap.push(MAX_TIME, self.start_node);
        self.costs[self.start_node as usize] = 0f64 as Cost;
    }
}

unsafe impl <'solver> Sync for Solver<'solver>  {
}

unsafe impl <'solver> Send for Solver<'solver>{
}