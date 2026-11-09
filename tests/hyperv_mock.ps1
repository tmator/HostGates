# Isolated contract test: remoting and Hyper-V cmdlets are replaced before execution.
$ErrorActionPreference = 'Stop'
$adapter = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '../src/hyperv.ps1')
$mock = @'
$global:mockState = 'Running'
function Import-Module { param($Name,$ErrorAction) }
function New-PSSessionOption {
    param($OpenTimeout,$OperationTimeout,[switch]$SkipCACheck,[switch]$SkipCNCheck,[switch]$SkipRevocationCheck)
    $expected = $request.skip_tls_verification -eq $true
    if ($SkipCACheck.IsPresent -ne $expected -or $SkipCNCheck.IsPresent -ne $expected -or $SkipRevocationCheck.IsPresent -ne $expected) { throw 'TLS exception scope mismatch' }
    @{ }
}
function Get-ClusterNode { [PSCustomObject]@{Name='node1'}; [PSCustomObject]@{Name='node2'} }
function Get-CimInstance { param($ClassName,$ErrorAction) [PSCustomObject]@{PartOfDomain=$true;Domain='example.test'} }
function Invoke-Command {
    param($ComputerName,$Credential,$Authentication,$ErrorAction,[switch]$UseSSL,$Port,$SessionOption,$ScriptBlock,$ArgumentList)
    if ($ComputerName -notin @('node1.example.test','node2.example.test')) { throw 'Unexpected endpoint' }
    if (-not $UseSSL -or $Port -ne 5986 -or $Authentication -ne 'Negotiate') { throw 'Transport changed' }
    $env:COMPUTERNAME = $ComputerName.Split('.')[0]
    & $ScriptBlock @ArgumentList
}
function Get-VM {
    param($Id,$ErrorAction)
    $uuid = if ($env:COMPUTERNAME -eq 'node2') { 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb' } else { 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa' }
    if ($Id -and $Id.ToString() -ne $uuid) { throw 'Wrong VM or wrong owner' }
    [PSCustomObject]@{Id=[Guid]$uuid;Name='Test VM';State=$global:mockState}
}
function Stop-VM {
    param($VM,[switch]$TurnOff,$Confirm,$ErrorAction)
    if ($VM.Id.ToString() -ne 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb') { throw 'Wrong VM shutdown' }
    if (($request.action -eq 'stop') -ne $TurnOff.IsPresent) { throw 'Unexpected hard poweroff' }
    $global:mockState = 'Off'
    if ($request.secret) { throw 'Password retained in request' }
}
function Start-VM {
    param($VM,$Confirm,$ErrorAction)
    if ($VM.Id.ToString() -ne 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb') { throw 'Wrong VM start' }
    if ($request.action -eq 'reboot' -and $global:mockState -ne 'Off') { throw 'Reboot before shutdown' }
    $global:mockState = 'Running'
}
'@
foreach ($skipCertificate in @($false,$true)) {
foreach ($action in @('list','console','start','shutdown','reboot','stop')) {
    $payload = @{url='https://node1.example.test:5986'; identity='LAB\tester'; secret="synthetic';throw 'injected"; cluster=$true; skip_tls_verification=$skipCertificate; action=$action; id='bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'} | ConvertTo-Json -Compress
    $result = $payload | & "$env:SystemRoot/System32/WindowsPowerShell/v1.0/powershell.exe" -NoLogo -NoProfile -NonInteractive -Command ($mock + "`n" + $adapter)
    if ($LASTEXITCODE -ne 0) { throw "Mock failed: $action" }
    $rows = $result | ConvertFrom-Json
    if ($action -eq 'list') {
        if ($rows.Count -ne 2) { throw ("Missing cluster VMs: " + $result) }
    } elseif ($rows.Count -ne 1 -or $rows[0].node -ne 'node2.example.test') { throw 'Owner resolution failed' }
    Write-Output "PASS Hyper-V mock $action"
}

}
