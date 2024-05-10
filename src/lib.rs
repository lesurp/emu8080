#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

use std::rc::Rc;
use std::{cell::RefCell, sync::Mutex};
use wasm_bindgen::{prelude::*, Clamped};

use cpu_state::System;
use in_out::InOut;
use web_sys::console::log_1;
use web_sys::{EventListener, ImageData};

mod cpu_state;
mod in_out;
mod interrupts;
mod op_code;

const ROM: &[u8] = std::include_bytes!("../roms/invaders");

#[derive(Default)]
struct Gui {
    //interrupt_tx: Sender<Interrupt>,
    ports: Mutex<[u8; 8]>,
}

impl Gui {
    //pub fn new(interrupt_tx: Sender<Interrupt>) -> Self {
    //Gui { interrupt_tx }
    //}
    fn set_input_bit(&self, port: usize, bit: u8) {
        self.ports.lock().unwrap()[port as usize] |= 1 << bit;
    }

    fn clear_input_bit(&self, port: usize, bit: u8) {
        self.ports.lock().unwrap()[port as usize] &= !(1 << bit);
    }
}

impl InOut for Gui {
    fn write(&self, port: u8, value: u8) {
        self.ports.lock().unwrap()[port as usize] = value;
    }

    fn read(&self, port: u8) -> u8 {
        self.ports.lock().unwrap()[port as usize]
    }
}

/*
#[wasm_bindgen(start)]
fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    context.begin_path();

    // Draw the outer circle.
    context
        .arc(75.0, 75.0, 50.0, 0.0, f64::consts::PI * 2.0)
        .unwrap();

    // Draw the mouth.
    context.move_to(110.0, 75.0);
    context.arc(75.0, 75.0, 35.0, 0.0, f64::consts::PI).unwrap();

    // Draw the left eye.
    context.move_to(65.0, 65.0);
    context
        .arc(60.0, 65.0, 5.0, 0.0, f64::consts::PI * 2.0)
        .unwrap();

    // Draw the right eye.
    context.move_to(95.0, 65.0);
    context
        .arc(90.0, 65.0, 5.0, 0.0, f64::consts::PI * 2.0)
        .unwrap();

    context.stroke();
}

fn next_frame(timestamp:
*/

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

fn canvas() -> web_sys::HtmlCanvasElement {
    document()
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap()
}

// This function is automatically invoked after the wasm module is instantiated.
#[wasm_bindgen(start)]
fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    // Here we want to call `requestAnimationFrame` in a loop, but only a fixed
    // number of times. After it's done we want all our resources cleaned up. To
    // achieve this we're using an `Rc`. The `Rc` will eventually store the
    // closure we want to execute on each frame, but to start out it contains
    // `None`.
    //
    // After the `Rc` is made we'll actually create the closure, and the closure
    // will reference one of the `Rc` instances. The other `Rc` reference is
    // used to store the closure, request the first frame, and then is dropped
    // by this function.
    //
    // Inside the closure we've got a persistent `Rc` reference, which we use
    // for all future iterations of the loop
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let canvas = canvas();
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let gui = Rc::new(Gui::default());

    let gui2 = gui.clone();
    let kb_closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
        log_1(&format!("Pressed: {:?}", event.key()).into());
        match event.key().as_str() {
            "a" => gui2.set_input_bit(1, 0),
            _ => {}
        }
    });

    canvas
        .add_event_listener_with_callback("keydown", kb_closure.as_ref().unchecked_ref())
        .unwrap();
    kb_closure.forget();

    //let image_data = ImageData::new_with_u8_clamped_array(data,
    let mut time = None;
    let mut system = System::new(ROM, 0);
    let system_frequency = 2_000_000;
    let system_frequency_for_ms = system_frequency / 1000;
    let video_buffer_offset = 0x2400;
    //let display_height = 224;
    let display_width = 256;
    let memory_width = 32;
    let memory_height = 224;
    let mut next_refresh_irq = 1;
    let mut cycle_count = 0;
    let refresh_rate = 60;
    // we divide by two because there are two triggers per frame, not one!
    let refresh_rate_irq_threshold = (system_frequency / refresh_rate) / 2;
    *g.borrow_mut() = Some(Closure::new(move |current_time: f64| {
        if time.is_none() {
            time = Some(current_time);
        }
        //let current_time = std::time::Instant::now();
        let dt = current_time - time.unwrap();
        time = Some(current_time);
        let cycles_to_do = (dt as u64) * system_frequency_for_ms;

        let mut cycles_done = 0;
        while cycles_done < cycles_to_do {
            let instruction = system.next_instruction().unwrap();
            let instruction_cycles = system.execute(instruction, gui.as_ref()).unwrap() as u64;
            cycles_done += instruction_cycles;
            cycle_count += instruction_cycles;
            if cycle_count >= refresh_rate_irq_threshold {
                /*
                system
                    .process(op_code::Instruction::Rst(next_refresh_irq), gui.as_ref())
                    .unwrap();
                */
                next_refresh_irq = if next_refresh_irq == 2 { 1 } else { 2 };
                cycle_count -= refresh_rate_irq_threshold;
            }
        }

        let raw_video_buffer = system.get_slice(video_buffer_offset).unwrap();
        let rgba_buffer = bitmap_to_rgba(raw_video_buffer, memory_width, memory_height);

        context
            .put_image_data(
                &ImageData::new_with_u8_clamped_array(Clamped(&rgba_buffer), display_width)
                    .unwrap(),
                0.0,
                0.0,
            )
            .unwrap();

        /*
        while let Ok(interrupt) = rx.try_recv() {
            system.process(interrupt, &gui)?;
        }
        */
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

fn bitmap_to_rgba(bitmap: &[u8], w: usize, h: usize) -> Vec<u8> {
    let number_pixels = w * h * 8;
    let mut out = Vec::new();
    out.resize(number_pixels * 4, 255);

    for y in 0..h {
        for x in 0..w {
            let byte = bitmap[y * w + x];
            for bit in 0..8 {
                let is_set = 255 * ((byte & (1 << bit)) >> bit);
                out[((y * w + x) * 8 + bit) * 4 + 0] = is_set;
                out[((y * w + x) * 8 + bit) * 4 + 1] = is_set;
                out[((y * w + x) * 8 + bit) * 4 + 2] = is_set;
            }
        }
    }

    out
}
