$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repository = "microsoft/shell-use"
$binaryName = "shell-use.exe"

if ($env:OS -ne "Windows_NT") {
    throw "install.ps1 only supports Windows. Use install.sh on macOS or Linux."
}

$processorArchitecture = $env:PROCESSOR_ARCHITEW6432
if ([string]::IsNullOrWhiteSpace($processorArchitecture)) {
    $processorArchitecture = $env:PROCESSOR_ARCHITECTURE
}

switch ($processorArchitecture.ToUpperInvariant()) {
    "AMD64" { $architecture = "x86_64" }
    "X86_64" { $architecture = "x86_64" }
    "ARM64" { $architecture = "aarch64" }
    "AARCH64" { $architecture = "aarch64" }
    default { throw "Unsupported Windows architecture: $processorArchitecture" }
}

$target = "$architecture-pc-windows-msvc"
$asset = "shell-use-$target.zip"
$version = $env:SHELL_USE_VERSION

if ([string]::IsNullOrWhiteSpace($version) -or $version -eq "latest") {
    $releaseUrl = "https://github.com/$repository/releases/latest/download"
}
else {
    if (-not $version.StartsWith("v")) {
        $version = "v$version"
    }
    $releaseUrl = "https://github.com/$repository/releases/download/$version"
}

$downloadUrl = "$releaseUrl/$asset"
$token = $env:GITHUB_TOKEN
if ([string]::IsNullOrWhiteSpace($token)) {
    $token = $env:GH_TOKEN
}

$tempDir = Join-Path ([IO.Path]::GetTempPath()) ("shell-use-" + [Guid]::NewGuid())
$archivePath = Join-Path $tempDir $asset
$extractDir = Join-Path $tempDir "extract"

New-Item -ItemType Directory -Path $extractDir -Force | Out-Null

try {
    $request = @{
        Uri = $downloadUrl
        OutFile = $archivePath
        UseBasicParsing = $true
    }
    if (-not [string]::IsNullOrWhiteSpace($token)) {
        $request.Headers = @{ Authorization = "Bearer $token" }
    }

    Write-Host "Downloading shell-use for $target..."
    Invoke-WebRequest @request
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force

    $files = @(Get-ChildItem -LiteralPath $extractDir -File -Recurse)
    if ($files.Count -ne 1 -or $files[0].Name -ne $binaryName) {
        throw "Downloaded archive has unexpected contents."
    }

    $installDir = $env:SHELL_USE_INSTALL_DIR
    if ([string]::IsNullOrWhiteSpace($installDir)) {
        if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
            throw "LOCALAPPDATA is not set. Set SHELL_USE_INSTALL_DIR to a writable directory."
        }
        $installDir = Join-Path $env:LOCALAPPDATA "Programs\shell-use\bin"
    }

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    $destination = Join-Path $installDir $binaryName
    $stagedDestination = Join-Path $installDir ".$binaryName.tmp"

    Copy-Item -LiteralPath $files[0].FullName -Destination $stagedDestination -Force
    Unblock-File -LiteralPath $stagedDestination
    Move-Item -LiteralPath $stagedDestination -Destination $destination -Force

    Write-Host "Installed shell-use to $destination"

    function Test-PathEntry {
        param(
            [string]$PathValue,
            [string]$Entry
        )

        if ([string]::IsNullOrWhiteSpace($PathValue)) {
            return $false
        }

        $normalizedEntry = $Entry.TrimEnd("\")
        foreach ($pathEntry in $PathValue.Split(";")) {
            if ($pathEntry.Trim().TrimEnd("\") -ieq $normalizedEntry) {
                return $true
            }
        }
        return $false
    }

    function Publish-EnvironmentChange {
        if (-not ("ShellUseInstaller.NativeMethods" -as [Type])) {
            Add-Type -Namespace ShellUseInstaller -Name NativeMethods -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd,
    uint Msg,
    UIntPtr wParam,
    string lParam,
    uint fuFlags,
    uint uTimeout,
    out UIntPtr lpdwResult);
'@
        }

        $result = [UIntPtr]::Zero
        [ShellUseInstaller.NativeMethods]::SendMessageTimeout(
            [IntPtr]0xffff,
            0x1a,
            [UIntPtr]::Zero,
            "Environment",
            2,
            5000,
            [ref]$result
        ) | Out-Null
    }

    function Get-UserPath {
        $environmentKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment")
        if ($null -eq $environmentKey) {
            return $null
        }

        try {
            return $environmentKey.GetValue(
                "Path",
                $null,
                [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
            )
        }
        finally {
            $environmentKey.Dispose()
        }
    }

    function Set-UserPath {
        param([string]$Value)

        $environmentKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
        if ($null -eq $environmentKey) {
            throw "Could not open the current user's environment registry key."
        }

        try {
            $valueKind = [Microsoft.Win32.RegistryValueKind]::String
            if ($Value.Contains("%")) {
                $valueKind = [Microsoft.Win32.RegistryValueKind]::ExpandString
            }
            elseif ($null -ne $environmentKey.GetValue("Path")) {
                $valueKind = $environmentKey.GetValueKind("Path")
            }

            $environmentKey.SetValue("Path", $Value, $valueKind)
        }
        finally {
            $environmentKey.Dispose()
        }

        Publish-EnvironmentChange
    }

    if (-not (Test-PathEntry -PathValue $env:Path -Entry $installDir)) {
        $env:Path = "$installDir;$env:Path"
    }

    $skipPathUpdate = $env:SHELL_USE_NO_MODIFY_PATH -match "^(1|true|yes)$"
    if (-not $skipPathUpdate) {
        $userPath = Get-UserPath
        if (-not (Test-PathEntry -PathValue $userPath -Entry $installDir)) {
            if ([string]::IsNullOrWhiteSpace($userPath)) {
                $newUserPath = $installDir
            }
            else {
                $newUserPath = "$installDir;$userPath"
            }
            Set-UserPath -Value $newUserPath
            Write-Host "Added $installDir to PATH. Restart your shell."
        }
    }
    elseif (-not (Test-PathEntry -PathValue (Get-UserPath) -Entry $installDir)) {
        Write-Host "Add $installDir to PATH."
    }
}
finally {
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
