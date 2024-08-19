use serde::{Serialize, Deserialize};
//use serde_json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum SeatState {
    Libre,
    Reservado,
    Comprado,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Seat {
    estado: SeatState,
}

#[derive(Debug, Serialize, Deserialize)]
struct Zone {
    nombre: String,
    asientos: Vec<Vec<Seat>>,  // Matriz de asientos
}

#[derive(Debug, Serialize, Deserialize)]
struct Category {
    nombre: String,
    zonas: Vec<Zone>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Estadio {
    categorias: Vec<Category>,
}

impl Estadio {
    fn new() -> Self {
        let zona_a = Zone {
            nombre: String::from("Zona A"),
            asientos: Self::crear_matriz_asientos(3, 5, vec![(0, 0, SeatState::Reservado), (1, 2, SeatState::Comprado)]),
        };

        let zona_b = Zone {
            nombre: String::from("Zona B"),
            asientos: Self::crear_matriz_asientos(7, 4, vec![(0, 1, SeatState::Libre), (6, 3, SeatState::Reservado)]),
        };

        let zona_c = Zone {
            nombre: String::from("Zona C"),
            asientos: Self::crear_matriz_asientos(5, 5, vec![(2, 2, SeatState::Comprado), (4, 4, SeatState::Libre)]),
        };

        let zona_d = Zone {
            nombre: String::from("Zona D"),
            asientos: Self::crear_matriz_asientos(6, 6, vec![(3, 3, SeatState::Libre), (5, 2, SeatState::Reservado)]),
        };

        let categoria_a = Category {
            nombre: String::from("Categoría A"),
            zonas: vec![zona_a],
        };

        let categoria_b = Category {
            nombre: String::from("Categoría B"),
            zonas: vec![zona_b],
        };

        let categoria_c = Category {
            nombre: String::from("Categoría C"),
            zonas: vec![zona_c],
        };

        let categoria_d = Category {
            nombre: String::from("Categoría D"),
            zonas: vec![zona_d],
        };

        Estadio {
            categorias: vec![categoria_a, categoria_b, categoria_c, categoria_d],
        }
    }

    // Crear una matriz de asientos para una zona específica
    fn crear_matriz_asientos(filas: usize, asientos_por_fila: usize, estados: Vec<(usize, usize, SeatState)>) -> Vec<Vec<Seat>> {
        let mut matriz = vec![vec![Seat { estado: SeatState::Libre }; asientos_por_fila]; filas];

        for (fila, numero, estado) in estados {
            if fila < filas && numero < asientos_por_fila {
                matriz[fila][numero].estado = estado;
            }
        }

        matriz
    }
}

type ClientMap = Arc<Mutex<HashMap<String, TcpStream>>>;

fn handle_client(stream: TcpStream, clients: ClientMap, pet_names: Arc<Mutex<Vec<String>>>, estadio: Arc<Estadio>) {
    let address = stream.peer_addr().unwrap().to_string();
    println!("New client connected: {}", address);
    clients.lock().unwrap().insert(address.clone(), stream.try_clone().unwrap());

    let mut reader = BufReader::new(stream);
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(bytes_read) if bytes_read > 0 => {
                let trimmed_message = buffer.trim();
                if trimmed_message == "GET_PET_NAMES" {
                    send_pet_names(&address, &clients, &pet_names);
                } else if trimmed_message == "GET_STADIUM_STRUCTURE" {
                    send_stadium_structure(&address, &clients, &estadio);
                } else {
                    let message = format!("{}: {}", address, buffer);
                    println!("Message received: {}", message);
                    broadcast_message(&message, &clients);
                }
            }
            Ok(_) => {
                println!("Client {} disconnected", address);
                clients.lock().unwrap().remove(&address);
                break;
            }
            Err(e) => {
                eprintln!("Error reading from client {}: {}", address, e);
                break;
            }
        }
    }
}

fn send_pet_names(requester: &str, clients: &ClientMap, pet_names: &Arc<Mutex<Vec<String>>>) {
    let pet_names = pet_names.lock().unwrap();
    let names_message = format!("Pet names: {}\n", pet_names.join(", "));
    
    if let Some(mut client) = clients.lock().unwrap().get(requester) {
        if let Err(e) = client.write_all(names_message.as_bytes()) {
            eprintln!("Error sending pet names to {}: {}", requester, e);
        }
    }
}

fn send_stadium_structure(requester: &str, clients: &ClientMap, estadio: &Arc<Estadio>) {
    let estadio = &**estadio;
    let mut stadium_structure = String::new();

    for categoria in &estadio.categorias {
        stadium_structure.push_str(&format!("Categoría: {}\n", categoria.nombre));
        for zona in &categoria.zonas {
            stadium_structure.push_str(&format!("  Zona: {}\n", zona.nombre));
            
            let mut available_seats = String::new();
            let mut reserved_and_purchased_seats = String::new();

            for (fila_idx, fila) in zona.asientos.iter().enumerate() {
                let available_row: Vec<String> = fila.iter().enumerate().filter_map(|(col_idx, asiento)| {
                    if asiento.estado == SeatState::Libre {
                        Some(format!("[Fila {}, Asiento {}: Libre]", fila_idx + 1, col_idx + 1))
                    } else {
                        None
                    }
                }).collect();
                
                let reserved_and_purchased_row: Vec<String> = fila.iter().enumerate().filter_map(|(col_idx, asiento)| {
                    match asiento.estado {
                        SeatState::Reservado => Some(format!("[Fila {}, Asiento {}: Reservado]", fila_idx + 1, col_idx + 1)),
                        SeatState::Comprado => Some(format!("[Fila {}, Asiento {}: Comprado]", fila_idx + 1, col_idx + 1)),
                        _ => None,
                    }
                }).collect();

                if !available_row.is_empty() {
                    available_seats.push_str(&format!("    {}\n", available_row.join(" | ")));
                }
                
                if !reserved_and_purchased_row.is_empty() {
                    reserved_and_purchased_seats.push_str(&format!("    {}\n", reserved_and_purchased_row.join(" | ")));
                }
            }

            if !available_seats.is_empty() {
                stadium_structure.push_str("  Asientos Disponibles:\n");
                stadium_structure.push_str(&available_seats);
            }

            if !reserved_and_purchased_seats.is_empty() {
                stadium_structure.push_str("  Asientos Reservados y Comprados:\n");
                stadium_structure.push_str(&reserved_and_purchased_seats);
            }

            stadium_structure.push_str("\n");
        }
    }

    if let Some(mut client) = clients.lock().unwrap().get(requester) {
        if let Err(e) = client.write_all(stadium_structure.as_bytes()) {
            eprintln!("Error sending stadium structure to {}: {}", requester, e);
        }
    }
}




fn broadcast_message(message: &str, clients: &ClientMap) {
    let clients = clients.lock().unwrap();
    for (_address, mut client) in clients.iter() {
        if let Err(e) = client.write_all(message.as_bytes()) {
            eprintln!("Error broadcasting message to a client: {}", e);
        }
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

    let estadio = Arc::new(Estadio::new());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients = Arc::clone(&clients);
                let pet_names = Arc::clone(&pet_names);
                let estadio = Arc::clone(&estadio);
                thread::spawn(move || handle_client(stream, clients, pet_names, estadio));
            }
            Err(e) => {
                eprintln!("Failed to accept client: {}", e);
            }
        }
    }
}



