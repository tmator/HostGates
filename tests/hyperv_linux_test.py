"""Offline adapter contract tests; no real hypervisor is contacted."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import unittest

spec = importlib.util.spec_from_file_location("adapter", Path(__file__).parents[1] / "src/hyperv_linux.py")
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)
VMID = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"


class AdapterTests(unittest.TestCase):
    def setUp(self):
        self.calls = []
        self.request = dict(url="https://node1.example.test:5986", identity="LAB\\tester",
                            secret="synthetic-secret", cluster=True, action="list", id="", ca_file="")
        self.fail_action = False
        self.duplicate = False

    def factory(self, node, **kwargs):
        self.assertTrue(kwargs["ssl"])
        self.assertIs(kwargs["cert_validation"], True)
        self.assertEqual(kwargs["auth"], "ntlm")
        self.assertEqual(kwargs["reconnection_retries"], 0)
        owner = self
        class Client:
            def __enter__(self): return self
            def __exit__(self, *args): pass
            def execute_ps(self, script):
                owner.calls.append((node, script))
                owner.assertNotIn(owner.request["secret"], script)
                if script == adapter.CLUSTER:
                    return json.dumps(["node1.example.test", "node2.example.test"]), None, False
                if script == adapter.INVENTORY:
                    vms = [dict(native_id=VMID, name="VM '$ dangerous", status="running")]
                    return json.dumps(vms if node.startswith("node2") or owner.duplicate else []), None, False
                return "", None, owner.fail_action
        return Client()

    def test_cluster_console_uses_current_owner_without_mutation(self):
        self.request.update(action="console", id=VMID)
        result = adapter.execute(self.request, self.factory)
        self.assertEqual(result[0]["node"], "node2.example.test")
        self.assertEqual(len(self.calls), 3)

    def test_actions_target_owner_and_do_not_retry(self):
        for action in ("start", "shutdown", "reboot", "stop"):
            self.calls.clear()
            self.request.update(action=action, id=VMID)
            adapter.execute(self.request, self.factory)
            self.assertEqual(self.calls[-1][0], "node2.example.test")
            self.assertIn("$action='" + action + "'", self.calls[-1][1])
            self.assertNotIn("VM '$ dangerous", self.calls[-1][1])
            self.assertEqual(len(self.calls), 4)
        self.fail_action = True
        self.calls.clear()
        with self.assertRaises(ValueError): adapter.execute(self.request, self.factory)
        self.assertEqual(len(self.calls), 4)

    def test_reject_duplicate_owner_and_invalid_input(self):
        self.duplicate = True
        self.request.update(action="stop", id=VMID)
        with self.assertRaises(ValueError): adapter.execute(self.request, self.factory)
        self.assertEqual(len(self.calls), 3)
        for changes in (dict(id="'; Stop-Computer"), dict(action="delete"),
                        dict(url="http://node1:5985"), dict(url="https://user:secret@node1")):
            self.calls.clear()
            with self.assertRaises(ValueError): adapter.execute(dict(self.request, **changes), self.factory)
            self.assertFalse(self.calls)

    @unittest.skipUnless(os.name == "nt", "PowerShell syntax check on Windows")
    def test_remote_scripts_parse_in_windows_powershell(self):
        scripts = json.dumps([adapter.CLUSTER, adapter.INVENTORY, adapter.ACTION])
        result = subprocess.run(["powershell.exe", "-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "$ErrorActionPreference='Stop'; $items = [Console]::In.ReadToEnd() | ConvertFrom-Json; "
            "foreach ($item in $items) { $null = [ScriptBlock]::Create($item) }"],
            input=scripts, capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
