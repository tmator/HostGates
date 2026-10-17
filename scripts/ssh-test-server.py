"""Loopback-only test fixture. No OS shell and no real user credentials.

Run with Paramiko in PYTHONPATH and one output directory argument.
The fixture exits after 120 seconds or two clients. Private keys stay in RAM.
"""
import pathlib
import socket
import sys
import threading
import time
import paramiko

output = pathlib.Path(sys.argv[1])
output.mkdir(parents=True, exist_ok=True)
key = paramiko.RSAKey.generate(2048)
wrong = paramiko.RSAKey.generate(2048)


class Server(paramiko.ServerInterface):
    def __init__(self):
        self.shell = threading.Event()

    def get_allowed_auths(self, username):
        return "password"

    def check_auth_password(self, username, password):
        return paramiko.AUTH_SUCCESSFUL if username == "hostgates-test" and password == "test-fixture-only" else paramiko.AUTH_FAILED

    def check_channel_request(self, kind, chanid):
        return paramiko.OPEN_SUCCEEDED if kind == "session" else paramiko.OPEN_FAILED_ADMINISTRATIVELY_PROHIBITED

    def check_channel_pty_request(self, *args):
        return True

    def check_channel_window_change_request(self, *args):
        return True

    def check_channel_shell_request(self, channel):
        self.shell.set()
        return True


with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    listener.listen(2)
    listener.settimeout(120)
    port = listener.getsockname()[1]
    (output / "wrong_known_hosts").write_text(f"[127.0.0.1]:{port} {wrong.get_name()} {wrong.get_base64()}\n", encoding="ascii")
    (output / "port").write_text(str(port), encoding="ascii")
    print(f"Loopback SSH fixture ready on port {port}", flush=True)
    for _ in range(2):
        try:
            client, _ = listener.accept()
        except socket.timeout:
            break
        with paramiko.Transport(client) as transport:
            transport.add_server_key(key)
            server = Server()
            try:
                transport.start_server(server=server)
                channel = transport.accept(20)
                if channel is None or not server.shell.wait(10):
                    continue
                channel.settimeout(20)
                channel.sendall("\x1b[32mHOSTGATES_SSH_OK été\x1b[0m\r\n".encode())
                data = bytearray()
                while len(data) < 1024:
                    chunk = channel.recv(128)
                    if not chunk:
                        break
                    data.extend(chunk)
                    if b"hostgates-check" in data:
                        channel.sendall(b"\r\nHOSTGATES_INPUT_OK\r\n")
                        channel.send_exit_status(0)
                        time.sleep(0.2)
                        channel.close()
                        break
            except (EOFError, OSError, paramiko.SSHException):
                pass  # Expected when the client rejects the deliberately wrong host key.
