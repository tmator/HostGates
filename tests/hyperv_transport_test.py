"""Real pypsrp HTTP transport against an isolated TLS server, no Windows host."""
import http.server
import importlib.util
import json
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import threading
import unittest

spec = importlib.util.spec_from_file_location("adapter", Path(__file__).parents[1] / "src/hyperv_linux.py")
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


class TransportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        root = Path(cls.temp.name)
        cls.cert = str(root / "cert.pem")
        key = str(root / "key.pem")
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                        "-keyout", key, "-out", cls.cert, "-days", "1", "-subj", "/CN=localhost",
                        "-addext", "subjectAltName=DNS:localhost"], check=True, capture_output=True)
        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args): pass
            def do_POST(self):
                self.rfile.read(int(self.headers.get("Content-Length", "0")))
                cls.received += 1
                self.send_response(cls.status)
                if cls.status == 302:
                    self.send_header("Location", "https://other.invalid/wsman")
                self.send_header("Content-Length", str(len(cls.body)))
                self.end_headers()
                try: self.wfile.write(cls.body)
                except (BrokenPipeError, ConnectionResetError): pass
        cls.server = http.server.ThreadingHTTPServer(("localhost", 0), Handler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cls.cert, key)
        cls.server.socket = context.wrap_socket(cls.server.socket, server_side=True)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()
        cls.temp.cleanup()

    def setUp(self):
        type(self).received = 0
        type(self).body = b"synthetic reply"
        type(self).status = 200

    def client(self, ca):
        return adapter.secure_client("localhost", port=self.server.server_port, ssl=True,
                                     username="LAB\\test", password="synthetic-secret", auth="ntlm",
                                     cert_validation=ca, reconnection_retries=0, no_proxy=True)

    def test_trusted_ca_and_untrusted_ca(self):
        with self.client(self.cert) as client:
            self.assertEqual(client.wsman.transport.send(b"synthetic request"), self.body)
        self.assertEqual(self.received, 1)
        with self.client(True) as client:
            with self.assertRaises(Exception): client.wsman.transport.send(b"not sent")
        self.assertEqual(self.received, 1)

    def test_redirect_refused_without_following(self):
        type(self).status = 302
        with self.client(self.cert) as client:
            with self.assertRaisesRegex(ValueError, "Redirect refused"):
                client.wsman.transport.send(b"synthetic")
        self.assertEqual(self.received, 1)

    def test_response_size_is_bounded(self):
        type(self).body = b"x" * (adapter.MAX_OUTPUT + 1)
        with self.client(self.cert) as client:
            with self.assertRaisesRegex(ValueError, "Response too large"):
                client.wsman.transport.send(b"synthetic")

    def test_response_budget_is_cumulative_within_a_session(self):
        type(self).body = b"x" * (adapter.MAX_OUTPUT // 2 + 1)
        with self.client(self.cert) as client:
            self.assertEqual(len(client.wsman.transport.send(b"first")), len(self.body))
            with self.assertRaisesRegex(ValueError, "Response too large"):
                client.wsman.transport.send(b"second")

    def test_isolated_helper_reports_tls_error_without_leaking_credentials(self):
        source = Path(adapter.__file__).read_text(encoding="utf-8")
        request = dict(url=f"https://localhost:{self.server.server_port}", identity="LAB\\test",
                       secret="synthetic-secret-do-not-log", cluster=False, action="list", id="", ca_file="")
        result = subprocess.run([sys.executable, "-I", "-c", source], input=json.dumps(request),
                                text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 11)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")
        self.assertEqual(self.received, 0)


if __name__ == "__main__":
    unittest.main()
