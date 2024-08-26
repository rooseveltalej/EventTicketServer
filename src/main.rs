extern crate regex;
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]  // Añadir PartialEq, Eq y Hash
enum SeatState {
    Libre,
    Reservado,
    ReservadoPorUsuario,
    Comprado,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Seat {
    estado: SeatState,
}

// Enum para las categorías de zona
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]  // Añadir PartialEq, Eq y Hash
enum CategoriaZona {
    VIP,
    Regular,
    Sol,
    Platea,
}

#[derive(Debug, Serialize, Deserialize)]
struct Zone {
    nombre: String,
    categorias: HashMap<CategoriaZona, Vec<Vec<Seat>>>,  // Matrices de asientos por categoría
}

#[derive(Debug, Serialize, Deserialize)]
struct Estadio {
    zonas: Vec<Zone>,
}

impl Estadio {
    fn new() -> Self {
        // Definir las zonas con sus nombres y categorías correctos
        let zona_a = Zone {
            nombre: String::from("A"),  // Nombre de la zona
            categorias: Self::crear_categorias(),
        };

        let zona_b = Zone {
            nombre: String::from("B"),  // Nombre de la zona
            categorias: Self::crear_categorias(),
        };

        let zona_c = Zone {
            nombre: String::from("C"),  // Nombre de la zona
            categorias: Self::crear_categorias(),
        };

        let zona_d = Zone {
            nombre: String::from("D"),  // Nombre de la zona
            categorias: Self::crear_categorias(),
        };

        Estadio {
            zonas: vec![zona_a, zona_b, zona_c, zona_d],
        }
    }

    // Crear las categorías con sus matrices de asientos
    fn crear_categorias() -> HashMap<CategoriaZona, Vec<Vec<Seat>>> {
        let mut categorias = HashMap::new();
        categorias.insert(CategoriaZona::VIP, Self::crear_matriz_asientos(3, 5, vec![(0, 0, SeatState::Reservado), (1, 2, SeatState::Comprado)]));
        categorias.insert(CategoriaZona::Regular, Self::crear_matriz_asientos(3, 5, vec![(0, 1, SeatState::Libre), (2, 3, SeatState::Reservado)]));
        categorias.insert(CategoriaZona::Sol, Self::crear_matriz_asientos(3, 5, vec![(2, 2, SeatState::Comprado), (4, 4, SeatState::Libre)]));
        categorias.insert(CategoriaZona::Platea, Self::crear_matriz_asientos(3, 5, vec![(3, 3, SeatState::Libre), (2, 2, SeatState::Reservado)]));
        categorias
    }

    // Crear una matriz de asientos para una categoría específica
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
                } else if trimmed_message.starts_with("CHECK_ASIENTO") {
                    check_seat_availability(trimmed_message, &address, &clients, &estadio);
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

fn check_seat_availability(request: &str, requester: &str, clients: &ClientMap, estadio: &Arc<Mutex<Estadio>>) {
    let re = Regex::new(r#"CHECK_ASIENTO\s+"([^"]+)"\s+"([^"]+)"\s+(\d+)\s+(\d+)"#).unwrap();

    if let Some(caps) = re.captures(request) {
        let categoria = caps[1].trim_matches('"');
        let zona = caps[2].trim_matches('"');
        let fila: usize = caps[3].parse().unwrap_or(0);
        let asiento: usize = caps[4].parse().unwrap_or(0);

        let estadio = estadio.lock().expect("Failed to lock estadio mutex");

        for zon in estadio.zonas.iter() {
            if zon.nombre == zona {
                if let Some(asientos) = zon.categorias.get(&CategoriaZona::VIP) {
                    if fila > 0 && fila <= asientos.len() && asiento > 0 && asiento <= asientos[0].len() {
                        let current_seat = &asientos[fila - 1][asiento - 1];
                        if current_seat.estado == SeatState::Libre {
                            send_message_to_client(requester, clients, "ASIENTO_DISPONIBLE true\n");
                            return;
                        }
                    }
                }
            }
        }
    }
    send_message_to_client(requester, clients, "ASIENTO_DISPONIBLE false\n");
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

        let mut estadio = estadio.lock().expect("Failed to lock estadio mutex");

        let mut seat_found = false;

        for zon in estadio.zonas.iter_mut() {
            if zon.nombre == zona {
                if let Some(asientos) = zon.categorias.get_mut(&CategoriaZona::VIP) {
                    if fila > 0 && fila <= asientos.len() && asiento > 0 && asiento <= asientos[0].len() {
                        let current_seat = &mut asientos[fila - 1][asiento - 1];
                        if current_seat.estado == SeatState::Libre {
                            current_seat.estado = SeatState::ReservadoPorUsuario;  // Cambiar a ReservadoPorUsuario
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

        if !seat_found {
            send_message_to_client(requester, clients, "Asiento no encontrado o no disponible.\n");
        }
    } else if let Some(caps) = re_compra.captures(request) {
        // Procesar compra de asiento
        let categoria = caps[1].trim_matches('"');
        let zona = caps[2].trim_matches('"');
        let fila: usize = caps[3].parse().unwrap_or(0);
        let asiento: usize = caps[4].parse().unwrap_or(0);

        let mut estadio = estadio.lock().expect("Failed to lock estadio mutex");

        let mut seat_found = false;

        for zon in estadio.zonas.iter_mut() {
            if zon.nombre == zona {
                if let Some(asientos) = zon.categorias.get_mut(&CategoriaZona::VIP) {
                    if fila > 0 && fila <= asientos.len() && asiento > 0 && asiento <= asientos[0].len() {
                        let current_seat = &mut asientos[fila - 1][asiento - 1];
                        if current_seat.estado == SeatState::ReservadoPorUsuario {
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

    for zona in &estadio.zonas {
        stadium_structure.push_str(&format!("Zona: {}\n", zona.nombre));
        for (categoria, asientos) in &zona.categorias {
            stadium_structure.push_str(&format!("  Categoría: {:?}\n", categoria));
            stadium_structure.push_str("  Asientos:\n");

            for (fila_idx, fila) in asientos.iter().enumerate() {
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
