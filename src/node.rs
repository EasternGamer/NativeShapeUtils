use rayon::prelude::*;
use crate::data::distance;
use crate::struts::{Node, SimdPosition, TrafficLight};

#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NodeType {
    AtTrafficLight = 2,
    NearTrafficLight = 1,
    Normal = 0
}
impl NodeType {
    pub fn assign_types<T : SimdPosition>(traffic_light : &TrafficLight, nodes : &[&Node<T>]) {
        nodes.into_par_iter().for_each(|node| unsafe {
            match *node.node_type.get() {
                NodeType::Normal => {
                    let distance = distance(&traffic_light.position, node.position());
                    if distance < 10f64 {
                        *node.node_type.get().as_mut().expect("") = NodeType::AtTrafficLight
                    } else if distance < 20f64 {
                        *node.node_type.get().as_mut().expect("") = NodeType::NearTrafficLight
                    }
                },
                NodeType::NearTrafficLight => {
                    let distance = distance(&traffic_light.position, node.position());
                    if distance < 10f64 {
                        *node.node_type.get().as_mut().expect("") = NodeType::AtTrafficLight
                    }
                },
                NodeType::AtTrafficLight => {}
            }
        });
    }
}