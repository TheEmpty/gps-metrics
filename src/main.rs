use nmea_parser::{gnss::RmcData, NmeaParser, ParsedMessage};
use std::net::{UdpSocket, TcpListener};
use std::io::Read;

#[allow(dead_code)]
fn recv_udp(buff: &mut [u8], socket: &UdpSocket) -> Option<String> {
    match socket.recv_from(buff) {
        Err(e) => {
            eprintln!("Failed to recv_from: {:?}", e);
            None
        }
        Ok((amt, _src)) => match std::str::from_utf8(&buff[..amt]) {
            Ok(sentence) => Some(sentence.to_string()),
            Err(e) => {
                eprintln!("Failed to parse from str: {:?}", e);
                None
            }
        },
    }
}

#[allow(dead_code)]
fn recv_tcp(buff: &mut [u8], listener: &TcpListener) -> Option<String> {
    match listener.accept() {
        Err(e) => {
            eprintln!("Failed to accept TCP connection: {:?}", e);
            None
        }
        Ok((mut socket, _addr)) => match socket.read(buff) {
            Err(e) => {
                eprintln!("Failed to read TCP connection: {:?}", e);
                None
            }
            Ok(amt) => match std::str::from_utf8(&buff[..amt]) {
                Ok(sentence) => Some(sentence.to_string()),
                Err(e) => {
                    eprintln!("Failed to parse from str: {:?}", e);
                    None
                }
            },
        },
    }
}

fn get_rmc(parser: &mut NmeaParser, sentence: &str) -> Option<RmcData> {
    match parser.parse_sentence(sentence) {
        Err(e) => {
            eprintln!("Failed to parse sentence {:?} due to {:?}", sentence, e);
            None
        }
        Ok(parsed) => match parsed {
            ParsedMessage::Rmc(rmc) => Some(rmc),
            _ => None,
        },
    }
}

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:8888").expect("Couldn't build socket");
    // let listener = TcpListener::bind("0.0.0.0:8888").expect("Failed to build listener");
    let mut parser = NmeaParser::new();
    let mut buff = [0; 100];
    let mut last_lat = None;
    let mut last_long = None;

    loop {
        if let Some(sentence) = recv_udp(&mut buff, &socket) {
            if let Some(rmc) = get_rmc(&mut parser, &sentence) {
                let this_lat = rmc.latitude.unwrap();
                let this_long = rmc.longitude.unwrap();

                if last_lat.is_some() && last_long.is_some() {
                    let diff_lat: f64 = this_lat - last_lat.unwrap();
                    let diff_long: f64 = this_long - last_long.unwrap();

                    // about half a second in either direction ~40-50ft
                    if diff_lat.abs() < 0.0001 || diff_long.abs() < 0.0001 {
                        // same location
                        continue;
                    }
                }

                last_lat = Some(this_lat);
                last_long = Some(this_long);

                if "\n" == &sentence[sentence.len() - 1..] {
                    print!("{}", sentence);
                } else {
                    println!("{}", sentence);
                }
            }
        }
    }
}
