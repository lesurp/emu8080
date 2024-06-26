use std::rc::Rc;
use std::sync::atomic::AtomicU16;
use std::{cell::RefCell, sync::Mutex};
use wasm_bindgen::{prelude::*, Clamped};

use crate::{
    cpu_state::{Ram, System},
    in_out::InOut,
    op_code::{Instruction, Register, RegisterPair},
};

use web_sys::console::log_1;
use web_sys::{CanvasRenderingContext2d, ImageData};

#[derive(Eq, PartialEq)]
enum RunState {
    Running,
    Stopping,
}

static STOP_FLAG: Mutex<RunState> = Mutex::new(RunState::Stopping);

fn stop_previous_game() {
    let mut l = STOP_FLAG.lock().unwrap();
    *l = RunState::Stopping;
}

fn dump_state(system: &System) {
    log_1(
        &"Dumping CPU state during execution error."
            .to_string()
            .into(),
    );
    log_1(&"Registers:".to_string().into());
    log_1(&format!("\tA: {:#04x}", system.cpu().get(Register::A)).into());
    log_1(&format!("\tF: {:#04x}", system.cpu().flags()).into());
    log_1(&format!("\tB: {:#04x}", system.cpu().get(Register::B)).into());
    log_1(&format!("\tC: {:#04x}", system.cpu().get(Register::C)).into());
    log_1(&format!("\tD: {:#04x}", system.cpu().get(Register::D)).into());
    log_1(&format!("\tE: {:#04x}", system.cpu().get(Register::E)).into());
    log_1(&format!("\tH: {:#04x}", system.cpu().get(Register::H)).into());
    log_1(&format!("\tL: {:#04x}", system.cpu().get(Register::L)).into());
    log_1(&"Register pairs:".to_string().into());
    log_1(&format!("\tA: {:#06x}", system.cpu().psw()).into());
    log_1(&format!("\tB: {:#06x}", system.cpu().get_rp(RegisterPair::B)).into());
    log_1(&format!("\tD: {:#06x}", system.cpu().get_rp(RegisterPair::D)).into());
    log_1(&format!("\tH: {:#06x}", system.cpu().get_rp(RegisterPair::H)).into());
    log_1(&format!("SP: {:#06x}", system.cpu().sp()).into());
    log_1(&format!("Inte: {}", system.cpu().inte()).into());
}

#[derive(Default)]
struct SpaceInvadersPorts {
    in_ports: Mutex<[u8; 8]>,
    out_ports: Mutex<[u8; 8]>,
    shift_port: AtomicU16,
}

impl SpaceInvadersPorts {
    fn set_input_bit(&self, port: usize, bit: u8) {
        self.in_ports.lock().unwrap()[port] |= 1 << bit;
    }

    #[allow(dead_code)]
    fn clear_input_bit(&self, port: usize, bit: u8) {
        self.in_ports.lock().unwrap()[port] &= !(1 << bit);
    }
}

impl InOut for SpaceInvadersPorts {
    fn write(&self, port: u8, value: u8) {
        if port == 4 {
            let prev = self.shift_port.load(std::sync::atomic::Ordering::Relaxed);
            let new = ((value as u16) << 8) + (prev >> 8);
            self.shift_port
                .store(new, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.out_ports.lock().unwrap()[port as usize] = value;
        }
    }

    fn read(&self, port: u8) -> u8 {
        if port == 3 {
            let val = self.shift_port.load(std::sync::atomic::Ordering::Relaxed);
            let offset = self.out_ports.lock().unwrap()[2] & 0x07;
            (val >> offset) as u8
        } else {
            self.in_ports.lock().unwrap()[port as usize]
        }
    }
}

#[derive(Default)]
struct CpuTestPorts {
    ports: [u8; 8],
}

impl InOut for CpuTestPorts {
    fn write(&self, port: u8, value: u8) {
        if port == 0 {
            let div = document().get_element_by_id("console").unwrap();
            let new_text = div.inner_html() + &format!("{}\n", value as char);
            div.set_inner_html(&new_text);
        }
    }

    fn read(&self, port: u8) -> u8 {
        self.ports[port as usize]
    }
}

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

#[wasm_bindgen]
pub fn cpu_test() -> Result<(), JsValue> {
    stop_previous_game();
    *STOP_FLAG.lock().unwrap() = RunState::Running;

    let div = document().get_element_by_id("console").unwrap();
    div.set_inner_html("");

    let mut ram = Ram::new(0x8000, true);
    let rom = include_bytes!("../roms/cputest");
    ram.register_rom(rom, 0x100).unwrap();
    // Shamelessly from: https://github.com/gergoerdi/clash-intel8080/blob/f2b09c5970efc0515f111b11d90c3ce648b648b6/test/Hardware/Intel8080/TestBench.hs#L20
    let rom = [
        0x3e, 0x0a, // 0x0000: exit:    MVI A, 0x0a
        0xd3, 0x00, // 0x0002:          OUT 0
        0x76, // 0x0004:          HLT
        0x3e, 0x02, // 0x0005: message: MVI A, 0x02
        0xb9, // 0x0007:          CMP C
        0xc2, 0x0f, 0x00, // 0x0008:          JNZ 0x000f
        0x7b, // 0x000B: putChr:  MOV A, E
        0xd3, 0x00, // 0x000C:          OUT 0
        0xc9, // 0x000E:          RET
        0x0e, 0x24, // 0x000F: putStr:  MVI C, '$'
        0x1a, // 0x0011: loop:    LDAX DE
        0xb9, // 0x0012:          CMP C
        0xc2, 0x17, 0x00, // 0x0013:          JNZ next
        0xc9, // 0x0016:          RET
        0xd3, 0x00, // 0x0017: next:    OUT 0
        0x13, // 0x0019:          INX DE
        0xc3, 0x11, 0x00, // 0x001a:          JMP loop
    ];
    ram.register_rom(&rom, 0x0).unwrap();
    let port_handler = Rc::new(CpuTestPorts::default());
    let pc = 0x100;

    let context = canvas()
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let mut emulator = EmulatorClosureState::new(System::new(ram, pc), port_handler, context);

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move |current_time: f64| {
        let l = STOP_FLAG.lock().unwrap();
        if *l == RunState::Stopping {
            return;
        }
        emulator.game_js_loop(current_time);
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

struct EmulatorClosureState {
    time: Option<f64>,
    system: System,
    port_handler: Rc<dyn InOut>,
    context: CanvasRenderingContext2d,
}

impl EmulatorClosureState {
    fn new(system: System, port_handler: Rc<dyn InOut>, context: CanvasRenderingContext2d) -> Self {
        EmulatorClosureState {
            time: None,
            system,
            port_handler,
            context,
        }
    }

    fn game_js_loop(&mut self, current_time: f64) {
        let system_frequency = 2_000_000;
        let system_frequency_for_ms = system_frequency / 1000;
        let video_buffer_offset = 0x2400;
        let display_width = 224;
        let memory_width = 32;
        let memory_height = 224;
        let mut next_refresh_irq = 1;
        let mut cycle_count = 0;
        let refresh_rate = 60;
        // we divide by two because there are two triggers per frame, not one!
        let refresh_rate_irq_threshold = (system_frequency / refresh_rate) / 2;

        if self.time.is_none() {
            self.time = Some(current_time);
        }
        let dt = current_time - self.time.unwrap();
        self.time = Some(current_time);
        let cycles_to_do = (dt as u64) * system_frequency_for_ms;

        let mut cycles_done = 0;
        while cycles_done < cycles_to_do {
            let instruction = self.system.next_instruction().unwrap();
            let instruction_cycles =
                match self.system.execute(instruction, self.port_handler.as_ref()) {
                    Ok(None) => return,
                    Ok(Some(i)) => i as u64,
                    Err(e) => {
                        dump_state(&self.system);
                        panic!("{}", e);
                    }
                };
            cycles_done += instruction_cycles;
            cycle_count += instruction_cycles;
            if cycle_count >= refresh_rate_irq_threshold {
                let irq_instruction = Instruction::Rst(next_refresh_irq);
                let incr = self
                    .system
                    .process(irq_instruction, self.port_handler.as_ref())
                    .unwrap()
                    .unwrap() as u64;
                next_refresh_irq = if next_refresh_irq == 2 { 1 } else { 2 };
                cycles_done += incr;
                cycle_count += incr;
                cycle_count -= refresh_rate_irq_threshold;
            }
        }

        let raw_video_buffer = self.system.get_slice(video_buffer_offset).unwrap();
        let rgba_buffer = bitmap_to_rgba(raw_video_buffer, memory_width, memory_height);

        self.context
            .put_image_data(
                &ImageData::new_with_u8_clamped_array(Clamped(&rgba_buffer), display_width)
                    .unwrap(),
                0.0,
                0.0,
            )
            .unwrap();
    }
}

// This function is automatically invoked after the wasm module is instantiated.
#[wasm_bindgen(start)]
fn init() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}

#[wasm_bindgen]
pub fn stop() -> Result<(), JsValue> {
    stop_previous_game();
    Ok(())
}

#[wasm_bindgen]
pub fn space_invaders() -> Result<(), JsValue> {
    stop_previous_game();
    *STOP_FLAG.lock().unwrap() = RunState::Running;

    let port_handler = Rc::new(SpaceInvadersPorts::default());
    port_handler.set_input_bit(0, 1);
    port_handler.set_input_bit(0, 2);
    port_handler.set_input_bit(0, 3);
    port_handler.set_input_bit(1, 3);

    let canvas = canvas();
    let kb_clone = port_handler.clone();
    let kb_closure_down = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
        match event.key().as_str() {
            "q" => kb_clone.set_input_bit(1, 5),
            "e" => kb_clone.set_input_bit(1, 6),
            "w" => kb_clone.set_input_bit(1, 4),
            "a" => kb_clone.set_input_bit(1, 0),
            "s" => kb_clone.set_input_bit(1, 2),
            _ => {}
        }
    });
    canvas
        .add_event_listener_with_callback("keydown", kb_closure_down.as_ref().unchecked_ref())
        .unwrap();
    kb_closure_down.forget();

    let kb_clone = port_handler.clone();
    let kb_closure_up = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
        match event.key().as_str() {
            "q" => kb_clone.clear_input_bit(1, 5),
            "e" => kb_clone.clear_input_bit(1, 6),
            "w" => kb_clone.clear_input_bit(1, 4),
            "a" => kb_clone.clear_input_bit(1, 0),
            "s" => kb_clone.clear_input_bit(1, 2),
            _ => {}
        }
    });
    canvas
        .add_event_listener_with_callback("keyup", kb_closure_up.as_ref().unchecked_ref())
        .unwrap();
    kb_closure_up.forget();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let mut ram = Ram::new(0x4000, false);
    let rom = include_bytes!("../roms/invaders");
    ram.register_rom(rom, 0).unwrap();
    let mut emulator = EmulatorClosureState::new(System::new(ram, 0), port_handler, context);

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move |current_time: f64| {
        let l = STOP_FLAG.lock().unwrap();
        if *l == RunState::Stopping {
            return;
        }
        emulator.game_js_loop(current_time);
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

fn bitmap_to_rgba(bitmap: &[u8], w: usize, h: usize) -> Vec<u8> {
    let number_pixels = w * h * 8;
    let mut out = Vec::new();
    out.resize(number_pixels * 4, 255);

    let display_width = w * 8;

    for y in 0..h {
        for x in 0..w {
            let byte = bitmap[y * w + x];
            for bit in 0..8 {
                let is_set = 255 * ((byte & (1 << bit)) >> bit);
                let display_x = x * 8 + bit;
                out[(y + h * (display_width - 1 - display_x)) * 4] = is_set;
                out[(y + h * (display_width - 1 - display_x)) * 4 + 1] = is_set;
                out[(y + h * (display_width - 1 - display_x)) * 4 + 2] = is_set;
            }
        }
    }

    out
}
