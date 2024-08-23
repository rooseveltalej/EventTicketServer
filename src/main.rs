extern crate regex;
use regex::Regex;
use serde::{Serialize, Deserialize};
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

fn handle_client(mut stream: TcpStream, clients: ClientMap, estadio: Arc<Mutex<Estadio>>) {
    let address = stream.peer_addr().unwrap().to_string();
    println!("New client connected: {}", address);

    if let Err(e) = stream.write_all(b"Bienvenido al evento de Metallica\n").and_then(|_| stream.flush()) {
        eprintln!("Error sending welcome message to {}: {}", address, e);
        return;
    }

    clients.lock().unwrap().insert(address.clone(), stream.try_clone().unwrap());

    let mut reader = BufReader::new(&mut stream);
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(bytes_read) if bytes_read > 0 => {
                let trimmed_message = buffer.trim();
                println!("Received message from {}: {}", address, trimmed_message);  // Agregado para depuración
                if trimmed_message == "GET_STADIUM_STRUCTURE" {
                    send_stadium_structure(&address, &clients, &estadio);
                } else if trimmed_message.starts_with("RESERVAR_ASIENTO") {
                    process_seat_request(trimmed_message, &address, &clients, &estadio, SeatState::Reservado);
                } else if trimmed_message.starts_with("COMPRAR_ASIENTO") {
                    process_seat_request(trimmed_message, &address, &clients, &estadio, SeatState::Comprado);
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




fn process_seat_request(request: &str, requester: &str, clients: &ClientMap, estadio: &Arc<Mutex<Estadio>>, new_state: SeatState) {
    let re = Regex::new(r#"RESERVAR_ASIENTO\s+"([^"]+)"\s+"([^"]+)"\s+(\d+)\s+(\d+)"#).unwrap();
    let re_compra = Regex::new(r#"COMPRAR_ASIENTO\s+"([^"]+)"\s+"([^"]+)"\s+(\d+)\s+(\d+)"#).unwrap();

    if let Some(caps) = re.captures(request) {
        // Procesar reserva de asiento
        let categoria = caps[1].trim_matches('"');
        let zona = caps[2].trim_matches('"');
        let fila: usize = caps[3].parse().unwrap_or(0);
        let asiento: usize = caps[4].parse().unwrap_or(0);

        println!("Procesando reserva: Categoría={}, Zona={}, Fila={}, Asiento={}", categoria, zona, fila, asiento);

        let mut estadio = estadio.lock().expect("Failed to lock estadio mutex");

        let mut seat_found = false;

        for cat in estadio.categorias.iter_mut() {
            if cat.nombre == categoria {
                for zon in cat.zonas.iter_mut() {
                    if zon.nombre == zona {
                        if fila > 0 && fila <= zon.asientos.len() && asiento > 0 && asiento <= zon.asientos[0].len() {
                            let current_seat = &mut zon.asientos[fila - 1][asiento - 1];
                            if current_seat.estado == SeatState::Libre {
                                current_seat.estado = new_state;
                                seat_found = true;
                                send_message_to_client(requester, clients, "Operación exitosa.\n");
                            } else {
                                send_message_to_client(requester, clients, "El asiento no está disponible.\n");
                            }
                        } else {
                            send_message_to_client(requester, clients, "Fila o asiento fuera de rango.\n");
                        }
                    }
                }
            }
        }

        if !seat_found {
            send_message_to_client(requester, clients, "Asiento no encontrado o no disponible.\n");
        }
    } else if let Some(caps) = re_compra.captures(request) {
        // Procesar compra de asiento
        let categoria = caps[1].trim_matches('"');
        let zona = caps[2].trim_matches('"');
        let fila: usize = caps[3].parse().unwrap_or(0);
        let asiento: usize = caps[4].parse().unwrap_or(0);

        println!("Procesando compra: Categoría={}, Zona={}, Fila={}, Asiento={}", categoria, zona, fila, asiento);

        let mut estadio = estadio.lock().expect("Failed to lock estadio mutex");

        let mut seat_found = false;

        for cat in estadio.categorias.iter_mut() {
            if cat.nombre == categoria {
                for zon in cat.zonas.iter_mut() {
                    if zon.nombre == zona {
                        if fila > 0 && fila <= zon.asientos.len() && asiento > 0 && asiento <= zon.asientos[0].len() {
                            let current_seat = &mut zon.asientos[fila - 1][asiento - 1];
                            if current_seat.estado == SeatState::Reservado {
                                current_seat.estado = new_state;
                                seat_found = true;
                                send_message_to_client(requester, clients, "Operación exitosa.\n");
                            } else {
                                send_message_to_client(requester, clients, "El asiento no está disponible para compra.\n");
                            }
                        } else {
                            send_message_to_client(requester, clients, "Fila o asiento fuera de rango.\n");
                        }
                    }
                }
            }
        }

        if !seat_found {
            send_message_to_client(requester, clients, "Asiento no encontrado o no disponible.\n");
        }
    } else {
        send_message_to_client(requester, clients, "Formato de comando incorrecto.\n");
    }
}




fn send_message_to_client(client_address: &str, clients: &ClientMap, message: &str) {
    if let Some(mut client) = clients.lock().unwrap().get(client_address) {
        if let Err(e) = client.write_all(message.as_bytes()) {
            eprintln!("Error sending message to {}: {}", client_address, e);
        }
    }
}


fn send_stadium_structure(requester: &str, clients: &ClientMap, estadio: &Arc<Mutex<Estadio>>) {
    // Bloquear el mutex para obtener acceso a los datos
    let estadio = estadio.lock().unwrap();

    let mut stadium_structure = String::new();

    for categoria in &estadio.categorias {
        stadium_structure.push_str(&format!("Categoría: {}\n", categoria.nombre));
        for zona in &categoria.zonas {
            stadium_structure.push_str(&format!("  Zona: {}\n", zona.nombre));
            stadium_structure.push_str("  Asientos:\n");

            for (fila_idx, fila) in zona.asientos.iter().enumerate() {
                let fila_str: Vec<String> = fila.iter().enumerate().map(|(col_idx, asiento)| {
                    format!("[{}, {}: {:?}]", fila_idx + 1, col_idx + 1, asiento.estado)
                }).collect();
                stadium_structure.push_str(&format!("    {}\n", fila_str.join(" | ")));
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
    let estadio = Arc::new(Mutex::new(Estadio::new()));  // Envolver estadio en un Mutex

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients = Arc::clone(&clients);
                let estadio = Arc::clone(&estadio);  // Clonar el Arc<Mutex<Estadio>>
                thread::spawn(move || handle_client(stream, clients, estadio));
            }
            Err(e) => {
                eprintln!("Failed to accept client: {}", e);
            }
        }
    }
}