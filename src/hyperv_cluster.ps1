$ErrorActionPreference='Stop';
$result = @(& {

            Import-Module FailoverClusters -ErrorAction Stop
            $system = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop
            Get-ClusterNode | ForEach-Object {
                if ($system.PartOfDomain -and $_.Name -notlike '*.*') { $_.Name + '.' + $system.Domain } else { $_.Name }
            }
}); ConvertTo-Json -InputObject @($result) -Compress
