mod register;
mod answer;

use register::load_registers;
use register::Register;
use answer::write_buffer;

use crate::register::REGISTERS;
use crate::register::MX_REGISTERS;

use std::os::raw::{c_char, c_int};
use std::ffi::CStr;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[repr(C)]
struct resolve {
    domain_name: *const c_char,
    lenght: u16,
}

unsafe extern "C" {
    fn intialize() -> c_int;
    fn stop(sock: c_int) -> i32;
    fn get_port() -> i32;
    fn set_timeout(sock: c_int, time: c_int);
    fn receive(sock: c_int, buffer: *mut u8, size: c_int) -> resolve;
    fn respond(sock: c_int, buffer: *mut u8, qlen: i32, res: u16, err: u8) -> i32;
}

fn search_register(domain: &str, mode: u16) -> Option<Register> {
    let register_map = REGISTERS.lock().unwrap();
    let mode_string = match mode {
        1 => String::from("IN"),
        3 => String::from("CH"),
        4 => String::from("HS"),
        _ => String::from("IN"),
    };

    if mode > 0 {
        if let Some(register) = register_map.get(&(domain.to_string(),mode_string)) {
            return Some(register.clone());
        }
    }

    return register_map.values().find(|r| r.question == domain.to_string()).cloned();
}

fn main() {
    let run = Arc::new(AtomicBool::new(true));
    let r = run.clone();

    ctrlc::set_handler(move || {
        println!("\nManual interruption detected.");
        r.store(false, Ordering::Relaxed);
        
    }).expect("Error in Trap-C Signal configuration.");

    let socket = unsafe { intialize() };
    if socket < 0 {
        eprintln!("Failed to initialize the DNS server.");
        return;
    }

    unsafe { set_timeout(socket, 5) };
    load_registers();

    println!("DNS server initialized on port {}.", unsafe { get_port() });
    while run.load(Ordering::Relaxed) {
        let mut error: u8 = 0;
        let mut responses: u16 = 0;

        let mut buffer = [0u8; 512];
        let mut response: resolve = unsafe { receive(socket, buffer.as_mut_ptr(), buffer.len() as i32) };

        if response.lenght == 0xFFFF {
            continue;
        }

        if response.domain_name == std::ptr::null() || response.lenght == 0 {
            error = 1; // error de formato o pregunta malformada
            let _ = unsafe { respond(socket, buffer.as_mut_ptr(), (response.lenght-1) as i32, responses, error) };
            continue;
        }
        
        let domain_str = unsafe { 
            CStr::from_ptr(response.domain_name)
            .to_string_lossy()
        };

        let mx_register_map = MX_REGISTERS.lock().unwrap();
        let mx_registers: Vec<&Register> = mx_register_map
        .iter()
        .filter(|((question, _order), _register) | *question == domain_str.to_string())
        .map(|(_,register) | register)
        .collect();

        for reg in mx_registers {
            let offset = (11+response.lenght) as usize;
            let long = write_buffer(&mut buffer, offset, &reg.question, reg.ttl, &reg.mode, &reg.register_type, &reg.answer, reg.order);
            response.lenght += long;
            responses += 1;
        }

        if responses > 0 {
            let _ = unsafe { respond(socket, buffer.as_mut_ptr(), (response.lenght-1) as i32, responses, error) };
            continue;
        }

        let mut offset = (9 + response.lenght) as usize;
        let type_code = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);

        let some_reg = match search_register(&domain_str, type_code) {
            Some(reg) => reg,
            None => {
                error = 3; // dominio no existente
                let _ = unsafe { respond(socket, buffer.as_mut_ptr(), (response.lenght-1) as i32, responses, error) };
                continue;
            }
        };

        let mut long;
        offset = (11+response.lenght) as usize;
        long = write_buffer(&mut buffer, offset, &some_reg.question, some_reg.ttl, &some_reg.mode, &some_reg.register_type, &some_reg.answer, 0);
        response.lenght += long;
        responses += 1;

        if some_reg.register_type == "CNAME" || some_reg.register_type == "NS" {
            if let Some(other_reg) = search_register(&some_reg.answer, 1) {
                offset = (11+response.lenght) as usize;
                long = write_buffer(&mut buffer, offset, &other_reg.question, other_reg.ttl, &other_reg.mode, &other_reg.register_type, &other_reg.answer, 0);
                response.lenght += long;
                responses += 1;
            } else {
                error = 2; // servior con datos incompletos - error tecnico
            }
        }    

        let err = unsafe { respond(socket, buffer.as_mut_ptr(), (response.lenght-1) as i32, responses, error) };
        if err != 0 {
            eprintln!("Failed to respond to the DNS request.");
            continue;
        }
    }

    unsafe { stop(socket) };
}