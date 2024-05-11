#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

use std::rc::Rc;
use std::{cell::RefCell, sync::Mutex};
use wasm_bindgen::{prelude::*, Clamped};

use cpu_state::{Ram, System,};
use op_code::{Register, RegisterPair};
use in_out::InOut;
use web_sys::console::log_1;
use web_sys::ImageData;

mod cpu_state;
mod in_out;
mod interrupts;
mod op_code;

    fn dump_state(system: &System) {
        log_1(&format!("Dumping CPU state during execution error.").into());
        log_1(&format!("Registers:").into());
        log_1(&format!("\tA: {:#04x}", system.cpu().get(Register::A)).into());
        log_1(&format!("\tF: {:#04x}", system.cpu().flags()).into());
        log_1(&format!("\tB: {:#04x}", system.cpu().get(Register::B)).into());
        log_1(&format!("\tC: {:#04x}", system.cpu().get(Register::C)).into());
        log_1(&format!("\tD: {:#04x}", system.cpu().get(Register::D)).into());
        log_1(&format!("\tE: {:#04x}", system.cpu().get(Register::E)).into());
        log_1(&format!("\tH: {:#04x}", system.cpu().get(Register::H)).into());
        log_1(&format!("\tL: {:#04x}", system.cpu().get(Register::L)).into());
        log_1(&format!("Register pairs:").into());
        log_1(&format!("\tA: {:#06x}", system.cpu().psw()).into());
        log_1(&format!("\tB: {:#06x}", system.cpu().get_rp(RegisterPair::B)).into());
        log_1(&format!("\tD: {:#06x}", system.cpu().get_rp(RegisterPair::D)).into());
        log_1(&format!("\tH: {:#06x}", system.cpu().get_rp(RegisterPair::H)).into());
        log_1(&format!("SP: {:#06x}", system.cpu().sp()).into());
        log_1(&format!("Inte: {}", system.cpu().inte()).into());
    }

#[derive(Default)]
struct SpaceInvadersPorts {
    //interrupt_tx: Sender<Interrupt>,
    ports: Mutex<[u8; 8]>,
}

impl SpaceInvadersPorts {
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

impl InOut for SpaceInvadersPorts {
    fn write(&self, port: u8, value: u8) {
        self.ports.lock().unwrap()[port as usize] = value;
    }

    fn read(&self, port: u8) -> u8 {
        self.ports.lock().unwrap()[port as usize]
    }
}

#[derive(Default)]
struct CpuTestPorts {
    ports: [u8; 8]
}

impl InOut for CpuTestPorts {
    fn write(&self, port: u8, value: u8) {
        if port == 0 {
            log_1(&format!("{}", value as char).into());
        }
    }

    fn read(&self, port: u8) -> u8 {
        self.ports[port as usize]
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

    let space_invaders = true;
    let (ram, port_handler, pc) : (_, Rc<dyn InOut>, _) = 
    if space_invaders {
        let mut ram = Ram::new(0x4000);
        let rom = include_bytes!("../roms/invaders");
        ram.register_rom(rom, 0).unwrap();
        let space_invaders_ports = Rc::new(SpaceInvadersPorts::default());
        let kb_clone = space_invaders_ports.clone();
        let kb_closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
            log_1(&format!("Pressed: {:?}", event.key()).into());
            match event.key().as_str() {
                "a" => kb_clone.set_input_bit(1, 0),
                _ => {}
            }
        });
    canvas
        .add_event_listener_with_callback("keydown", kb_closure.as_ref().unchecked_ref())
        .unwrap();
    kb_closure.forget();

    (ram, space_invaders_ports, 0)
    } else {
        let mut ram = Ram::new(0x8000);
        let rom = include_bytes!("../roms/cputest");
        ram.register_rom(rom, 0x100).unwrap();
        // Shamelessly from: https://github.com/gergoerdi/clash-intel8080/blob/f2b09c5970efc0515f111b11d90c3ce648b648b6/test/Hardware/Intel8080/TestBench.hs#L20
        let rom = 
        [  0x3e, 0x0a         // 0x0000: exit:    MVI A, 0x0a
        ,  0xd3, 0x00         // 0x0002:          OUT 0
        ,  0x76               // 0x0004:          HLT

        ,  0x3e, 0x02         // 0x0005: message: MVI A, 0x02
        ,  0xb9               // 0x0007:          CMP C
        ,  0xc2, 0x0f, 0x00   // 0x0008:          JNZ 0x000f
        ,  0x7b               // 0x000B: putChr:  MOV A, E
        ,  0xd3, 0x00         // 0x000C:          OUT 0
        ,  0xc9               // 0x000E:          RET

        ,  0x0e, 0x24         // 0x000F: putStr:  MVI C, '$'
        ,  0x1a               // 0x0011: loop:    LDAX DE
        ,  0xb9               // 0x0012:          CMP C
        ,  0xc2, 0x17, 0x00   // 0x0013:          JNZ next
        ,  0xc9               // 0x0016:          RET
        ,  0xd3, 0x00         // 0x0017: next:    OUT 0
        ,  0x13               // 0x0019:          INX DE
        ,  0xc3, 0x11, 0x00   // 0x001a:          JMP loop
        ];
        ram.register_rom(&rom, 0x0).unwrap();
        (ram, Rc::new(CpuTestPorts::default()), 0x100)
    };


    let mut system = System::new(ram, pc);

    let mut time = None;
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
            log_1(&format!("{:04x} {:?}", system.cpu().pc(), instruction).into());
            let instruction_cycles = match system.execute(instruction, port_handler.as_ref()) {
                Ok(i) => i as u64,
                Err(e) => {
                    dump_state(&system);
                    panic!("{}", e);
                }
            };
            cycles_done += instruction_cycles;
            cycle_count += instruction_cycles;
            if cycle_count >= refresh_rate_irq_threshold {
                let irq_instruction = op_code::Instruction::Rst(next_refresh_irq);
                if system.cpu().inte() {
                    log_1(&format!("{:04x} {:?}", system.cpu().pc(), instruction).into());
                }
                let incr = system
                    .process(irq_instruction, port_handler.as_ref())
                    .unwrap() as u64;
                next_refresh_irq = if next_refresh_irq == 2 { 1 } else { 2 };
                cycles_done += incr;
                cycle_count += incr;
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
