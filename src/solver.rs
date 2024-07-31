use std::simd::Simd;

use chrono::{Timelike, Utc};
use pheap::PairingHeap;
use radix_heap::RadixHeapMap;

use crate::data::distance;
use crate::node::*;
use crate::struts::{HasIndex, SimdPosition, SuperCell};
use crate::types::{Cost, Pos};

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
    pub start_node : &'solver SuperCell<Node>,
    pub end_node : &'solver SuperCell<Node>,
    pub avoid_traffic_lights : bool,
    pub total_iterations : u32,
    pub path : Option<(Box<[Simd<Pos, 2>]>, Cost)>,
    heap : RadixHeapMap<u32, &'solver SuperCell<Node>>,
    backup_heap : RadixHeapMap<u32, &'solver SuperCell<Node>>,
    direct_heap: PairingHeap<&'solver SuperCell<Node>, Cost>,
    current_iteration : u32,
    max_iterations : u32,
    nodes: &'solver [SuperCell<Node>]
}
const ASSUMED_SPEED : Pos = 120f64 as Pos;
#[inline]
fn calculate_weight_optimal(node_1 : &Node, node_2 : &Node) -> Pos {
    distance(node_1.position(), node_2.position())/ASSUMED_SPEED
}

impl <'solver> Solver<'solver>  {
    pub fn new(nodes : &'solver [SuperCell<Node>], start_node_index : usize, end_node_index : usize, max_iterations : u32) -> Self {
         Self {
             heap : RadixHeapMap::new(),
             backup_heap : RadixHeapMap::new(),
             direct_heap: PairingHeap::new(),
             path : None,
             avoid_traffic_lights : false,
             start_node : &nodes[start_node_index],
             end_node : &nodes[end_node_index],
             current_iteration : 0u32,
             total_iterations : 0u32,
             max_iterations,
             nodes
        }
    }

    pub fn start(&mut self) {
        self.heap.push(MAX_TIME, self.start_node);
        self.direct_heap.insert(self.start_node, calculate_weight_optimal(self.start_node.get(), self.end_node.get()) as Cost);
        self.start_node.get_mut().set_cost(0f64 as Cost);
    }

    #[inline(always)]
    pub fn get_nodes(&'solver self) -> &'solver [SuperCell<Node>] {
        self.nodes
    }

    #[inline(always)]
    pub fn update_search_speed(&mut self, new_speed : u32) {
        self.max_iterations = new_speed;
    }
    
    pub fn update_search(&mut self, start_node_index : usize, end_node_index : usize) {
        self.start_node = &self.nodes[start_node_index];
        self.end_node = &self.nodes[end_node_index];
        self.reset();
    }
    #[inline(always)]
    pub fn fully_searched(&self) -> bool {
        self.heap.is_empty()
    }

    fn merge(&mut self) {
        if !self.backup_heap.is_empty() {
            println!("Merging...");
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

    const fn is_load_shedding(flag : u32, current_cost_time : Cost) -> Cost {
        (flag << (31 - current_cost_time as u32) >> 31) as Cost
    }
    
    fn compute_pairing_direct(&mut self) {
        let end_node_index = self.end_node.get().index() as u32;
        let time_in_hour = (Utc::now().time().minute() as f64/60f64) as Cost;
        if !self.end_node.get_mut().has_visited() {
            println!("Computing Direct");
            let mut visited = Vec::new();
            let mut found = false;
            self.direct_heap.insert(self.start_node, calculate_weight_optimal(self.start_node.get(), self.end_node.get()) as Cost);
            while !self.direct_heap.is_empty() && !found {
                let current_node = self.direct_heap.delete_min().expect("Heap was not empty, but had nothing to pop.").0.get_mut();
                let local_cost = current_node.get_cost();
                let time_offset_cost = time_in_hour + local_cost;
                let new_node_length = current_node.get_connection_len() + 1;
                let previous_index = current_node.index();
                for connection in current_node.get_connections() {
                    let super_connection = &self.nodes[connection.index as usize];
                    let connected_node = super_connection.get_mut();
                    let connection_cost =
                        if self.avoid_traffic_lights {
                            match current_node.node_type {
                                NodeType::Normal => connection.cost,
                                NodeType::NearTrafficLight => connection.cost * (10f64 as Cost) * Self::is_load_shedding(current_node.flag, time_offset_cost),
                                NodeType::AtTrafficLight => connection.cost * (20f64 as Cost) * Self::is_load_shedding(current_node.flag, time_offset_cost)
                            }
                        } else {
                            connection.cost
                        };
                    let new_local_cost = local_cost + connection_cost;
                    found = connection.index == end_node_index;
                    if !found && !connected_node.has_visited() {
                        visited.push(super_connection);
                    }
                    if connected_node.check_updated_and_save(new_local_cost, previous_index as u32, new_node_length) && !found {
                        self.direct_heap.insert(super_connection, calculate_weight_optimal(connected_node, self.end_node.get()) as Cost);
                    }
                }
            }
            self.direct_heap = PairingHeap::new();
            visited.into_iter().for_each(|node| node.get_mut().reset());
            self.start_node.get_mut().reset();
            self.start_node.get_mut().set_cost(0f64 as Cost);
        }
    }
    
    fn compute_radix(&mut self) {
        let end_node_index = self.end_node.get().index() as u32;
        let time_in_hour = (Utc::now().time().minute() as f64/60f64) as Cost;
        while !self.heap.is_empty() && self.current_iteration < self.max_iterations {
            self.current_iteration += 1;
            self.total_iterations += 1;
            let pop = self.heap.pop().expect("Heap was not empty, but had nothing to pop.");
            let current_node = pop.1.get_mut();
            let local_cost = current_node.get_cost();
            if self.end_node.get_mut().is_lower_cost(local_cost) {
                continue;
            }
            let previous_index = current_node.index();
            let pop_cost = pop.0;
            let new_node_length = current_node.get_connection_len() + 1;
            let time_offset_cost = time_in_hour + local_cost;
            for connection in current_node.get_connections() {
                let connection_cost =
                    if self.avoid_traffic_lights {
                        match current_node.node_type {
                            NodeType::Normal => connection.cost,
                            NodeType::NearTrafficLight => connection.cost * (10f64 as Cost) * Self::is_load_shedding(current_node.flag, time_offset_cost),
                            NodeType::AtTrafficLight => connection.cost * (20f64 as Cost) * Self::is_load_shedding(current_node.flag, time_offset_cost)
                        }
                    } else {
                        connection.cost
                    };
                let new_local_cost = local_cost + connection_cost;
                let super_node = &self.nodes[connection.index as usize];
                let connected_node = super_node.get_mut();
                if connected_node.check_updated_and_save(new_local_cost, previous_index as u32, new_node_length) && connection.index != end_node_index {
                    let tmp = (new_local_cost*CONVERSION_FACTOR) as u32;
                    let push_cost = MAX_TIME - tmp;
                    if push_cost <= pop_cost {
                        self.heap.push(push_cost, super_node);
                    } else {
                        self.backup_heap.push(push_cost, super_node);
                    }
                }
            }
        }
        let iterations = self.total_iterations;
        println!("Computed Radix a total of {iterations}");
    }
    
    pub fn compute_pre_find(&mut self) {
        self.compute_pairing_direct();
        self.compute_radix();
        self.merge();
        self.current_iteration = 0;
        if self.end_node.get_mut().has_visited() {
            self.path = Some((self.backtrack(), self.end_node.get_mut().get_cost()))
        }
    }

    pub fn compute(&mut self) {
        self.compute_radix();
        self.merge();
        self.current_iteration = 0;
        if self.end_node.get_mut().has_visited() {
            self.path = Some((self.backtrack(), self.end_node.get_mut().get_cost()))
        }
    }
    
    pub fn backtrack(&self) -> Box<[Simd<Pos, 2>]> {
        let length = self.end_node.get_mut().get_connection_len() as usize;
        let mut path = Vec::with_capacity(length);
        let mut previous_node = self.end_node;
        for _ in 0..(length-1) {
            path.push(*previous_node.get().position());
            let previous_index = previous_node.get_mut().get_previous();
            if previous_index != u32::MAX {
                previous_node = &self.nodes[previous_index as usize];
            } else {
                break;
            }
        }
        //assert_eq!(previous_node.value.index(), self.start_node.value.index());
        path.push(*previous_node.get().position());
        path.into_boxed_slice()
    }

    pub fn reset(&mut self) {
        self.heap.clear();
        self.direct_heap = PairingHeap::new();
        self.path = None;
        self.nodes.iter().for_each(|x| {x.get_mut().reset()});
        self.current_iteration = 0;
        self.total_iterations = 0;
        self.heap.push(MAX_TIME, self.start_node);
        self.start_node.get_mut().set_cost(0f64 as Cost);
    }
}

unsafe impl <'solver> Sync for Solver<'solver>  {
}
unsafe impl <'solver> Send for Solver<'solver>{
}