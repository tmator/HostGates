"""Embedded Linux adapter. Only stdin carries credentials; stdout is bounded JSON.
Requires pypsrp >=0.8.1,<0.9. No shell, SSH, or local PowerShell required.
"""
import json
import logging
import re
import sys
import uuid
from urllib.parse import urlsplit

logging.disable(logging.CRITICAL)
MAX_OUTPUT = 4_194_304
CLUSTER = r"""$ErrorActionPreference='Stop';
$result = @(& {

            Import-Module FailoverClusters -ErrorAction Stop
            $system = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop
            Get-ClusterNode | ForEach-Object {
                if ($system.PartOfDomain -and $_.Name -notlike '*.*') { $_.Name + '.' + $system.Domain } else { $_.Name }
            }
}); ConvertTo-Json -InputObject @($result) -Compress
"""
INVENTORY = r"""$ErrorActionPreference='Stop';
$result = @(& {

            Import-Module Hyper-V -ErrorAction Stop
            @(Get-VM | ForEach-Object {
                $state = switch ($_.State.ToString()) {
                    'Running' {'running'} 'Off' {'stopped'} 'Paused' {'paused'} 'Saved' {'saved'}
                    'Disabled' {'disabled'} 'Starting' {'starting'} 'Stopping' {'stopping'} 'Saving' {'saving'} default {'unknown'}
                }
                [PSCustomObject]@{vmid=0; native_id=$_.Id.ToString(); name=$_.Name; node=$env:COMPUTERNAME; status=$state; template=0; kind='HyperV'}
            })
}); ConvertTo-Json -InputObject @($result) -Depth 4 -Compress
"""
ACTION = r"""
                Import-Module Hyper-V -ErrorAction Stop
                $vm = Get-VM -Id ([Guid]$id) -ErrorAction Stop
                switch ($action) {
                    'start' { Start-VM -VM $vm -Confirm:$false -ErrorAction Stop | Out-Null }
                    'shutdown' { Stop-VM -VM $vm -Confirm:$false -ErrorAction Stop | Out-Null }
                    'reboot' {
                        if ($vm.State -ne 'Running') { throw 'VM not running' }
                        Stop-VM -VM $vm -Confirm:$false -ErrorAction Stop | Out-Null
                        $limit = [DateTime]::UtcNow.AddSeconds(60)
                        do {
                            $vm = Get-VM -Id ([Guid]$id) -ErrorAction Stop
                            if ($vm.State -eq 'Off') { break }
                            Start-Sleep -Milliseconds 500
                        } while ([DateTime]::UtcNow -lt $limit)
                        if ($vm.State -ne 'Off') { throw 'Guest shutdown incomplete; no forced fallback' }
                        Start-VM -VM $vm -Confirm:$false -ErrorAction Stop | Out-Null
                    }
                    'stop' { Stop-VM -VM $vm -TurnOff -Confirm:$false -ErrorAction Stop | Out-Null }
                    default { throw 'Unsupported action' }
                }
"""

def execute(request, client_factory):
    uri = urlsplit(request["url"])
    if (uri.scheme != "https" or not uri.hostname or uri.username or uri.password
            or uri.path not in ("", "/") or uri.query or uri.fragment):
        raise ValueError("Invalid endpoint")
    action = request["action"]
    if action not in ("list", "console", "start", "shutdown", "reboot", "stop"):
        raise ValueError("Invalid action")
    vmid = str(uuid.UUID(request["id"])) if action != "list" else ""
    def invoke(node, script):
        if not re.fullmatch(r"[A-Za-z0-9_.:-]{1,253}", node):
            raise ValueError("Invalid node")
        with client_factory(node, port=uri.port or 5986, ssl=True,
                username=request["identity"], password=request["secret"],
                auth="ntlm", cert_validation=request.get("ca_file") or True,
                connection_timeout=15, operation_timeout=20, read_timeout=30,
                reconnection_retries=0, no_proxy=True) as client:
            output, _streams, failed = client.execute_ps(script)
            if failed or len(output.encode("utf-8")) > MAX_OUTPUT:
                raise ValueError("Remote operation failed")
            return output
    nodes = json.loads(invoke(uri.hostname, CLUSTER)) if request["cluster"] else [uri.hostname]
    if not isinstance(nodes, list) or not 1 <= len(nodes) <= 64 or len(set(nodes)) != len(nodes):
        raise ValueError("Invalid cluster")
    inventory = []
    for node in nodes:
        machines = json.loads(invoke(node, INVENTORY))
        if not isinstance(machines, list):
            raise ValueError("Invalid inventory")
        for vm in machines:
            vm["native_id"] = str(uuid.UUID(vm["native_id"]))
            vm["node"] = node
        inventory.extend(machines)
        if len(inventory) > 10000:
            raise ValueError("Inventory too large")
    if action == "list":
        return sorted(inventory, key=lambda vm: (vm["name"], vm["native_id"]))
    selected = [vm for vm in inventory if vm["native_id"] == vmid]
    if len(selected) != 1:
        raise ValueError("VM moved or unavailable")
    if action != "console":
        # Only canonical UUID and allowlisted action are interpolated; never names or credentials.
        script = "$ErrorActionPreference='Stop'; $id='" + vmid + "'; $action='" + action + "';\n" + ACTION
        invoke(selected[0]["node"], script)
    return selected


def secure_client(node, **kwargs):
    from pypsrp.client import Client
    client = Client(node, **kwargs)
    transport = client.wsman.transport
    session = transport._build_session()
    session.trust_env = False
    session.proxies = {}
    session.verify = kwargs["cert_validation"]
    send = session.send
    received_bytes = 0
    def bounded_send(request, **options):
        nonlocal received_bytes
        if request.url != transport.endpoint:
            raise ValueError("Endpoint changed")
        options.update(allow_redirects=False, stream=True, verify=kwargs["cert_validation"])
        response = send(request, **options)
        try:
            if 300 <= response.status_code < 400:
                raise ValueError("Redirect refused")
            body = bytearray()
            for chunk in response.iter_content(65536):
                if received_bytes + len(chunk) > MAX_OUTPUT:
                    raise ValueError("Response too large")
                received_bytes += len(chunk)
                body.extend(chunk)
            response._content = bytes(body)
            response._content_consumed = True
            return response
        except Exception:
            response.close()
            raise
    session.send = bounded_send
    transport.session = session
    return client


def main():
    try:
        from importlib.metadata import version
        parts = version("pypsrp").split(".")
        if parts[:2] != ["0", "8"] or int(parts[2]) < 1:
            return 10
        import pypsrp.client  # Verify dependencies before accepting a request.
    except Exception:
        return 10
    try:
        raw = sys.stdin.buffer.read(131073)
        if len(raw) > 131072:
            return 1
        request = json.loads(raw)
        raw = None
        result = json.dumps(execute(request, secure_client), ensure_ascii=True)
        request["secret"] = None
        if len(result) > MAX_OUTPUT:
            return 1
        sys.stdout.write(result)
        return 0
    except Exception as error:
        # Never print remote exceptions, authentication data or response bodies.
        from requests.exceptions import SSLError, ConnectionError
        from pypsrp.exceptions import AuthenticationError
        if isinstance(error, SSLError):
            return 11
        if isinstance(error, AuthenticationError):
            return 12
        if isinstance(error, ConnectionError):
            return 13
        return 1

if __name__ == "__main__":
    sys.exit(main())
