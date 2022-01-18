use log::{error, info, trace};
use nmea_parser::{gnss::RmcData, NmeaParser, ParsedMessage};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::net::UdpSocket;

fn recv_udp(buff: &mut [u8], socket: &UdpSocket) -> Option<String> {
    match socket.recv_from(buff) {
        Err(e) => {
            error!("Failed to recv_from: {:?}", e);
            None
        }
        Ok((amt, _src)) => match std::str::from_utf8(&buff[..amt]) {
            Ok(sentence) => Some(sentence.to_string()),
            Err(e) => {
                error!("Failed to parse from str: {:?}", e);
                None
            }
        },
    }
}

fn get_rmc(parser: &mut NmeaParser, sentence: &str) -> Option<RmcData> {
    match parser.parse_sentence(sentence) {
        Err(e) => {
            error!("Failed to parse sentence {:?} due to {:?}", sentence, e);
            None
        }
        Ok(parsed) => match parsed {
            ParsedMessage::Rmc(rmc) => Some(rmc),
            _ => None,
        },
    }
}

fn get_last_sentence(file: &mut File) -> Option<String> {
    let mut offset = -5;
    let mut last_byte = vec![0; 1];

    loop {
        offset -= 1;

        if let Err(e) = file.seek(std::io::SeekFrom::End(offset)) {
            error!("Failed to seek due to {:?}", e);
            return None;
        }

        if let Err(e) = file.read(&mut last_byte) {
            error!("Failed to read due to {:?}", e);
            return None;
        }

        if last_byte[0] == b'\n' {
            break;
        }
    }

    let mut str = String::new();
    match file.read_to_string(&mut str) {
        Ok(_) => Some(str),
        Err(e) => {
            error!("Failed to read_to_string due to {:?}", e);
            None
        }
    }
}

fn main() {
    env_logger::init();

    let socket = UdpSocket::bind("0.0.0.0:8888").expect("Couldn't build socket");
    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .append(true)
        .create(true)
        .open("nemas.txt")
        .expect("Failed to open nemas.txt");
    let mut parser = NmeaParser::new();
    let mut buff = [0; 100];
    let mut last_lat = None;
    let mut last_long = None;

    if let Some(last_sentence) = get_last_sentence(&mut file) {
        if let Some(rmc) = get_rmc(&mut parser, &last_sentence) {
            last_lat = rmc.latitude;
            last_long = rmc.longitude;
            info!("Restored from {:?}, {:?}", last_lat, last_long);
        }
    }

    info!("Ready for connections.");
    loop {
        if let Some(sentence) = recv_udp(&mut buff, &socket) {
            if let Some(rmc) = get_rmc(&mut parser, &sentence) {
                trace!("Processing {:?}", rmc);

                let this_lat = match rmc.latitude {
                    Some(lat) => lat,
                    None => continue,
                };

                let this_long = match rmc.longitude {
                    Some(long) => long,
                    None => continue,
                };

                if let (Some(last_lat_val), Some(last_long_val)) = (last_lat, last_long) {
                    let diff_lat: f64 = this_lat - last_lat_val;
                    let diff_long: f64 = this_long - last_long_val;

                    // about half a second in either direction ~40-50ft
                    if diff_lat.abs() < 0.0001 || diff_long.abs() < 0.0001 {
                        trace!("Location has not moved.");
                        continue;
                    }
                }

                info!("Updating location.");
                last_lat = Some(this_lat);
                last_long = Some(this_long);

                let result = if "\n" == &sentence[sentence.len() - 1..] {
                    write!(file, "{}", sentence)
                } else {
                    writeln!(file, "{}", sentence)
                };

                if result.is_err() {
                    error!(
                        "Failed to write sentence '{}' due to {:?}",
                        sentence, result
                    );
                }
            }
        }
    }
}
