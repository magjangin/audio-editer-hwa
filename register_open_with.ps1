# Audition-RS 를 탐색기 "연결 프로그램" 목록에 등록합니다 (현재 사용자, 관리자 권한 불필요).
# 사용법:  powershell -ExecutionPolicy Bypass -File .\register_open_with.ps1
#          (해제) powershell -ExecutionPolicy Bypass -File .\register_open_with.ps1 -Unregister
param(
    [string]$ExePath = (Join-Path $PSScriptRoot "target\release\audition_rs.exe"),
    [switch]$Unregister
)

$exts    = @(".wav", ".ogg", ".mp3", ".flac", ".m4a", ".aac")
$exeName = Split-Path $ExePath -Leaf
$appKey  = "HKCU:\Software\Classes\Applications\$exeName"
$progId  = "AuditionRS.Audio"
$progKey = "HKCU:\Software\Classes\$progId"

if ($Unregister) {
    Remove-Item $appKey  -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $progKey -Recurse -Force -ErrorAction SilentlyContinue
    foreach ($e in $exts) {
        Remove-ItemProperty "HKCU:\Software\Classes\$e\OpenWithProgids" -Name $progId -ErrorAction SilentlyContinue
    }
    Write-Host "등록 해제 완료."
    exit
}

if (-not (Test-Path $ExePath)) { Write-Error "exe 를 찾을 수 없습니다: $ExePath  (먼저 cargo build --release)"; exit 1 }
$ExePath = (Resolve-Path $ExePath).Path
$cmd = "`"$ExePath`" `"%1`""

# Applications\<exe> : "다른 앱 선택" 목록에 표시
New-Item "$appKey\shell\open\command" -Force | Out-Null
Set-ItemProperty $appKey -Name "FriendlyAppName" -Value "Audition-RS 파형 편집기"
Set-Item "$appKey\shell\open\command" -Value $cmd
New-Item "$appKey\SupportedTypes" -Force | Out-Null
foreach ($e in $exts) { Set-ItemProperty "$appKey\SupportedTypes" -Name $e -Value "" }

# ProgID + OpenWithProgids : 확장자별 "연결 프로그램" 하위 메뉴에 표시
New-Item "$progKey\shell\open\command" -Force | Out-Null
Set-Item $progKey -Value "Audition-RS 오디오 파일"
Set-ItemProperty $progKey -Name "FriendlyTypeName" -Value "Audition-RS 오디오 파일"
New-Item "$progKey\DefaultIcon" -Force | Out-Null
Set-Item "$progKey\DefaultIcon" -Value "`"$ExePath`",0"
Set-Item "$progKey\shell\open\command" -Value $cmd
foreach ($e in $exts) {
    New-Item "HKCU:\Software\Classes\$e\OpenWithProgids" -Force | Out-Null
    New-ItemProperty "HKCU:\Software\Classes\$e\OpenWithProgids" -Name $progId -Value ([byte[]]@()) -PropertyType None -Force | Out-Null
}

Write-Host "등록 완료: $ExePath"
Write-Host "탐색기에서 오디오 파일 우클릭 > 연결 프로그램 > 'Audition-RS 파형 편집기' 를 선택하세요."
