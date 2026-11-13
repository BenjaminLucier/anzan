# Installs Anzan on Windows from its GitHub Releases.
#
#   irm https://github.com/BenjaminLucier/anzan/releases/latest/download/install.ps1 | iex
#
# Set ANZAN_VERSION (such as "0.2.0") for a particular release instead of the
# latest, and ANZAN_INSTALL_DIR to install somewhere other than
# %LOCALAPPDATA%\Programs\anzan. That folder is added to your user PATH.
# To uninstall, delete the folder and remove it from PATH.

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'  # Invoke-WebRequest is far slower with it
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    $repo = 'BenjaminLucier/anzan'

    # A 32-bit or emulated shell reports its own architecture; the real one is in W6432.
    $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $target = switch ($arch) {
        'AMD64' { 'x86_64-pc-windows-msvc' }
        'ARM64' { 'aarch64-pc-windows-msvc' }
        default { throw "Anzan has no Windows build for the $arch architecture." }
    }

    $base = if ($env:ANZAN_VERSION) {
        "https://github.com/$repo/releases/download/v$($env:ANZAN_VERSION.TrimStart('v'))"
    } else {
        "https://github.com/$repo/releases/latest/download"
    }
    $dir = if ($env:ANZAN_INSTALL_DIR) { $env:ANZAN_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\anzan' }

    $archive = "anzan-$target.zip"
    $temp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $temp | Out-Null
    try {
        Write-Host "Downloading $base/$archive"
        $zip = Join-Path $temp $archive
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$archive" -OutFile $zip
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$archive.sha256" -OutFile "$zip.sha256"

        $expected = ((Get-Content "$zip.sha256" -Raw).Trim() -split '\s+')[0]
        $actual = (Get-FileHash -Algorithm SHA256 $zip).Hash
        if ($actual -ne $expected) {
            throw "Checksum mismatch for ${archive}: expected $expected, got $actual"
        }

        Expand-Archive -Path $zip -DestinationPath (Join-Path $temp 'x')
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Copy-Item -Force (Join-Path $temp 'x\anzan.exe') (Join-Path $dir 'anzan.exe')
    } finally {
        Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
    }

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = if ($userPath) { $userPath -split ';' | Where-Object { $_ } } else { @() }
    if ($entries -notcontains $dir) {
        [Environment]::SetEnvironmentVariable('Path', (@($entries) + $dir) -join ';', 'User')
        $env:Path = "$env:Path;$dir"
        Write-Host "Added $dir to your PATH. Open a new terminal for other windows to see it."
    }

    $version = & (Join-Path $dir 'anzan.exe') --version
    Write-Host "Installed $version to $dir. Run 'anzan' to start."
}
