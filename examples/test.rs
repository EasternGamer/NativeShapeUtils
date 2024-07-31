#![feature(portable_simd)]
#![feature(duration_millis_float)]
#![feature(slice_pattern)]
#![feature(new_uninit)]

extern crate core;
extern crate pheap;

use core::slice::SlicePattern;
use std::fs::File;
use std::io::Read;
use std::simd::Simd;
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use crossbeam::atomic::AtomicCell;
use jni::sys::{jdouble, jint};
use kiss3d::camera::{Camera, FirstPerson};
use kiss3d::event::{Action, Key};
use kiss3d::nalgebra::{Point2, Point3, Translation3};
use kiss3d::ncollide3d::math::Translation;
use kiss3d::text::Font;
use kiss3d::window::Window;
use rayon::prelude::*;

use crate::data::*;
use crate::helper::*;
use crate::lib::*;
use crate::node::*;
use crate::solver::*;
use crate::stop_watch::*;
use crate::struts::*;

#[path = "../src/lib.rs"]
mod lib;

#[path = "../src/data.rs"]
mod data;

#[path = "../src/node.rs"]
mod node;

#[path = "../src/struts.rs"]
mod struts;

#[path = "../src/stop_watch.rs"]
mod stop_watch;

#[path = "../src/solver.rs"]
mod solver;

#[path = "../src/helper.rs"]
mod helper;

#[path = "../src/parallel_list.rs"]
mod parallel_list;

pub fn computation() {
    println!("Computing...");
    let mut stop_watch = StopWatch::start();
    let mut f = File::open("cache\\traffic.dat").expect("No traffic file not found");
    let mut file_bytes = Vec::new();
    let mut index = 0;
    f.read_to_end(&mut file_bytes).expect("Really Bad");
    stop_watch.elapsed_store("Traffic File Read");
    
    let traffic_data_size = read_i32(&file_bytes, &mut index);
    println!("Reading traffic lights {traffic_data_size}");
    for _index in 0..traffic_data_size {
        let size = read_i32(&file_bytes, &mut index) as usize;
        let segement = &file_bytes[index..(index+size)];
        index = index + size;
        let light = TrafficLight::from_bytes(segement);
        add_traffic_light(light.id as jint, light.position[0] as jdouble, light.position[1] as jdouble);

        //X.as_mut().insert(light, light.id);
    }
    stop_watch.elapsed_store("Traffic Data to Memory");

    let f = &mut File::open("cache\\suburb.dat").expect("No suburb file not found");
    index = 0;
    file_bytes.clear();
    f.read_to_end(&mut file_bytes).expect("Really Bad");
    stop_watch.elapsed_store("Suburb File Read");
    
    let suburb_size = read_i32(&file_bytes, &mut index);
    println!("Reading geometries {suburb_size}");
    for _index in 0..suburb_size {
        skip_i32(&mut index);
        let id = read_i32(&file_bytes, &mut index);
        let name_length = read_i32(&file_bytes, &mut index) as usize;
        let coordinate_length = read_i32(&file_bytes, &mut index) as usize;
        let min_x = read_f64(&file_bytes, &mut index);
        let min_y = read_f64(&file_bytes, &mut index);
        let max_x = read_f64(&file_bytes, &mut index);
        let max_y = read_f64(&file_bytes, &mut index);
        skip_string(&mut index, name_length);
        let mut x_points = new_double_slice(coordinate_length);
        let mut y_points = new_double_slice(coordinate_length);
        for index_c in 0..coordinate_length {
            x_points[index_c] = read_f64(&file_bytes, &mut index);
            y_points[index_c] = read_f64(&file_bytes, &mut index);
        }
        add_geometry(id, max_x, min_x, max_y, min_y, x_points, y_points);
    }
    println!("Completed reading geometry");
    stop_watch.elapsed_store("Suburb Data to Memory");

    let f = &mut File::open("cache\\nodes.dat").expect("No nodes file not found");
    index = 0;
    file_bytes.clear();
    f.read_to_end(&mut file_bytes).expect("Really Bad");
    stop_watch.elapsed_store("Node File Read");
    let nodes_size = read_i32(&file_bytes, &mut index);
    println!("Reading nodes {nodes_size}");
    for _index in 0..nodes_size {
        let size = read_i32(&file_bytes, &mut index) as usize;
        let segment = &file_bytes[index..(index+size)];
        index += size;
        let node = Node::from_bytes(segment);
        add_node(node.index as jint, node.position[0], node.position[1], 0 as jdouble, 0, node.connections);
    }
    stop_watch.elapsed_store("Node Data To Memory");
    println!("Completed reading nodes");
    let mut x : Loader<Node> = Loader::new("cache\\nodes.dat");
    let result = x.load().unwrap();
    stop_watch.elapsed_store("Node Data P");
    let temp_geo = data::get_geometry();
    let temp_traffic = data::get_traffic_lights();
    let geometries = temp_geo.as_parallel_slice();
    let traffic_lights = temp_traffic.as_slice();
    stop_watch.elapsed_store("Memory Read Data");

    let start_time_map = Instant::now();
    let _results: Vec<(jint, jint)> = compute(geometries, traffic_lights);
    stop_watch.elapsed_store("Computation Total Time");
    let nanos = start_time_map.elapsed().as_nanos() as f64;
    let nano_seconds_per_op = nanos / ((geometries.len() * traffic_lights.len()) as f64);
    let per_core_average = nano_seconds_per_op * 24f64;
    stop_watch.print_prefixed("Rust Binding");
    println!("Rust Binding - Check Average: {nano_seconds_per_op} ns/op");
    println!("Rust Binding - Per Core Average: {per_core_average} ns/op");
}

const X_OFFSET: f32 = -24.39524976974711;
const Y_OFFSET: f32 = 29.089456781809393;

#[inline]
fn add_traffic_light_to_scene(window: &mut Window, traffic_light: &TrafficLight) {
    add_point_to_scene(window, &traffic_light.position)
}


#[inline]
fn add_graph_node_to_scene(window: &mut Window, node: &Node, color: &Point3<f32>) {
    let position = node.position;
    window.draw_point(
        &point(position[0], position[1]),
        color,
    );
}

#[inline]
fn add_point_to_scene(window: &mut Window, position: &Simd<f64, 2>) {
    window.draw_point(
        &point(position[0], position[1]),
        &Point3::new(1f32, 1f32, 1f32),
    );
}

#[inline]
fn point(x: f64, y: f64) -> Point3<f32> {
    Point3::new(0f32, y as f32 + Y_OFFSET, x as f32 + X_OFFSET)
}

#[inline]
fn add_suburb_to_scene(window: &mut Window, geometry: &Geometry) {
    let x = &geometry.x_points;
    let y = &geometry.y_points;
    let size = x.len() - 1;
    let suburb_boundary_color = &Point3::new(200f32 / 255f32, 200f32 / 255f32, 200f32 / 255f32);
    for i in 0..size {
        window.draw_line(
            &point(x[i], y[i]),
            &point(x[i + 1], y[i + 1]),
            suburb_boundary_color,
        );
    }
    window.draw_line(
        &point(x[size], y[size]),
        &point(x[0], y[0]),
        suburb_boundary_color,
    );
}

#[inline]
fn input_of_key(window: &Window, input_key: Key) -> i32 {
    input(window, input_key, 1, 0)
}

#[inline]
fn input<T>(window: &Window, input_key: Key, pressed: T, unpressed: T) -> T {
    match window.get_key(input_key) {
        Action::Press => pressed,
        Action::Release => unpressed
    }
}

#[inline]
fn input_dynamic<T>(window: &Window, input_key: Key, pressed: fn() -> T, unpressed: fn() -> T) -> T {
    match window.get_key(input_key) {
        Action::Press => pressed(),
        Action::Release => unpressed()
    }
}

#[inline]
fn handle_input(window: &Window, camera: &mut FirstPerson,
                search_speed: &mut u32,
                display_traffic_lights: &mut bool,
                display_suburbs: &mut bool,
                display_nodes: &mut bool,
                display_path: &mut bool,
                display_tree: &mut bool,
                key_pressed: &mut bool) {
    let fwd = input_of_key(window, Key::S) - input_of_key(window, Key::W);
    let right = input_of_key(window, Key::D) - input_of_key(window, Key::A);
    let up = input_of_key(window, Key::Space) - input_of_key(window, Key::C);
    let speed = input(window, Key::LShift, 10f32, 1f32) * 0.01;
    let slow_down = input(window, Key::LControl, 0.01f32, 1f32);
    let move_vector = Point3::new(right as f32, up as f32, fwd as f32);
    let result = camera.view_transform().rotation.inverse_transform_point(&move_vector) * speed * slow_down;
    camera.translate_mut(&Translation::from(result));

    if *key_pressed {
        *key_pressed = input(window, Key::Key1, true,
                             input(window, Key::Key2, true,
                                   input(window, Key::Key3, true,
                                         input(window, Key::Key4, true,
                                               input(window, Key::Key5, true,
                                                     input(window, Key::Equals, true,
                                                           input(window, Key::Minus, true, false),
                                                     ),
                                               ),
                                         ),
                                   ),
                             ),
        );
    } else {
        let current = *display_traffic_lights;
        *display_traffic_lights = input(window, Key::Key1, !current, current);
        let current = *display_suburbs;
        *display_suburbs = input(window, Key::Key2, !current, current);
        let current = *display_nodes;
        *display_nodes = input(window, Key::Key3, !current, current);
        let current = *display_path;
        *display_path = input(window, Key::Key4, !current, current);
        let current = *display_tree;
        *display_tree = input(window, Key::Key5, !current, current);

        *search_speed = input(window, Key::Equals, *search_speed * 10, *search_speed);
        *search_speed = input(window, Key::Minus, u32::max(*search_speed / 10, 1), *search_speed);

        /*window.events().iter().filter(|event| {event.value.is_mouse_event()})
            .for_each(|event| {
                match event.value {
                    WindowEvent::MouseButton(Button1, Press, Modifiers::Alt) => {
                        match window.cursor_pos() {
                            Some(pos) => {
                                let size = window.size();
                                let result = camera.unproject(&Point2::new(pos.0 as f32, pos.1 as f32), &Vector2::new(size[0] as f32, size[1] as f32));

                            },
                            None => {}
                        }
                    }
                };
            });*/
        /*input_dynamic(window, Key::RAlt, || {
            window.events().iter().filter(|event| {event.value.is_mouse_event()})
                .for_each(|event| {
                    match event {
                        WindowEvent::MouseButton(Button1, Press, Modifiers::Alt) => 0
                    }
                });
            return 0;
        }, || {
            0
        });*/

        *key_pressed = input(window, Key::Key1, true,
                             input(window, Key::Key2, true,
                                   input(window, Key::Key3, true,
                                         input(window, Key::Key4, true,
                                               input(window, Key::Key5, true,
                                                    input(window, Key::Equals, true,
                                                            input(window, Key::Minus, true, false),
                                                    ),
                                               ),
                                         ),
                                   ),
                             ),
        );
    }
}

#[inline]
fn draw_boundary(window: &mut Window, boundary: &BoundarySIMD) {
    let blue = &Point3::new(204f32 / 255f32, 204f32 / 255f32, 1f32);
    window.draw_line(
        &point(boundary.corner_min[0], boundary.corner_min[1]),
        &point(boundary.corner_min[0], boundary.corner_max[1]),
        blue,
    );
    window.draw_line(
        &point(boundary.corner_min[0], boundary.corner_max[1]),
        &point(boundary.corner_max[0], boundary.corner_max[1]),
        blue,
    );
    window.draw_line(
        &point(boundary.corner_max[0], boundary.corner_max[1]),
        &point(boundary.corner_max[0], boundary.corner_min[1]),
        blue,
    );
    window.draw_line(
        &point(boundary.corner_max[0], boundary.corner_min[1]),
        &point(boundary.corner_min[0], boundary.corner_min[1]),
        blue,
    );
}

fn draw_tree<T : SimdPosition>(window: &mut Window, tree: &QuadTree<T>) {
    if tree.has_children {
        draw_tree(window, tree.top_left.as_ref().as_ref().unwrap());
        draw_tree(window, tree.top_right.as_ref().as_ref().unwrap());
        draw_tree(window, tree.bottom_left.as_ref().as_ref().unwrap());
        draw_tree(window, tree.bottom_right.as_ref().as_ref().unwrap());
    } else {
        draw_boundary(window, &tree.boundary)
    }
}

fn main() {
    computation();
    let size = size_of::<Node>();
    println!("Node size: {size}");
    let mut timer = StopWatch::start();
    let mut camera = FirstPerson::new_with_frustrum(70f32, 0.0001, 1000f32, Point3::new(0f32, 0f32, 0f32), Point3::new(1f32, 0f32, 0f32));
    camera.translate_mut(&Translation3::new(-15f32, 0f32, 0f32));
    let mut window = Window::new("Rust Debugging Viewer");
    let temp_traffic_lights = get_traffic_lights();
    let traffic_lights = temp_traffic_lights.as_slice();


    let temp_geo = get_geometry();
    let geometries = temp_geo.as_slice();

    let temp_nodes = get_nodes();
    let nodes = temp_nodes.get_slice();
    let mut counter = 0;
    let mut last_percentage = 10000;
    timer.elapsed_store("Initial Setup");

    let mut search_speed: u32 = 100_000;
    
    {
        let geo_size = geometries.len();
        let node_size = nodes.len();
        let traffic_size = traffic_lights.len();
        println!("Traffic Light Count: {traffic_size}");
        println!("Node Count: {node_size}");
        println!("Geo Size: {geo_size}");
    }
    
    timer.elapsed_store("Further initialization");
    unsafe {
        NODE_TREE = Some(create_tree(nodes));
        for traffic_light in traffic_lights {
            match get_node_tree().find_data(traffic_light.position()) {
                Some(data) => {
                    let d = data.as_slice();
                    NodeType::assign_types(traffic_light, d);
                }
                None => {}
            }
            
            counter += 1;
            let percentage = (counter*100)/traffic_lights.len();
            if last_percentage != percentage {
                println!("Done {percentage}%");
                last_percentage = percentage;
            }
        }
    }
    unsafe {
        SOLVER = Some(Solver::new(nodes, 373729, 37887, search_speed))
    }
    timer.elapsed_store("Construct Tree");
    let mut display_traffic_lights = false;
    let mut display_suburbs = false;
    let mut display_nodes = true;
    let mut display_path = true;
    let mut display_tree = false;
    let mut key_pressed = false;

    get_solver().start();
    let found = Arc::new(AtomicCell::new(false));
    
    let threaded_found = found.clone();
    let max_speed = 360f64;
    let min_speed = 0f64;
    let difference = max_speed - min_speed;
    let range = 255f64*3f64;
    let multiplier = range/difference;
    let closed = Arc::new(AtomicCell::new(false));
    let threaded_closed = closed.clone();
    let join = spawn(move || unsafe {
        let mut timer = StopWatch::start();
        loop {
            get_solver().compute_pre_find();
            if get_solver().fully_searched() || *threaded_closed.as_ptr().as_mut().unwrap() {
                threaded_found.store(true);
                break;
            }
            sleep(Duration::from_millis(16))
        }
        timer.print_prefixed("Thread");
    });
/*
    let mut timer = StopWatch::start();
    loop {
        get_solver().compute_pre_find();
        if get_solver().fully_searched() {
            threaded_found.store(true);
            break;
        }
    }
    timer.print_prefixed("Thread");*/
    let get_color = |speed: f64| -> Point3<f32> {
        let red = ((multiplier * (1f64 / 3f64) * (speed - min_speed)).clamp(0f64, 255f64) / 255f64) as f32;
        let green = ((multiplier * (1f64 / 3f64) * (max_speed - speed)).clamp(0f64, 255f64) / 255f64) as f32;
        Point3::new(red, green, 0f32)
    };
    while window.render_with_camera(&mut camera) {
        timer.elapsed_store("Render Time");
        timer.print_prefixed("Window");
        handle_input(&window, &mut camera, &mut search_speed, &mut display_traffic_lights, &mut display_suburbs, &mut display_nodes, &mut display_path, &mut display_tree, &mut key_pressed);
        get_solver().update_search_speed(search_speed);
        timer.elapsed_store("Handle Input");
        if display_traffic_lights {
            traffic_lights.iter().for_each(|x| {
                add_traffic_light_to_scene(&mut window, x);
            });
            timer.elapsed_store("Traffic Display");
        }

        if display_suburbs {
            geometries.iter().for_each(|x| {
                add_suburb_to_scene(&mut window, x);
            });
            timer.elapsed_store("Suburb Display");
        }

        if display_tree {
            draw_tree(&mut window, &get_node_tree());
            timer.elapsed_store("Tree Display");
        }

        //timer.elapsed_store("Reset Nodes");
        //let result = solver.compute();
        timer.elapsed_store("Path Find");
        if display_nodes {
            get_solver().get_nodes()
                .par_iter()
                .filter(|x2| x2.get_mut().has_visited())
                .collect::<Vec<_>>()
                .iter()
                .for_each(|x| {
                    add_graph_node_to_scene(&mut window, x.get(), &get_color(match x.get_mut().node_type { 
                        NodeType::Normal => 0f64,
                        NodeType::NearTrafficLight => 180f64,
                        NodeType::AtTrafficLight => 360f64
                    }))
                });
            timer.elapsed_store("Visited Node Display");
        }
        unsafe {
            if let Some(c_path) = &get_solver().path {
                if display_path {
                    let path = &c_path.0;
                    let destination_color = &Point3::new(1f32, 0f32, 0f32);
                    let start_color = &Point3::new(0f32, 0f32, 1f32);
                    for i in 1..path.len() {
                        let pos1 = &path[i - 1];
                        let pos2 = &path[i];
                        window.draw_line(
                            &point(pos1[0], pos1[1]),
                            &point(pos2[0], pos2[1]),
                            destination_color,
                        );
                    }
                    for item in path.iter() {
                        window.draw_point(
                            &point(item[0], item[1]),
                            start_color,
                        );
                    }
                    let cost = &c_path.1;
                    let length = path.len();
                    window.draw_text(format!("Cost {cost}").as_str(), &Point2::new((window.width() as f32)/2f32, (window.height() as f32)/2f32), 90f32, &Font::default(), &Point3::new(1f32, 1f32, 1f32));
                    println!("{length} of path");
                    timer.elapsed_store("Path Display");
                }
            }
        }
    }
    unsafe {
        *closed.as_ptr().as_mut().unwrap() = true;
    }
    join.join().expect("TODO: panic message");
}