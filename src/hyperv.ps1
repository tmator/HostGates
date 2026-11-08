$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    $uri = [Uri]$request.url
    $credential = [PSCredential]::new($request.identity, (ConvertTo-SecureString $request.secret -AsPlainText -Force))
    $request.secret = $null
    $settings = @{ Credential = $credential; Authentication = 'Negotiate'; ErrorAction = 'Stop' }
    if ($uri.Scheme -eq 'https') { $settings.UseSSL = $true }
    $settings.Port = $uri.Port
    $settings.SessionOption = New-PSSessionOption -OpenTimeout 15000 -OperationTimeout 90000
    $nodes = @($uri.DnsSafeHost)
    if ($request.cluster) {
        $nodes = @(Invoke-Command -ComputerName $uri.DnsSafeHost @settings -ScriptBlock {
            Import-Module FailoverClusters -ErrorAction Stop
            $system = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop
            Get-ClusterNode | ForEach-Object {
                if ($system.PartOfDomain -and $_.Name -notlike '*.*') { $_.Name + '.' + $system.Domain } else { $_.Name }
            }
        })
        if ($nodes.Count -eq 0) { throw 'Empty cluster' }
    }
    $inventory = @()
    foreach ($node in $nodes) {
        $nodeVMs = @(Invoke-Command -ComputerName $node @settings -ScriptBlock {
            Import-Module Hyper-V -ErrorAction Stop
            @(Get-VM | ForEach-Object {
                $state = switch ($_.State.ToString()) {
                    'Running' {'running'} 'Off' {'stopped'} 'Paused' {'paused'} 'Saved' {'saved'}
                    'Disabled' {'disabled'} 'Starting' {'starting'} 'Stopping' {'stopping'} 'Saving' {'saving'} default {'unknown'}
                }
                [PSCustomObject]@{vmid=0; native_id=$_.Id.ToString(); name=$_.Name; node=$env:COMPUTERNAME; status=$state; template=0; kind='HyperV'}
            })
        })
        foreach ($item in $nodeVMs) { $item.node = $node }
        $inventory += $nodeVMs
    }
    $inventory = @($inventory | Sort-Object name,native_id)
    if ($request.action -ne 'list') {
        $selected = @($inventory | Where-Object { $_.native_id -eq $request.id })
        if ($selected.Count -ne 1) { throw 'VM moved or unavailable; refresh inventory' }
        $vm = $selected[0]
        if ($request.action -ne 'console') {
            Invoke-Command -ComputerName $vm.node @settings -ArgumentList $vm.native_id,$request.action -ScriptBlock {
                param($id,$action)
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
            } | Out-Null
        }
        [Console]::Write((ConvertTo-Json -InputObject @($vm) -Depth 4 -Compress))
    } else {
        [Console]::Write((ConvertTo-Json -InputObject @($inventory) -Depth 4 -Compress))
    }
} catch {
    # Never serialize exceptions: remoting errors may contain supplied data.
    [Console]::Error.Write('Hyper-V operation failed')
    exit 1
}
