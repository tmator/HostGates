
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
