import socket
import threading

def receive_messages(sock):
    while True:
        try:
            message = sock.recv(1024).decode('utf-8')
            if message:
                print(message)
            else:
                break
        except:
            break

def main():
    client_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    client_socket.connect(('127.0.0.1', 8080))

    threading.Thread(target=receive_messages, args=(client_socket,)).start()

    while True:
        message = input("Enter your message or press 1 to get pet names: ")
        if message == "1":
            client_socket.send("GET_PET_NAMES\n".encode('utf-8'))
        elif message:
            message += '\n'
            client_socket.send(message.encode('utf-8'))

if __name__ == "__main__":
    main()
