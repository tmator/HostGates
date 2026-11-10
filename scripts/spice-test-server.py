"""Isolated SPICE/QEMU TLS fixture. No disk image and no production credentials.
Run on Linux with QEMU and OpenSSL, then set HOSTGATES_SPICE_FIXTURE to the
printed config.json path and run: cargo test --lib native_qemu -- --ignored
Ctrl+C stops QEMU, the local proxy, and removes the temporary test directory.
"""
import argparse
import json
import pathlib
import socket
import socketserver
import subprocess
import tempfile
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", help="Parent for the temporary fixture, optionally a shared Windows/WSL folder")
    parser.add_argument("--port", type=int, default=15901)
    parser.add_argument("--proxy-port", type=int, default=15902)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="hostgates-spice-test-", dir=args.directory) as directory:
        root = pathlib.Path(directory).resolve()
        commands = [
            ["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", "ca-key.pem", "-out", "ca-cert.pem", "-subj", "/CN=HostGates-test-CA", "-days", "2"],
            ["req", "-newkey", "rsa:2048", "-nodes", "-keyout", "server-key.pem", "-out", "server.csr", "-subj", "/CN=hostgates-spice-test"],
            ["x509", "-req", "-in", "server.csr", "-CA", "ca-cert.pem", "-CAkey", "ca-key.pem", "-CAcreateserial", "-out", "server-cert.pem", "-days", "2"],
        ]
        for command in commands:
            subprocess.run(["openssl", *command], cwd=root, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        (root / "password").write_text("synthetic-spice-test", encoding="utf-8")
        config = {"type": "spice", "host": "opaque-proxmox-ticket", "proxy": f"http://127.0.0.1:{args.proxy_port}", "tls-port": args.port, "host-subject": "CN=hostgates-spice-test", "ca": (root / "ca-cert.pem").read_text(), "password": "synthetic-spice-test"}
        (root / "config.json").write_text(json.dumps(config), encoding="utf-8")

        class Handler(socketserver.BaseRequestHandler):
            def handle(self):
                client = self.request
                client.settimeout(10)
                header = bytearray()
                while not header.endswith(b"\r\n\r\n"):
                    byte = client.recv(1)
                    if not byte or len(header) >= 16384:
                        return
                    header.extend(byte)
                if not header.startswith(f"CONNECT opaque-proxmox-ticket:{args.port} HTTP/1.".encode()):
                    return
                with socket.create_connection(("127.0.0.1", args.port), timeout=10) as remote:
                    client.sendall(b"HTTP/1.0 200 Connection established\r\n\r\n")
                    client.settimeout(None)
                    remote.settimeout(None)

                    def pump(source, destination):
                        try:
                            while data := source.recv(65536):
                                destination.sendall(data)
                        except OSError:
                            pass
                        finally:
                            try:
                                destination.shutdown(socket.SHUT_WR)
                            except OSError:
                                pass

                    thread = threading.Thread(target=pump, args=(client, remote), daemon=True)
                    thread.start()
                    pump(remote, client)
                    thread.join(timeout=2)

        class Proxy(socketserver.ThreadingTCPServer):
            allow_reuse_address = True
            daemon_threads = True

        with Proxy(("127.0.0.1", args.proxy_port), Handler) as proxy:
            process = subprocess.Popen(["qemu-system-x86_64", "-machine", "pc,accel=tcg", "-m", "64", "-display", "none", "-vga", "qxl", "-monitor", "none", "-object", f"secret,id=spicepass,file={root / 'password'}", "-spice", f"tls-port={args.port},addr=127.0.0.1,x509-dir={root},password-secret=spicepass,tls-channel=default"], stdout=subprocess.DEVNULL)
            try:
                print(root / "config.json", flush=True)
                thread = threading.Thread(target=proxy.serve_forever, daemon=True)
                thread.start()
                process.wait()
            except KeyboardInterrupt:
                pass
            finally:
                proxy.shutdown()
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()


if __name__ == "__main__":
    main()
