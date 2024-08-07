use std::io;
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
    fn exec_err() {
        Command::new("bash")
            .arg("-c")
            .exec();
        println!("如果出错就看不到我惹")
    }
    // 上面这个不是panic，直接替换进程然后在libc处强行结束了gcc
}