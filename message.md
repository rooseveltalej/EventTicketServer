# Descripción General del Código

Este código implementa un servidor TCP que gestiona la reserva y compra de asientos en un estadio. Utiliza varias bibliotecas de Rust, como `regex` para manejar expresiones regulares, `serde` para serialización y deserialización, y `std::sync` para manejo de concurrencia.

## Estructuras de Datos Principales

1. **SeatState**: Enum que representa el estado de un asiento (Libre, Reservado, ReservadoPorUsuario, Comprado).
2. **Seat**: Estructura que contiene el estado de un asiento.
3. **CategoriaZona**: Enum que representa las diferentes categorías de zonas en el estadio (VIP, Regular, Sol, Platea).
4. **Zone**: Estructura que contiene el nombre de la zona y un mapa de categorías a matrices de asientos.
5. **Estadio**: Estructura que contiene una lista de zonas.

## Inicialización del Estadio

El estadio se inicializa con cuatro zonas (A, B, C, D), cada una con sus respectivas categorías y matrices de asientos. La función `crear_categorias` crea estas matrices y asigna estados iniciales a algunos asientos.

## Algoritmo de Búsqueda de Asientos

El algoritmo de búsqueda de asientos se encuentra en la función `check_seat_availability`. Aquí está el desglose:

1. **Regex**: Se utiliza una expresión regular para extraer los parámetros de la solicitud (categoría, zona, fila, asiento).
2. **Bloqueo del Mutex**: Se bloquea el mutex para obtener acceso seguro a los datos del estadio.
3. **Iteración sobre Zonas**: Se itera sobre las zonas del estadio para encontrar la zona especificada.
4. **Verificación de Disponibilidad**: Dentro de la zona, se verifica si el asiento especificado está libre.
5. **Respuesta al Cliente**: Se envía un mensaje al cliente indicando si el asiento está disponible o no.

## Ejemplo de Uso

Si un cliente envía la solicitud `CHECK_ASIENTO "VIP" "A" 1 1`, el servidor:

1. Extrae los parámetros usando la expresión regular.
2. Bloquea el mutex para acceder al estadio.
3. Busca la zona "A" y la categoría "VIP".
4. Verifica si el asiento en la fila 1, columna 1 está libre.
5. Envía una respuesta al cliente indicando la disponibilidad del asiento.

## Funciones Adicionales

- **process_seat_request**: Maneja solicitudes de reserva, compra y liberación de asientos.
- **send_stadium_structure**: Envía la estructura del estadio al cliente.
- **broadcast_message**: Envía un mensaje a todos los clientes conectados.