param([string]$OutputPath = (Join-Path $env:APPDATA 'com.aitoken.desktop\claude-usage.json'))
# Only explicit statusline input is read. No credentials or conversation content.
try {
    $inputText = [Console]::In.ReadToEnd()
    if ($inputText.Length -gt 1048576) { throw 'Input too large' }
    $data = $inputText | ConvertFrom-Json
    $limits = @{}
    foreach ($key in @('five_hour', 'seven_day')) {
        $window = $data.rate_limits.$key
        if ($null -ne $window -and $null -ne $window.used_percentage) {
            $percent = [double]$window.used_percentage
            if (-not [double]::IsNaN($percent) -and $percent -ge 0 -and $percent -le 100) {
                $limits[$key] = @{ used_percentage = $percent; resets_at = $window.resets_at }
            }
        }
    }
    $payload = @{ captured_at = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds(); rate_limits = $limits }
    $directory = Split-Path -Parent $OutputPath
    [System.IO.Directory]::CreateDirectory($directory) | Out-Null
    # Per-session temporary name avoids writers sharing an intermediate file.
    $temporaryPath = Join-Path $directory ('claude-' + [guid]::NewGuid().ToString('N') + '.tmp')
    [System.IO.File]::WriteAllText($temporaryPath, ($payload | ConvertTo-Json -Depth 5 -Compress), (New-Object System.Text.UTF8Encoding($false)))
    Move-Item -LiteralPath $temporaryPath -Destination $OutputPath -Force
    $parts = foreach ($key in @('five_hour', 'seven_day')) {
        if ($limits.ContainsKey($key)) { '{0}: {1:0}% left' -f $key, (100 - $limits[$key].used_percentage) }
    }
    if ($parts) { [Console]::WriteLine(($parts -join ' | ')) } else { [Console]::WriteLine('AI Token: quota unavailable') }
} catch { [Console]::WriteLine('AI Token: statusline unavailable') }
