# WinUHid 驱动安装 / 卸载 / 状态（官方驱动包 + 官方 CA，不重新签名）
#
# 用法（install / uninstall 需要管理员）：
#   powershell -NoProfile -ExecutionPolicy Bypass -File winuhid-driver.ps1 -Action status  -PackageDir <dir>
#   powershell -NoProfile -ExecutionPolicy Bypass -File winuhid-driver.ps1 -Action install -PackageDir <dir>
#   powershell -NoProfile -ExecutionPolicy Bypass -File winuhid-driver.ps1 -Action uninstall
#
# 输出约定：每个关键结论一行 `key=value`，调用方（主程序）按行匹配，不解析中文。
param(
    [ValidateSet('status', 'install', 'uninstall')]
    [string]$Action = 'status',
    [string]$PackageDir = ''
)

$ErrorActionPreference = 'Stop'

function Write-Result([string]$key, [string]$value) {
    Write-Output ("{0}={1}" -f $key, $value)
}

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-WinUHidDevice {
    return Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
        Where-Object { $_.InstanceId -like 'ROOT\WINUHID\*' } |
        Select-Object -First 1
}

# ---- 状态 ----------------------------------------------------------------
if ($Action -eq 'status') {
    $device = Get-WinUHidDevice
    if ($device) {
        Write-Result 'installed' 'true'
        Write-Result 'instance' $device.InstanceId
        Write-Result 'status' $device.Status
    } else {
        Write-Result 'installed' 'false'
    }
    exit 0
}

if (-not (Test-Admin)) {
    Write-Result 'error' 'admin_required'
    exit 2
}

# ---- 定位驱动包 ----------------------------------------------------------
if ([string]::IsNullOrWhiteSpace($PackageDir)) {
    $PackageDir = Join-Path (Split-Path -Parent $PSCommandPath) '..\..\helper\rc003\vendor\winuhid-driver'
}
if (-not (Test-Path -LiteralPath $PackageDir)) {
    Write-Result 'error' "package_dir_missing:$PackageDir"
    exit 3
}
$infPath = Join-Path $PackageDir 'WinUHidDriver.inf'
$certPath = Join-Path $PackageDir 'WinUHidCertificate.cer'
foreach ($required in @($infPath, $certPath)) {
    if (-not (Test-Path -LiteralPath $required)) {
        Write-Result 'error' "package_file_missing:$required"
        exit 3
    }
}

# ---- 卸载 ----------------------------------------------------------------
if ($Action -eq 'uninstall') {
    $device = Get-WinUHidDevice
    if ($device) {
        & pnputil.exe /remove-device $device.InstanceId | Out-Null
        Write-Result 'device_removed' $device.InstanceId
    } else {
        Write-Result 'device_removed' 'none'
    }
    # 删掉驱动包（按 INF 路径匹配 oemXX.inf，避免误删别的驱动）
    $published = & pnputil.exe /enum-drivers 2>$null
    $target = $null
    for ($i = 0; $i -lt $published.Count; $i++) {
        if ($published[$i] -match '^\s*发布名称:\s*(\S+\.inf)' -or $published[$i] -match '^\s*Published Name:\s*(\S+\.inf)') {
            $current = $Matches[1]
        }
        if ($published[$i] -match 'WinUHidDriver\.inf') {
            $target = $current
        }
    }
    if ($target) {
        & pnputil.exe /delete-driver $target /uninstall /force | Out-Null
        Write-Result 'driver_deleted' $target
    } else {
        Write-Result 'driver_deleted' 'none'
    }
    Write-Result 'cert_kept' 'true'   # 证书留在受信任发布者里：避免下次安装再弹信任确认
    Write-Result 'uninstalled' 'true'
    exit 0
}

# ---- 安装 ----------------------------------------------------------------
$device = Get-WinUHidDevice
if ($device) {
    Write-Result 'installed' 'true'
    Write-Result 'instance' $device.InstanceId
    Write-Result 'note' 'already_present'
    exit 0
}

# 1) 信任官方发布者证书（官方 CA；静默安装时避免驱动安装弹信任确认）
& certutil.exe -addstore -f TrustedPublisher $certPath | Out-Null
Write-Result 'cert_trusted' 'true'

# 2) 把驱动包放进驱动仓库
& pnputil.exe /add-driver $infPath /install | Out-Null
Write-Result 'driver_staged' 'true'

# 3) 创建 root 枚举设备并安装驱动（SetupAPI；等价于官方 MSI 的 CreateRootDevice 动作）
#    注意：整段逻辑必须写在 C# 里。PowerShell 用 [ref] 传结构体会让 P/Invoke 编组失败，
#    表现为 SetupDiCreateDeviceInfoW 返回 false 且 GetLastError=0（2026-11-17 实测）。
if (-not ('ShengjianRootDev' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class ShengjianRootDev
{
    private const uint DICD_GENERATE_ID = 0x1;
    private const uint DIF_REGISTERDEVICE = 0x19;
    private const uint SPDRP_HARDWAREID = 0x1;
    private const uint INSTALLFLAG_FORCE = 0x1;

    [StructLayout(LayoutKind.Sequential)]
    private struct SP_DEVINFO_DATA
    {
        public uint cbSize;
        public Guid ClassGuid;
        public uint DevInst;
        public IntPtr Reserved;
    }

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr SetupDiCreateDeviceInfoList(ref Guid classGuid, IntPtr hwndParent);

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool SetupDiCreateDeviceInfoW(IntPtr deviceInfoSet, string deviceName, ref Guid classGuid,
        string deviceDescription, IntPtr hwndParent, uint creationFlags, ref SP_DEVINFO_DATA deviceInfoData);

    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool SetupDiSetDeviceRegistryPropertyW(IntPtr deviceInfoSet, ref SP_DEVINFO_DATA deviceInfoData,
        uint property, byte[] propertyBuffer, uint propertyBufferSize);

    [DllImport("setupapi.dll", SetLastError = true)]
    private static extern bool SetupDiCallClassInstaller(uint installFunction, IntPtr deviceInfoSet,
        ref SP_DEVINFO_DATA deviceInfoData);

    [DllImport("setupapi.dll", SetLastError = true)]
    private static extern bool SetupDiDestroyDeviceInfoList(IntPtr deviceInfoSet);

    [DllImport("newdev.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool UpdateDriverForPlugAndPlayDevicesW(IntPtr hwndParent, string hardwareId,
        string fullInfPath, uint installFlags, out bool rebootRequired);

    /// 只给"已经存在的实例"更新驱动（不创建设备）。
    /// 用于 SetupDiCreateDeviceInfo 报 ERROR_DEVINST_ALREADY_EXISTS 的兜底：
    /// 幻影实例虽然枚举不到，但 UpdateDriver 仍可能把它唤回来。
    public static string UpdateOnly(string hardwareId, string infPath, out bool rebootRequired)
    {
        rebootRequired = false;
        if (!UpdateDriverForPlugAndPlayDevicesW(IntPtr.Zero, hardwareId, infPath, INSTALLFLAG_FORCE, out rebootRequired))
            return "update_driver_failed:" + Marshal.GetLastWin32Error();
        return null;
    }

    /// <returns>失败原因；成功返回 null。</returns>
    public static string CreateAndInstall(string hardwareId, string infPath, out bool rebootRequired)
    {
        rebootRequired = false;
        Guid classGuid = new Guid("4d36e97d-e325-11ce-bfc1-08002be10318"); // System
        IntPtr set = SetupDiCreateDeviceInfoList(ref classGuid, IntPtr.Zero);
        if (set == IntPtr.Zero || set.ToInt64() == -1)
            return "createdevicelist_failed:" + Marshal.GetLastWin32Error();

        try
        {
            var data = new SP_DEVINFO_DATA();
            data.cbSize = (uint)Marshal.SizeOf(typeof(SP_DEVINFO_DATA));
            // 设备名必须是不带 "Root\" 前缀的短名：它会被当作设备描述，实例 ID 由系统生成。
            // 传 "Root\WinUHid" 会被当成实例 ID，一旦有残留就报 ERROR_DEVINST_ALREADY_EXISTS
            // （0xE0000205，2026-11-17 实测；MoteDeck 的脚本传的是 "WinUHid"）。
            if (!SetupDiCreateDeviceInfoW(set, "WinUHid", ref classGuid, "WinUHid Virtual HID Enumerator",
                    IntPtr.Zero, DICD_GENERATE_ID, ref data))
                return "create_device_info_failed:" + Marshal.GetLastWin32Error();

            byte[] hwid = System.Text.Encoding.Unicode.GetBytes(hardwareId + "\0\0");
            if (!SetupDiSetDeviceRegistryPropertyW(set, ref data, SPDRP_HARDWAREID, hwid, (uint)hwid.Length))
                return "set_hardware_id_failed:" + Marshal.GetLastWin32Error();

            if (!SetupDiCallClassInstaller(DIF_REGISTERDEVICE, set, ref data))
                return "register_device_failed:" + Marshal.GetLastWin32Error();
        }
        finally
        {
            SetupDiDestroyDeviceInfoList(set);
        }

        if (!UpdateDriverForPlugAndPlayDevicesW(IntPtr.Zero, hardwareId, infPath, INSTALLFLAG_FORCE, out rebootRequired))
            return "update_driver_failed:" + Marshal.GetLastWin32Error();
        return null;
    }
}
"@
}

$reboot = $false
$failure = [ShengjianRootDev]::CreateAndInstall('Root\WinUHid', (Resolve-Path -LiteralPath $infPath).Path, [ref]$reboot)
if ($failure) {
    # 兜底：实例可能存在但枚举不到（ERROR_DEVINST_ALREADY_EXISTS，0xE0000205）。
    # 这时直接给已存在的实例更新驱动，成功的话设备会重新出现。
    Write-Result 'create_failed' $failure
    $reboot2 = $false
    $fallback = [ShengjianRootDev]::UpdateOnly('Root\WinUHid', (Resolve-Path -LiteralPath $infPath).Path, [ref]$reboot2)
    if ($fallback) {
        $orphan = Get-WinUHidDevice
        if ($orphan) { & pnputil.exe /remove-device $orphan.InstanceId | Out-Null }
        Write-Result 'error' $failure
        Write-Result 'fallback_error' $fallback
        exit 5
    }
    Write-Result 'fallback' 'update_only_ok'
    if ($reboot2) { Write-Result 'reboot_required' 'true' }
}Write-Result 'driver_installed' 'true'
if ($reboot) { Write-Result 'reboot_required' 'true' }

$device = Get-WinUHidDevice
if ($device) {
    Write-Result 'installed' 'true'
    Write-Result 'instance' $device.InstanceId
    exit 0
}
Write-Result 'installed' 'false'
exit 6
