use std::fs;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;

const FOLDER: &str = "zones";

#[derive(Clone)]
struct Context {
    current_origin: String,
    current_domain: String,
    current_ip: String,
    current_ttl: u32,
    last_record: String,
    inside_parentheses: bool,
    mx_priority: u16,
    machine_mode: String,
}

#[derive(Clone)]
pub struct Register {
    pub question: String,
    pub register_type: String,
    pub mode: String,
    pub answer: String,
    pub order: u16,
    pub ttl: u32,
}

pub static REGISTERS: LazyLock<Mutex<HashMap<(String, String), Register>>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    let r1 = Register {
        question: String::from("localbase.localhost."),
        register_type: String::from("A"),
        mode: String::from("IN"),
        answer: String::from("0.0.0.0"),
        order: 0,
        ttl: 86400,
    };

    m.insert((r1.question.clone(), r1.mode.clone()), r1);
    Mutex::new(m)
});

pub static MX_REGISTERS: LazyLock<Mutex<HashMap<(String, u16), Register>>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    let r1 = Register {
        question: String::from("mail.localbase.localhost."),
        register_type: String::from("MX"),
        mode: String::from("IN"),
        answer: String::from("0.0.0.0"),
        order: 1,
        ttl: 86400,
    };

    m.insert((r1.question.clone(), r1.order), r1);
    Mutex::new(m)
});

fn read_zone_file(path: std::path::PathBuf) -> Vec<String> {
    let content = fs::read_to_string(path).expect("Failed to read zone file");
    let mut buffer: Vec<String> = Vec::new();

    for line in content.lines() {
        let cleared_line = line.split(";").next().unwrap_or("").trim();
        if cleared_line.is_empty() {
            continue;
        }

        if cleared_line.contains("$INCLUDE") {
            continue;
        }

        let tokens: Vec<String> = cleared_line.split_whitespace().map(|s| s.to_string()).collect();
        buffer.extend(tokens);
    }

    return buffer;
}

fn read_zones() -> Vec<String> {
    let mut buffer: Vec<String> = Vec::new();

    for file in fs::read_dir(FOLDER).expect("Failed to read zone directory") {
        let file = file.expect("Failed to read zone file");
        let path = file.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("zone") {
            buffer.extend(read_zone_file(path));
        }
    }
    return buffer;
}

fn validate_ip(ip: String) -> bool {
    let octets: Vec<&str> = ip.split('.').collect();

    if octets.len() != 4 {
        return false;
    }

    for octet in octets {
        match octet.parse::<u8>() {
            Ok(_) => continue,
            Err(_) => return false,
        }
    }

    return true;
}

fn validate_domain(domain: String) -> bool {
    if domain.len() > 63 {
        return false;
    }

    if !domain.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.') {
        return false;
    }

    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 3 {
        return false;
    }

    if parts[parts.len() - 2].ends_with('-') || parts[0].starts_with('-') {
        return false;
    }

    return true;
}

fn manage_register(token: String, context: &mut Context) -> bool {
    match context.last_record.as_str() {
        "$ORIGIN" => {
            context.last_record.clear();
            if validate_domain(token.clone()) {
                context.current_origin = token.clone();
            }
        },
        "$TTL" => {
            context.last_record.clear();
            match token.parse::<u32>() {
                Ok(_) => context.current_ttl = token.parse::<u32>().expect("Parsing TTL Error"),
                Err(_) => return false,
            }
        },
        "A" => {
            context.last_record.clear();
            if !context.current_origin.is_empty() && validate_ip(token.clone()) {
                context.current_ip = token.clone();
            }
        },
        "CNAME" | "NS" => {
            context.last_record.clear();
            if !context.current_origin.is_empty() && validate_domain(token.clone()) {
                context.current_domain = token.clone();
            }
        },
        "MX" => {
            if context.mx_priority > 0 {
                if validate_domain(token.clone()) {
                    context.current_domain = token.clone();
                }
                context.last_record.clear();
            } else {
                match token.parse::<u16>() {
                    Ok(_) => context.mx_priority = token.parse::<u16>().expect("Parsing mx_priority Error"),
                    Err(_) => return false,
                }
            }
        },
        "TXT" => {
            context.last_record.clear();
            if !context.current_origin.is_empty() {
                context.current_domain = token.clone();
            }
        }
        _ => {
            return false;
        }
    }

    return true;
}

fn manage_complete_register(record: String, context: Context) -> bool {
    let mode: String;
    if context.machine_mode.is_empty() {
        mode = "IN".to_string();
    } else {
        mode = context.machine_mode.clone();
    }

    let mut register_map = REGISTERS.lock().unwrap();
    let mut mx_register_map = MX_REGISTERS.lock().unwrap();

    match record.as_str() {
        "A" => {
            if !context.current_origin.is_empty() && !context.current_ip.is_empty() {
                let new_register = Register {
                    question: context.current_origin.clone(),
                    register_type: "A".to_string(),
                    mode: mode.clone(),
                    answer: context.current_ip.clone(),
                    order: 0,
                    ttl: context.current_ttl,
                };

                register_map.insert((new_register.question.clone(), mode.clone()), new_register);
                return true;
            }
        },
        "CNAME" | "NS" => {
            if !context.current_domain.is_empty() && !context.current_origin.is_empty() {
                let new_register =  Register {
                    question: context.current_origin.clone(),
                    register_type: record,
                    mode: mode.clone(),
                    answer: context.current_domain.clone(),
                    order: 0,
                    ttl: context.current_ttl,
                };

                register_map.insert((new_register.question.clone(), mode.clone()), new_register);
                return true;
            }
        },
        "MX" => {
            if !context.current_origin.is_empty() && context.mx_priority > 0 && !context.current_domain.is_empty() {
                let new_register = Register {
                    question: context.current_origin.clone(),
                    register_type: "MX".to_string(),
                    mode: "IN".to_string(),
                    answer: context.current_domain.clone(),
                    order: context.mx_priority,
                    ttl: context.current_ttl,
                };

                mx_register_map.insert((new_register.question.clone(), new_register.order), new_register);
                return true;
            }
        },
        "TXT" => {
            if !context.current_domain.is_empty() {
                let new_register = Register {
                    question: context.current_origin.clone(),
                    register_type: "TXT".to_string(),
                    mode: mode.clone(),
                    answer: context.current_domain.clone(),
                    order: 0,
                    ttl: context.current_ttl,
                };

                register_map.insert((new_register.question.clone(), mode.clone()), new_register);
                return true;
            }
        },
        _ => {
            return false;
        }
    }

    return false;
}

pub fn load_registers() {
    let buffer = read_zones();
    let mut context = Context {
        current_origin: String::new(),
        current_domain: String::new(),
        current_ip: String::new(),
        current_ttl: 0,
        last_record: String::new(),
        inside_parentheses: false,
        mx_priority: 0,
        machine_mode: String::new(),
    };

    for token in buffer.clone().iter() {
        match token.as_str() {
            "(" => {
                context.inside_parentheses = true;
                continue;
            },
            ")" => {
                context.inside_parentheses = false;
                continue;
            },
            "A" | "CNAME" | "MX" | "TXT" | "NS" => {
                context.last_record = token.clone();
                continue;
            },
            "SOA" | "AAAA" | "PTR" | "CAA" | "SRV" => {
                continue;
            },
            "IN" | "HS" | "CH" => {
                context.machine_mode = token.clone();
                continue;
            },
            _ => {
                if token.starts_with("$") || token == "@" {
                    context.last_record = token.clone();
                    continue;
                }

                if context.last_record.is_empty() && validate_domain(token.clone()) {
                    context.current_origin = token.clone();
                    continue;
                }
            }
        }

        if context.last_record == "@" {
            continue;
        }

        if !context.last_record.is_empty() {
            let last_record = context.last_record.clone();
            let final_status: bool = manage_register(token.clone(), &mut context);

            if !final_status {
                continue;
            }

            let complete: bool = manage_complete_register(last_record, context.clone());
            if complete {
                context.machine_mode.clear();
                context.current_domain.clear();
                context.current_ip.clear();
                context.mx_priority = 0;
            }
        }
    }

    if context.inside_parentheses {
        println!("Warning: Unmatched parentheses in zone files");
        return;
    }
}