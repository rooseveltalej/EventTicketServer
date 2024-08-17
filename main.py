import subprocess

def launch_clients(num_clients):
    client_script_path = r"client.py"  # Reemplaza con la ruta al script client.py

    for i in range(num_clients):
        subprocess.Popen(["powershell", "-Command", f"python {client_script_path}"])

if __name__ == "__main__":
    launch_clients(5)  # Lanza 5 clientes
