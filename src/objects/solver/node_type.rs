use rayon::prelude::ParallelSlice;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use crate::distance;
use crate::objects::solver::node::Node;
use crate::objects::traffic_light::TrafficLight;
use crate::objects::util::super_cell::SuperCell;
use crate::traits::Positional;
use crate::types::Pos;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum NodeType {
    AtTrafficLight = 2,
    NearTrafficLight = 1,
    Normal = 0
}

const AT_TRAFFIC_LIGHT_THRESHOLD: Pos = 25f64 as Pos;
const NEAR_TRAFFIC_LIGHT_THRESHOLD: Pos = 100f64 as Pos;

impl NodeType {
    pub fn assign_types(traffic_light : &TrafficLight, nodes : &[&SuperCell<Node>]) {
        nodes.as_parallel_slice().into_par_iter().for_each(|node| {
            let mutable_node = node.get_mut();
            match mutable_node.node_type {
                NodeType::Normal => {
                    let distance = distance(&traffic_light.position, mutable_node.position());
                    if distance < AT_TRAFFIC_LIGHT_THRESHOLD {
                        mutable_node.node_type = NodeType::AtTrafficLight
                    } else if distance < NEAR_TRAFFIC_LIGHT_THRESHOLD {
                        mutable_node.node_type = NodeType::NearTrafficLight
                    }
                },
                NodeType::NearTrafficLight => {
                    let distance = distance(&traffic_light.position, mutable_node.position());
                    if distance < AT_TRAFFIC_LIGHT_THRESHOLD {
                        mutable_node.node_type = NodeType::AtTrafficLight
                    }
                },
                NodeType::AtTrafficLight => {}
            }
        });
    }
}