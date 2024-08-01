use std::simd::Simd;
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::Duration;

use crossbeam::atomic::AtomicCell;
use kiss3d::camera::{Camera, FirstPerson};
use kiss3d::event::{Action, Key};
use kiss3d::nalgebra::{Point2, Point3, Translation3};
use kiss3d::ncollide3d::math::Translation;
use kiss3d::text::Font;
use kiss3d::window::Window;
use rayon::prelude::*;

use crate::{get_node_tree, get_nodes, get_solver, get_suburbs, get_traffic_lights};
use crate::objects::boundary::Boundary;
use crate::objects::solver::node::Node;
use crate::objects::solver::node_type::NodeType;
use crate::objects::suburb::Suburb;
use crate::objects::traffic_light::TrafficLight;
use crate::objects::util::quad_tree::QuadTree;
use crate::objects::util::stop_watch::StopWatch;
use crate::traits::Positional;
use crate::types::Pos;

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
fn add_point_to_scene(window: &mut Window, position: &Simd<Pos, 2>) {
    window.draw_point(
        &point(position[0], position[1]),
        &Point3::new(1f32, 1f32, 1f32),
    );
}

#[inline]
fn point(x: Pos, y: Pos) -> Point3<f32> {
    Point3::new(0f32, y as f32 + Y_OFFSET, x as f32 + X_OFFSET)
}

#[inline]
fn add_suburb_to_scene(window: &mut Window, geometry: &Suburb) {
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

fn handle_key_pressed(window: &Window) -> bool {
    input(window, Key::Key1, true,
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
    )
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
        *key_pressed = handle_key_pressed(window);
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

        *key_pressed = handle_key_pressed(window);
    }
}

#[inline]
fn draw_boundary(window: &mut Window, boundary: &Boundary) {
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

fn draw_tree<T : Positional>(window: &mut Window, tree: &QuadTree<T>) {
    if tree.has_children {
        draw_tree(window, tree.top_left.as_ref().as_ref().unwrap());
        draw_tree(window, tree.top_right.as_ref().as_ref().unwrap());
        draw_tree(window, tree.bottom_left.as_ref().as_ref().unwrap());
        draw_tree(window, tree.bottom_right.as_ref().as_ref().unwrap());
    } else {
        draw_boundary(window, &tree.boundary)
    }
}

pub fn start_window() {
    let mut timer = StopWatch::start();
    let mut camera = FirstPerson::new_with_frustrum(70f32, 0.0001, 1000f32, Point3::new(0f32, 0f32, 0f32), Point3::new(1f32, 0f32, 0f32));
    camera.translate_mut(&Translation3::new(-15f32, 0f32, 0f32));
    camera.rebind_rotate_button(None);
    let mut window = Window::new("Rust Debugging Viewer");
    let temp_traffic_lights = get_traffic_lights();
    let traffic_lights = temp_traffic_lights.as_slice();


    let temp_geo = get_suburbs();
    let geometries = temp_geo.as_slice();
    timer.elapsed_store("Initial Setup");

    let mut search_speed: u32 = 100_000;
    let mut display_traffic_lights = true;
    let mut display_suburbs = false;
    let mut display_nodes = false;
    let mut display_path = true;
    let mut display_tree = false;
    let mut key_pressed = false;
    
    let found = Arc::new(AtomicCell::new(false));
    let threaded_found = found.clone();
    let closed = Arc::new(AtomicCell::new(false));
    let threaded_closed = closed.clone();
    timer.disable();
    let thread = spawn(move || unsafe {
        let mut timer = StopWatch::start();
        loop {
            get_solver().compute_pre_find();
            sleep(Duration::from_millis(16));
            if *threaded_closed.as_ptr().as_mut().unwrap() {
                break;
            }
            if get_solver().fully_searched() {
                timer.print_prefixed("Thread");
                threaded_found.store(true);
                sleep(Duration::from_secs(10));
                get_solver().update_search(373729, 37887);
                timer.reset();
            }
        }
    });
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
            draw_tree(&mut window, get_node_tree());
            timer.elapsed_store("Tree Display");
        }

        timer.elapsed_store("Path Find");
        if display_nodes {
            get_nodes()
                .get_slice()
                .par_iter()
                .filter(|x2| x2.get_mut().has_visited())
                .collect::<Vec<_>>()
                .iter()
                .for_each(|x| {
                    add_graph_node_to_scene(&mut window, x.get(), &(match x.get_mut().node_type {
                        NodeType::Normal => Point3::new(0f32, 1f32, 0f32),
                        NodeType::NearTrafficLight => Point3::new(0.5f32, 0.5f32, 0f32),
                        NodeType::AtTrafficLight => Point3::new(1f32, 0f32, 0f32)
                    }))
                });
            timer.elapsed_store("Visited Node Display");
        }
        if let Some(c_path) = get_solver().get_path_as_positions() {
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
                window.draw_text(format!("Cost {cost}").as_str(), &Point2::new((window.width() as f32)/2f32, (window.height() as f32)/2f32), 90f32, &Font::default(), &Point3::new(1f32, 1f32, 1f32));
                timer.elapsed_store("Path Display");
            }
        }
    }
    unsafe {
        *closed.as_ptr().as_mut().unwrap() = true;
    }
    thread.join().expect("TODO: panic message");
}
