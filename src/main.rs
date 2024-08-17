use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

type ClientMap = Arc<Mutex<HashMap<String, TcpStream>>>;

fn handle_client(stream: TcpStream, clients: ClientMap, pet_names: Arc<Mutex<Vec<String>>>) {
    let address = stream.peer_addr().unwrap().to_string();
    println!("New client connected: {}", address);
    clients.lock().unwrap().insert(address.clone(), stream.try_clone().unwrap());

    let mut reader = BufReader::new(stream);
    let mut buffer = String::new();

    loop {
        buffer.clear();
        let bytes_read = reader.read_line(&mut buffer).unwrap();
        if bytes_read == 0 {
            println!("Client {} disconnected", address);
            clients.lock().unwrap().remove(&address);
            break;
        }

        let trimmed_message = buffer.trim();
        if trimmed_message == "GET_PET_NAMES" {
            send_pet_names(&address, &clients, &pet_names);
        } else {
            let message = format!("{}: {}", address, buffer);
            println!("Message received: {}", message);
            broadcast_message(&message, &clients);
        }
    }
}

fn send_pet_names(requester: &str, clients: &ClientMap, pet_names: &Arc<Mutex<Vec<String>>>) {
    let clients = clients.lock().unwrap();  // Bloquea el Mutex para acceder al HashMap
    let pet_names = pet_names.lock().unwrap();
    let names_message = format!("Pet names: {}\n", pet_names.join(", "));
    
    if let Some(mut client) = clients.get(requester) {  // Declarar 'client' como mutable
        let _ = client.write_all(names_message.as_bytes());
    }
}



fn broadcast_message(message: &str, clients: &ClientMap) {
    let clients = clients.lock().unwrap();
    for (_address, mut client) in clients.iter() {
        let _ = client.write_all(message.as_bytes());
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Server running on 127.0.0.1:8080");

    let clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    let pet_names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![
        "Fido".to_string(), 
        "Whiskers".to_string(), 
        "Buddy".to_string()
    ]));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients = Arc::clone(&clients);
                let pet_names = Arc::clone(&pet_names);
                thread::spawn(move || handle_client(stream, clients, pet_names));
            }
            Err(e) => {
                eprintln!("Failed to accept client: {}", e);
            }
        }
    }
}





