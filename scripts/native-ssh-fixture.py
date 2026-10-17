"""Ephemeral loopback SSH/SFTP/forwarding fixture, never runs a shell."""
import hashlib
import os
import pathlib
import socket
import sys
import threading
import time
import paramiko

base = pathlib.Path(sys.argv[1])
root = base / "remote"
root.mkdir()
profile = sys.argv[2] if len(sys.argv) > 2 else "rsa"
if profile == "ed25519":
    import io
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from cryptography.hazmat.primitives import serialization
    private = Ed25519PrivateKey.generate().private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.OpenSSH, serialization.NoEncryption())
    key = paramiko.Ed25519Key.from_private_key(io.StringIO(private.decode("ascii")))
elif profile == "ecdsa":
    key = paramiko.ECDSAKey.generate()
else:
    key = paramiko.RSAKey.generate(2048)
(base / "fingerprint").write_text(hashlib.sha256(key.asbytes()).hexdigest(), encoding="ascii")

class Files(paramiko.SFTPServerInterface):
    def local(self, path):
        p = (root / path.lstrip("/")).resolve()
        if not p.is_relative_to(root.resolve()):
            raise PermissionError("outside fixture")
        return p

    def canonicalize(self, path):
        return "/" + self.local(path).relative_to(root.resolve()).as_posix().removeprefix(".")

    def list_folder(self, path):
        try:
            result = []
            for p in self.local(path).iterdir():
                attr = paramiko.SFTPAttributes.from_stat(p.lstat())
                attr.filename = p.name
                result.append(attr)
            return result
        except OSError as e:
            return paramiko.SFTPServer.convert_errno(e.errno)

    def stat(self, path):
        try:
            return paramiko.SFTPAttributes.from_stat(self.local(path).stat())
        except OSError as e:
            return paramiko.SFTPServer.convert_errno(e.errno)

    def lstat(self, path):
        return self.stat(path)

    def mkdir(self, path, attr):
        try:
            self.local(path).mkdir()
            return paramiko.SFTP_OK
        except OSError as e:
            return paramiko.SFTPServer.convert_errno(e.errno)

    def open(self, path, flags, attr):
        try:
            fd = os.open(self.local(path), flags | getattr(os, "O_BINARY", 0), 0o600)
            file = os.fdopen(fd, "wb" if flags & os.O_WRONLY else "rb")
            handle = paramiko.SFTPHandle(flags)
            handle.readfile = file
            handle.writefile = file
            return handle
        except OSError as e:
            return paramiko.SFTPServer.convert_errno(e.errno)

class Server(paramiko.ServerInterface):
    def __init__(self):
        self.destinations = {}

    def get_allowed_auths(self, username):
        return "password"

    def check_auth_password(self, username, password):
        return paramiko.AUTH_SUCCESSFUL if (username, password) == ("fixture", "fixture-secret") else paramiko.AUTH_FAILED

    def check_channel_request(self, kind, chanid):
        return paramiko.OPEN_SUCCEEDED if kind == "session" else paramiko.OPEN_FAILED_ADMINISTRATIVELY_PROHIBITED

    def check_channel_direct_tcpip_request(self, chanid, origin, destination):
        if destination[0] != "127.0.0.1":
            return paramiko.OPEN_FAILED_ADMINISTRATIVELY_PROHIBITED
        self.destinations[chanid] = destination
        return paramiko.OPEN_SUCCEEDED

    def check_channel_pty_request(self, *args):
        return True

    def check_channel_window_change_request(self, *args):
        return True

    def check_channel_shell_request(self, channel):
        def echo():
            try:
                channel.sendall(b"NATIVE_SSH_READY\r\n")
                while data := channel.recv(8192):
                    channel.sendall(data)
            except (OSError, EOFError):
                pass
        threading.Thread(target=echo, daemon=True).start()
        return True

def serve(client):
    try:
        with paramiko.Transport(client) as t:
            # No moduli file in this fixture: do not advertise group exchange.
            t.get_security_options().kex = tuple(k for k in t.get_security_options().kex if "group-exchange" not in k)
            t.add_server_key(key)
            t.set_subsystem_handler("sftp", paramiko.SFTPServer, Files)
            server = Server()
            t.start_server(server=server)
            channels = []
            while t.is_active():
                channel = t.accept(0.1)
                if channel is None:
                    continue
                channels.append(channel)
                dest = server.destinations.get(channel.chanid)
                if dest:
                    def forward(ch, address):
                        try:
                            with socket.create_connection(address, timeout=5) as sock:
                                def incoming():
                                    try:
                                        while data := sock.recv(8192):
                                            ch.sendall(data)
                                    except (OSError, EOFError):
                                        pass
                                    finally:
                                        ch.shutdown_write()
                                thread = threading.Thread(target=incoming, daemon=True)
                                thread.start()
                                while data := ch.recv(8192):
                                    sock.sendall(data)
                                sock.shutdown(socket.SHUT_WR)
                                thread.join(5)
                        except (OSError, EOFError):
                            pass
                        finally:
                            ch.close()
                    threading.Thread(target=forward, args=(channel, dest), daemon=True).start()
    except (paramiko.SSHException, EOFError, OSError):
        pass

with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    listener.listen(32)
    listener.settimeout(0.2)
    (base / "port").write_text(str(listener.getsockname()[1]), encoding="ascii")
    end = time.monotonic() + 180
    while time.monotonic() < end and not (base / "stop").exists():
        try:
            client, _ = listener.accept()
        except socket.timeout:
            continue
        threading.Thread(target=serve, args=(client,), daemon=True).start()
