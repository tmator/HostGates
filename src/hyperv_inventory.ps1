$ErrorActionPreference='Stop';
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
