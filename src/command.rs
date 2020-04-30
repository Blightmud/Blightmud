use crate::event::Event;

pub fn parse_command(msg: &str) -> Event {
    let msg = String::from(msg);
    let lc_msg = msg.to_ascii_lowercase();
    let mut iter = lc_msg.split_whitespace();
    match iter.next() {
        Some("/connect") => {
            let p1 = iter.next();
            let p2 = iter.next();

            if p1 == None || p2 == None {
                Event::Info("[**] USAGE: /connect <host> <port>".to_string())
            } else {
                let p1 = p1.unwrap().to_string();
                if let Ok(p2) = p2.unwrap().parse::<u32>() {
                    Event::Connect(p1, p2)
                } else {
                    Event::Error("USAGE: /connect <host: String> <port: Positive number>".to_string())
                }
            }

        }
        Some("/disconnect") | Some("/dc") => Event::Disconnect,
        Some("/quit") | Some("/q") => Event::Quit,
        _ => Event::ServerInput(msg),
    }
}
