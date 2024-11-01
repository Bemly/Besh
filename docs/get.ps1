
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

function sr {
    
    Add-Type -AssemblyName System.Web

    $ProgressPreference = 'SilentlyContinue'

    # >>> [崩铁] 劫持跃迁引擎启动中...
    Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x52ab)$([char]0x6301)$([char]0x8dc3)$([char]0x8fc1)$([char]0x5f15)$([char]0x64ce)$([char]0x542f)$([char]0x52a8)$([char]0x4e2d)" -ForegroundColor Green

    if ($args.Length -eq 0) {
        $app_data = [Environment]::GetFolderPath('ApplicationData')
        # 崩坏：星穹铁道
        $locallow_path = "$app_data\..\LocalLow\miHoYo\$([char]0x5d29)$([char]0x574f)$([char]0xff1a)$([char]0x661f)$([char]0x7a79)$([char]0x94c1)$([char]0x9053)\"

        $log_path = "$locallow_path\Player.log"

        if (-Not [IO.File]::Exists($log_path)) {
            # >>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服
            Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x83b7)$([char]0x53d6)$([char]0x65e5)$([char]0x5fd7)$([char]0x6587)$([char]0x4ef6)$([char]0x5931)$([char]0x8d25)$([char]0xff0c)$([char]0x672c)$([char]0x811a)$([char]0x672c)$([char]0x4ec5)$([char]0x652f)$([char]0x6301)$([char]0x7b80)$([char]0x4f53)$([char]0x4e2d)$([char]0x6587)$([char]0x7cfb)$([char]0x7edf)$([char]0x548c)$([char]0x533a)$([char]0x670d)"  -ForegroundColor Red
            Write-Host ">>> [StarRail] Fail to get log_files, The script only support Windows Chinese version and China Mihoyo server."  -ForegroundColor Red
            return
        }

        $log_lines = Get-Content $log_path -First 11

        if ([string]::IsNullOrEmpty($log_lines)) {
            $log_path = "$locallow_path\Player-prev.log"

            if (-Not [IO.File]::Exists($log_path)) {
                # >>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服
                Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x83b7)$([char]0x53d6)$([char]0x65e5)$([char]0x5fd7)$([char]0x6587)$([char]0x4ef6)$([char]0x5931)$([char]0x8d25)$([char]0xff0c)$([char]0x672c)$([char]0x811a)$([char]0x672c)$([char]0x4ec5)$([char]0x652f)$([char]0x6301)$([char]0x7b80)$([char]0x4f53)$([char]0x4e2d)$([char]0x6587)$([char]0x7cfb)$([char]0x7edf)$([char]0x548c)$([char]0x533a)$([char]0x670d)"  -ForegroundColor Red
                Write-Host ">>> [StarRail] Fail to get log_files, The script only support Windows Chinese version and China Mihoyo server."  -ForegroundColor Red
                return
            }

            $log_lines = Get-Content $log_path -First 11
        }

        if (-Not [IO.File]::Exists($log_path)) {
            # >>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服
            Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x83b7)$([char]0x53d6)$([char]0x65e5)$([char]0x5fd7)$([char]0x6587)$([char]0x4ef6)$([char]0x5931)$([char]0x8d25)$([char]0xff0c)$([char]0x672c)$([char]0x811a)$([char]0x672c)$([char]0x4ec5)$([char]0x652f)$([char]0x6301)$([char]0x7b80)$([char]0x4f53)$([char]0x4e2d)$([char]0x6587)$([char]0x7cfb)$([char]0x7edf)$([char]0x548c)$([char]0x533a)$([char]0x670d)"  -ForegroundColor Red
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
        # >>> [崩铁] 获取日志文件失败，本脚本仅支持简体中文系统和区服
        Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x83b7)$([char]0x53d6)$([char]0x65e5)$([char]0x5fd7)$([char]0x6587)$([char]0x4ef6)$([char]0x5931)$([char]0x8d25)$([char]0xff0c)$([char]0x672c)$([char]0x811a)$([char]0x672c)$([char]0x4ec5)$([char]0x652f)$([char]0x6301)$([char]0x7b80)$([char]0x4f53)$([char]0x4e2d)$([char]0x6587)$([char]0x7cfb)$([char]0x7edf)$([char]0x548c)$([char]0x533a)$([char]0x670d)"  -ForegroundColor Red
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

    # ">>> [崩铁] 日志路径: " + $log_path
    # ">>> [崩铁] 日志析出路径: " + $log_lines
    # ">>> [崩铁] 游戏路径: " + $game_path
    # ">>> [崩铁] 正在请求用户密钥: " + $copy_path
    ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x65e5)$([char]0x5fd7)$([char]0x8def)$([char]0x5f84): " + $log_path
    ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x65e5)$([char]0x5fd7)$([char]0x6790)$([char]0x51fa)$([char]0x8def)$([char]0x5f84): " + $log_lines
    ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x6e38)$([char]0x620f)$([char]0x8def)$([char]0x5f84): " + $game_path
    ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x6b63)$([char]0x5728)$([char]0x8bf7)$([char]0x6c42)$([char]0x7528)$([char]0x6237)$([char]0x5bc6)$([char]0x94a5): " + $copy_path

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

                $latest_url = $uri.Scheme + "://" + $uri.Host + $uri.AbsolutePath + "?" + $query.ToString()

                Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] " $latest_url -ForegroundColor Green
                Set-Clipboard -Value $latest_url

                # Write-Host ">>> [崩铁] 获取跃迁记录成功，已经自动复制，如果没有请手动复制绿色链接，祝你抽卡必出货" -ForegroundColor Green
                # Write-Host ">>> [崩铁] 链接中含有游戏账号关键信息，请勿分享给他人链接" -ForegroundColor Yellow
                Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x83b7)$([char]0x53d6)$([char]0x8dc3)$([char]0x8fc1)$([char]0x8bb0)$([char]0x5f55)$([char]0x6210)$([char]0x529f)$([char]0xff0c)$([char]0x5df2)$([char]0x7ecf)$([char]0x81ea)$([char]0x52a8)$([char]0x590d)$([char]0x5236)$([char]0xff0c)$([char]0x5982)$([char]0x679c)$([char]0x6ca1)$([char]0x6709)$([char]0x8bf7)$([char]0x624b)$([char]0x52a8)$([char]0x590d)$([char]0x5236)$([char]0x7eff)$([char]0x8272)$([char]0x94fe)$([char]0x63a5)$([char]0xff0c)$([char]0x795d)$([char]0x4f60)$([char]0x62bd)$([char]0x5361)$([char]0x5fc5)$([char]0x51fa)$([char]0x8d27)" -ForegroundColor Green
                Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x94fe)$([char]0x63a5)$([char]0x4e2d)$([char]0x542b)$([char]0x6709)$([char]0x6e38)$([char]0x620f)$([char]0x8d26)$([char]0x53f7)$([char]0x5173)$([char]0x952e)$([char]0x4fe1)$([char]0x606f)$([char]0xff0c)$([char]0x8bf7)$([char]0x52ff)$([char]0x5206)$([char]0x4eab)$([char]0x7ed9)$([char]0x4ed6)$([char]0x4eba)$([char]0x94fe)$([char]0x63a5)" -ForegroundColor Yellow
                return;
            }
        }
    }

    # Write-Host ">>> [崩铁] 联网请求失败，请检查是否有网页连接"  -ForegroundColor Red
    Write-Host ">>> [$([char]0x5d29)$([char]0x94c1)] $([char]0x8054)$([char]0x7f51)$([char]0x8bf7)$([char]0x6c42)$([char]0x5931)$([char]0x8d25)$([char]0xff0c)$([char]0x8bf7)$([char]0x68c0)$([char]0x67e5)$([char]0x662f)$([char]0x5426)$([char]0x6709)$([char]0x7f51)$([char]0x9875)$([char]0x8fde)$([char]0x63a5)"  -ForegroundColor Red
}

function ww {
    # $logFilePath = Read-Host ">>> [鸣潮] 无法做到识别,目前需要自己手动输入路径(直接回车默认F:\Wuthering Waves\Wuthering Waves Game\Client\Saved\Logs\Client.log)"
    $logFilePath = Read-Host ">>> [$([char]0x9e23)$([char]0x6f6e)] $([char]0x65e0)$([char]0x6cd5)$([char]0x505a)$([char]0x5230)$([char]0x8bc6)$([char]0x522b),$([char]0x76ee)$([char]0x524d)$([char]0x9700)$([char]0x8981)$([char]0x81ea)$([char]0x5df1)$([char]0x624b)$([char]0x52a8)$([char]0x8f93)$([char]0x5165)$([char]0x8def)$([char]0x5f84)($([char]0x76f4)$([char]0x63a5)$([char]0x56de)$([char]0x8f66)$([char]0x9ed8)$([char]0x8ba4)F:\Wuthering Waves\Wuthering Waves Game\Client\Saved\Logs\Client.log)"
    if ($logFilePath -eq "") {
        $logFilePath = 'F:\Wuthering Waves\Wuthering Waves Game\Client\Saved\Logs\Client.log'
    }
    $pattern = '(https://aki-gm-resources(?:-oversea)?\.aki-game\.(?:net|com).*?)"'
    $latest_url = ''
    if (Test-Path $logFilePath) {
        $content = Get-Content $logFilePath
        $matches_value = [regex]::Matches($content, $pattern)
        foreach ($match in $matches_value) {
            $latest_url = $match.Groups[1].Value
        }
        if ($latest_url -eq '') {
            # Write-Host '>>> [鸣潮] 文件不存在，轻哼打开鸣潮重试！' -ForegroundColor Red
            Write-Host '>>> [$([char]0x9e23)$([char]0x6f6e)] $([char]0x6587)$([char]0x4ef6)$([char]0x4e0d)$([char]0x5b58)$([char]0x5728)$([char]0xff0c)$([char]0x8f7b)$([char]0x54fc)$([char]0x6253)$([char]0x5f00)$([char]0x9e23)$([char]0x6f6e)$([char]0x91cd)$([char]0x8bd5)$([char]0xff01)' -ForegroundColor Red
        } else {
            Set-Clipboard -Value $latest_url
            Write-Host ">>> [$([char]0x9e23)$([char]0x6f6e)] " $latest_url -ForegroundColor Green
            # Write-Host ">>> [鸣潮] 获取唤取记录成功，已经自动复制，如果没有请手动复制绿色链接，祝你抽卡必出货" -ForegroundColor Green
            # Write-Host ">>> [鸣潮] 链接中含有游戏账号关键信息，请勿分享给他人链接" -ForegroundColor Yellow
            Write-Host ">>> [$([char]0x9e23)$([char]0x6f6e)] $([char]0x83b7)$([char]0x53d6)$([char]0x5524)$([char]0x53d6)$([char]0x8bb0)$([char]0x5f55)$([char]0x6210)$([char]0x529f)$([char]0xff0c)$([char]0x5df2)$([char]0x7ecf)$([char]0x81ea)$([char]0x52a8)$([char]0x590d)$([char]0x5236)$([char]0xff0c)$([char]0x5982)$([char]0x679c)$([char]0x6ca1)$([char]0x6709)$([char]0x8bf7)$([char]0x624b)$([char]0x52a8)$([char]0x590d)$([char]0x5236)$([char]0x7eff)$([char]0x8272)$([char]0x94fe)$([char]0x63a5)$([char]0xff0c)$([char]0x795d)$([char]0x4f60)$([char]0x62bd)$([char]0x5361)$([char]0x5fc5)$([char]0x51fa)$([char]0x8d27)" -ForegroundColor Green
            Write-Host ">>> [$([char]0x9e23)$([char]0x6f6e)] $([char]0x94fe)$([char]0x63a5)$([char]0x4e2d)$([char]0x542b)$([char]0x6709)$([char]0x6e38)$([char]0x620f)$([char]0x8d26)$([char]0x53f7)$([char]0x5173)$([char]0x952e)$([char]0x4fe1)$([char]0x606f)$([char]0xff0c)$([char]0x8bf7)$([char]0x52ff)$([char]0x5206)$([char]0x4eab)$([char]0x7ed9)$([char]0x4ed6)$([char]0x4eba)$([char]0x94fe)$([char]0x63a5)" -ForegroundColor Yellow
        }
    } else {
        # Write-Host '>>> [鸣潮] 文件不存在，轻哼打开鸣潮重试！' -ForegroundColor Red
        Write-Host ">>> [$([char]0x9e23)$([char]0x6f6e)] $([char]0x6587)$([char]0x4ef6)$([char]0x4e0d)$([char]0x5b58)$([char]0x5728)$([char]0xff0c)$([char]0x8f7b)$([char]0x54fc)$([char]0x6253)$([char]0x5f00)$([char]0x9e23)$([char]0x6f6e)$([char]0x91cd)$([char]0x8bd5)$([char]0xff01)" -ForegroundColor Red
    }
}

do {
    #                 "`n`n===== 作者:蓝莓小果冻 ====="
    #                     ">>> 输入 1 查询崩铁"
    #                     ">>> 输入 2 查询鸣潮"
    #                     ">>> 输入 3 退出脚本"
    #                     ">>> input 4 toggle English/Chinese"
    # $input_use = Read-Host  ">>> 选择要查询的游戏/功能"
                        "`n`n===== $([char]0x4f5c)$([char]0x8005):$([char]0x84dd)$([char]0x8393)$([char]0x5c0f)$([char]0x679c)$([char]0x51bb) ====="
                            ">>> $([char]0x8f93)$([char]0x5165) 1 $([char]0x67e5)$([char]0x8be2)$([char]0x5d29)$([char]0x94c1)"
                            ">>> $([char]0x8f93)$([char]0x5165) 2 $([char]0x67e5)$([char]0x8be2)$([char]0x9e23)$([char]0x6f6e)"
                            ">>> $([char]0x8f93)$([char]0x5165) 3 $([char]0x9000)$([char]0x51fa)$([char]0x811a)$([char]0x672c)"
                            ">>> input 4 toggle English/Chinese"
    $input_use = Read-Host  ">>> $([char]0x9009)$([char]0x62e9)$([char]0x8981)$([char]0x67e5)$([char]0x8be2)$([char]0x7684)$([char]0x6e38)$([char]0x620f)/$([char]0x529f)$([char]0x80fd)"
    switch ($input_use) {
        1 { sr }
        2 { ww }
        3 {}
        4 {
            # TODO: 他喵的还没写呢
        }
        # default { Write-Host "`n>>> [派蒙] 叫你输入指定数字啊喂！！！" -ForegroundColor Yellow }
        default { Write-Host "`n>>> [$([char]0x6d3e)$([char]0x8499)] $([char]0x53eb)$([char]0x4f60)$([char]0x8f93)$([char]0x5165)$([char]0x6307)$([char]0x5b9a)$([char]0x6570)$([char]0x5b57)$([char]0x554a)$([char]0x5582)$([char]0xff01)$([char]0xff01)$([char]0xff01)" -ForegroundColor Yellow }
    }
} while ($input_use -ne 3)

# `u(....) => $([char]0x$1) 修改unicode :)
