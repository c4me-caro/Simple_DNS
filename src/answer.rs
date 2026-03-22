fn map_type_class(word: &str) -> u16 {
  return match word {
    "A" => 1,
    "IN" => 1,
    "NS" => 2,
    "CH" => 3,
    "HS" => 4,
    "CNAME" => 5,
    "MX" => 15,
    "TXT" => 16,
    _ => 1, 
  };
}

fn encode_domain_name(domain: &str) -> Vec<u8> {
  let mut buffer = Vec::new();
  let parts: Vec<&str> = domain.split('.').collect();

  for part in parts {
    if part.len() == 0 {
      buffer.push(0 as u8);
      break;
    }

    buffer.push(part.len() as u8);
    buffer.extend_from_slice(part.as_bytes());
  }

  return buffer;
}

fn convert_to_vec(name: &str, ttl: u32, class: &str, typer: &str, data: &str, priority: u16) -> Vec<u8> {
  let mut buffer = Vec::new();
  
  buffer.extend_from_slice(&encode_domain_name(name));
  buffer.extend_from_slice(&map_type_class(typer).to_be_bytes());
  buffer.extend_from_slice(&map_type_class(class).to_be_bytes());
  buffer.extend_from_slice(&ttl.to_be_bytes());

  match typer {
    "A" => {
      buffer.extend_from_slice(&4u16.to_be_bytes());
      let ip_parts: Vec<&str> = data.split('.').collect();

      for octect in ip_parts {
        buffer.push(octect.parse::<u8>().unwrap_or(0));
      }
    },
    "CNAME" | "NS" => {
      let long = 1+data.len() as u16;
      buffer.extend_from_slice(&long.to_be_bytes());
      buffer.extend_from_slice(&encode_domain_name(&data));
    },
    "MX" => {
      let long = 3+data.len() as u16;
      buffer.extend_from_slice(&long.to_be_bytes());
      buffer.extend_from_slice(&priority.to_be_bytes());
      buffer.extend_from_slice(&encode_domain_name(&data));
    },
    "TXT" => {
      let long = 1+data.len() as u16;
      buffer.extend_from_slice(&long.to_be_bytes());
      buffer.push(data.len() as u8);
      buffer.extend_from_slice(data.as_bytes());
    },
    _ => {
      
    }
  }

  return buffer;
}

pub fn write_buffer(buffer: &mut [u8], offset: usize, name: &str, ttl: u32, class: &str, typer: &str, data: &str, priority: u16) -> u16 {
  let new_buffer = convert_to_vec(&name, ttl, &class, &typer, &data, priority);
  buffer[offset..offset + new_buffer.len()].copy_from_slice(&new_buffer);
  return new_buffer.len() as u16;
}