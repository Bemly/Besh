use std::io;
use std::io::{stdout, Write};
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;

# [test]
fn test_spawn() -> io::Result<()> {
    let mut output = Command::new("bash")
        .arg("-c")
        .stdout(Stdio::piped())
        .spawn()?;

    output.wait()?;

    let o = output.wait_with_output()?;

    println!(r"{:#?} \n {:#?}", String::from_utf8(o.stdout).unwrap(), o.status.success());
    Ok(())
}

# [test]
fn test_status() -> io::Result<()> {
    let status = Command::new("ls")
        .status()?; // 没找到程序
    if status.success() {
        println!("子进程成功执行");
    } else {
        // 程序执行失败
        eprintln!("子进程执行失败：{}", status);
    }
    Ok(())
}

# [test]
fn test_exec() {

    // exec 执行失败直接panic
    Command::new("ll")
        .arg(".")
        .exec();

    # [ignore]
    # [should_panic]
    fn _exec_err() {
        Command::new("bash")
            .arg("-c")
            .exec();
        println!("如果出错就看不到我惹")
    }
    // 上面这个不是panic，直接替换进程然后在libc处强行结束了gcc
}

// 下面这些需要在命令行中执行，幽默老JB厂不能输入流进去
# [test]
fn test_lines() {
    use std::io::BufRead;
    
    // Ctrl+D 结束 EOF
    let stdin = io::stdin();
    let iterator = stdin.lock().lines();

    for line in iterator {
        let line = line.expect("Failed to read line");
        println!("Line: {}", line);
    }

    println!("Input stream ended.");
}

// 测试是按尾缀通配符匹配的 .*?$
// test_line就是test_line*来执行多个测试项目了
// 测试项目不能println
# [test]
fn test_line2() {
    use std::io::{self, BufRead};

    // 按行读取
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();

    while let Some(Ok(line)) = iterator.next() {
        println!("Line: {}", line);
        stdout().flush().unwrap();

        // 某个条件满足时退出循环
        if line == "exit" {
            break;
        }
    }

    println!("Exited the loop.");
}