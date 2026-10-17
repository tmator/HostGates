$ErrorActionPreference = 'Continue'
Write-Output 'HostGates : diagnostic uniquement, aucune installation ni modification.'
foreach ($program in @('cargo','rustc','ssh','sftp','mstsc')) {
    $found = Get-Command $program -ErrorAction SilentlyContinue
    [pscustomobject]@{ Programme=$program; Chemin=$found.Source; Disponible=[bool]$found }
}
$localCargo = Join-Path (Split-Path $PSScriptRoot -Parent) '.tools\cargo\bin\cargo.exe'
Write-Output "Cargo local présent : $(Test-Path $localCargo)"
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path $vswhere) { & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath }
Write-Output 'WebView2 et Node.js non requis. Coffre natif vérifiable avec le test facultatif documenté.'
