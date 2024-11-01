function sr {
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    Add-Type -AssemblyName System.Web

    $ProgressPreference = 'SilentlyContinue'

    Write-Host ">>> [崩铁] 劫持跃迁引擎启动中..." -ForegroundColor Green

    if ($args.Length -eq 0) {
        $app_data = [Environment]::GetFolderPath('ApplicationData')
        $locallow_path = "$app_data\..\LocalLow\miHoYo\崩坏：星穹铁道\"

        $log_path = "$locallow_path\Player.log"

        if (-Not [IO.File]::Exists($log_path)) {
            Write-Host ">>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服"  -ForegroundColor Red
            Write-Host ">>> [StarRail] Fail to get log_files, The script only support Windows Chinese version and China Mihoyo server."  -ForegroundColor Red
            return
        }

        $log_lines = Get-Content $log_path -First 11

        if ([string]::IsNullOrEmpty($log_lines)) {
            $log_path = "$locallow_path\Player-prev.log"

            if (-Not [IO.File]::Exists($log_path)) {
                Write-Host ">>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服"  -ForegroundColor Red
                Write-Host ">>> [StarRail] Fail to get log_files, The script only support Windows Chinese version and China Mihoyo server."  -ForegroundColor Red
                return
            }

            $log_lines = Get-Content $log_path -First 11
        }

        if (-Not [IO.File]::Exists($log_path)) {
            Write-Host ">>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服"  -ForegroundColor Red
            Write-Host ">>> [StarRail] Fail to get log_files, The script only support Windows Chinese version and China Mihoyo server."  -ForegroundColor Red
            return
        }

        $log_lines = $log_lines.split([Environment]::NewLine)

        for ($i = 0; $i -lt 10; $i++) {
            $log_line = $log_lines[$i]

            if ($log_line.startsWith("Loading player data from ")) {
                $game_path = $log_line.replace("Loading player data from ", "").replace("data.unity3d", "")
                break
            }
        }
    } else {
        $game_path = $args[0]
    }

    if ([string]::IsNullOrEmpty($game_path)) {
        Write-Host ">>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服"  -ForegroundColor Red
        Write-Host ">>> [StarRail] Fail to get log_files, The script only support Windows Chinese version and China Mihoyo server."  -ForegroundColor Red
    }

    $dir_folder = $game_path + "webCaches/"
    $dir_folders = Get-ChildItem -Path $dir_folder -Directory
    $dir_max = @(0, 0, 0, 0)
    foreach ($dir_string in $dir_folders) {
        $dir_numbers = $dir_string -split "\."
        $dir_result = @($dir_numbers[0], $dir_numbers[1], $dir_numbers[2], $dir_numbers[3])
        if ( $dir_result -gt $dir_max ) {
            $dir_max = $dir_result
        }
    }
    $game_copy_path = $dir_max.ForEach({ $_.ToString() }) -join '.'
    $game_copy_path = "$game_path/webCaches/" + $game_copy_path + "/Cache/Cache_Data/data_2"

    $copy_path = [IO.Path]::GetTempPath() + [Guid]::NewGuid().ToString()

    Copy-Item -Path $game_copy_path -Destination $copy_path
    $cache_data = Get-Content -Encoding UTF8 -Raw $copy_path
    Remove-Item -Path $copy_path

    $cache_data_split = $cache_data -split '1/0/'

    ">>> [崩铁] 日志路径: " + $log_path
    ">>> [崩铁] 日志析出路径: " + $log_lines
    ">>> [崩铁] 游戏路径: " + $game_path
    ">>> [崩铁] 正在请求用户密钥: " + $copy_path

    for ($i = $cache_data_split.Length - 1; $i -ge 0; $i--) {
        $line = $cache_data_split[$i]

        if ($line.StartsWith('http') -and $line.Contains("getGachaLog")) {
            $url = ($line -split "\0")[0]

            $res = Invoke-WebRequest -Uri $url -ContentType "application/json" -UseBasicParsing | ConvertFrom-Json

            if ($res.retcode -eq 0) {
                $uri = [Uri]$url
                $query = [Web.HttpUtility]::ParseQueryString($uri.Query)

                $keys = $query.AllKeys
                foreach ($key in $keys) {
                    # Retain required params
                    if ($key -eq "authkey") { continue }
                    if ($key -eq "authkey_ver") { continue }
                    if ($key -eq "sign_type") { continue }
                    if ($key -eq "game_biz") { continue }
                    if ($key -eq "lang") { continue }

                    $query.Remove($key)
                }

                $latest_url = ">>> [崩铁] " + $uri.Scheme + "://" + $uri.Host + $uri.AbsolutePath + "?" + $query.ToString()

                Write-Output $latest_url -ForegroundColor Green
                Set-Clipboard -Value $latest_url

                Write-Host ">>> [崩铁] 获取跃迁记录成功，请复制绿色链接，祝你抽卡必出货" -ForegroundColor Green
                Write-Host ">>> [崩铁] 链接中含有游戏账号关键信息，请勿分享给他人链接" -ForegroundColor Yellow
                return;
            }
        }
    }

    Write-Host ">>> [崩铁] 联网请求失败，请检查是否有网页连接"  -ForegroundColor Red
}

function ww {
    $logFilePath = Read-Host ">>> [鸣潮] 无法做到识别,目前需要自己手动输入路径(直接回车默认F:\Wuthering Waves\Wuthering Waves Game\Client\Saved\Logs\Client.log)"
    if ($logFilePath -eq "") {
        $logFilePath = 'F:\Wuthering Waves\Wuthering Waves Game\Client\Saved\Logs\Client.log'
    }
    $pattern = '(https://aki-gm-resources(?:-oversea)?\.aki-game\.(?:net|com).*?)"'
    $ww_content = ''
    if (Test-Path $logFilePath) {
        $content = Get-Content $logFilePath
        $matches = [regex]::Matches($content, $pattern)
        foreach ($match in $matches) {
            $ww_content = ">>> [鸣潮] " + $match.Groups[1].Value
        }
        Write-Host $ww_content -ForegroundColor Green
        Write-Host ">>> [鸣潮] 获取唤取记录成功，请复制绿色链接，祝你抽卡必出货" -ForegroundColor Green
        Write-Host ">>> [鸣潮] 链接中含有游戏账号关键信息，请勿分享给他人链接" -ForegroundColor Yellow
    } else {
        Write-Host '>>> [鸣潮] 文件不存在，轻哼打开鸣潮重试！' -ForegroundColor Red
    }
}

do {
                    "`n`n===== 作者:蓝莓小果冻 ====="
                        ">>> 输入 1 查询崩铁"
                        ">>> 输入 2 查询鸣潮"
                        ">>> 输入 3 退出脚本"
                        ">>> input 4 toggle English/Chinese"
    $input = Read-Host  ">>> 选择要查询的游戏/功能"
    switch ($input) {
        1 { sr }
        2 { ww }
        3 {}
        4 { iex(irm 'https://besh.bemly.moe/get.ps1') }
        default { Write-Host "`n>>> [派蒙] 叫你输入指定数字啊喂！！！" -ForegroundColor Yellow }
    }
} while ($input -ne 3)
