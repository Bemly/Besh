// 下面说明只适用于x86_64-unknown-linux-gnu及其它Unix类系统的抽象层实现(Rust Std Version: ad96323 1.0.0)
// # output
// pub fn output(&mut self) -> io::Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
//         let (proc, pipes) = self.spawn(Stdio::MakePipe, false)?;
//         crate::sys_common::process::wait_with_output(proc, pipes)
//     }
//  output也是要创建新线程的，但是会等待新线程返回数据来阻塞主线程(重载到spawn上面)
// source: https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/src/std/sys/unix/process/process_unix.rs.html#170-173
//
// # spawn
// spawn和exec实现均要用到unsafe的do_exec
// pub fn spawn(
//      &mut self,
//      default: Stdio,
//      needs_stdin: bool
//      ) -> Result<(Process, StdioPipes)>
// let Err(err) = unsafe { self.do_exec(theirs, envp.as_ref()) };
// source: https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/src/std/sys/unix/process/process_unix.rs.html#113
//
// # exec
// 不会开新线程，需要自己手动创建，否则报错会强制主进程恐慌
// pub fn exec(&mut self, default: Stdio) -> io::Error
// let Err(e) = self.do_exec(theirs, envp.as_ref());
// source: https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/src/std/sys/unix/process/process_unix.rs.html#473
//
// # do_exec
// 需要用到POSIX标准的C库函数execvp
// C 函数原型: int execvp(const char *file, char *const argv[]);
// execvp 这个变体接受一个程序名和一个字符串参数的数组（也叫做向量（vector），因此是‘v’）（数组的第一个元素应当是程序名）
// unsafe fn do_exec(
//      &mut self,
//      stdio: ChildPipes,
//      maybe_envp: Option<&CStringArray>
//      ) -> Result<!, Error>
// libc::execvp(self.get_program_cstr().as_ptr(), self.get_argv().as_ptr());
// source: https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/src/std/sys/unix/process/process_unix.rs.html#473

use std::io::{self, stdin, stdout, Write};
use std::process::{Command, exit, Output};
use super::error::*;

// rust 有各种平台实现，可以跨平台编译使用
pub fn main(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {

    println!("Welcome to the Bemly shell!\n");

    // 快速执行命令
    if args.len() != 0 {

        // match cmd.get(0).unwrap().to_lowercase().trim() {
        //     // 指定参数功能输出
        //     "-h" | "--help" => println!("toggle use libc shell: -u | --use-libc"),
        //     _ => todo!()
        // }

        // exec_cmd：命令执行体，命令名
        // 命令执行体：命令名，参数数组
        let cmd = args.get(0).unwrap().as_str();
        exec_cmd(Command::new(cmd).args(args.iter().skip(1)).output(), cmd)?;
    }

    // 获取用户名和主机名
    let username = std::env::var("USER")
        .unwrap_or(String::from_utf8(Command::new("/usr/bin/whoami").output()?.stdout)?);
    let hostname = std::fs::read_to_string("/etc/hostname")
        .unwrap_or(String::from("unknown")).replace("\n", "");
    
    // 获取当前工作目录（先转&str再转String加上所有权）
    let mut pwd = std::env::current_dir()?.to_str().expect(NOT_FIND_CRR_DIR)
        .replace(std::env::var("HOME").unwrap_or(String::from("~")).as_str(), "~");

    // 进入循环执行模式
    loop {
        print!("{username}@{hostname} {pwd}> ");
        stdout().flush()?;
        // 读取用户输入
        let mut command_buffer = String::new();
        stdin().read_line(&mut command_buffer)?;

        // 捕获指定命令
        match command_buffer.trim() {
            // 退出程序 没有正则的痛苦 哇的一声就哭出来了昂
            "exit" | "exit()" | "quit" | "qui" | "qu" | "q" | ":q" => exit(0),
            // 就是空行
            "" => println!(),
            // 执行路径下软件或者脚本
            s => {
                // TODO: 屎山代码 分割空格为 实现了迭代器方法的spw对象
                let mut program = s.split_whitespace();
                let cmd = program.next().expect(NOT_GET_PROGRAM_NAME);
                match cmd { 
                    "cd" => {
                        // 获取当前工作目录
                        let pwd = std::env::current_dir()?;
                        // 获取用户输入的路径
                        let path = program.next().expect(NOT_GET_PATH);
                        // 获取用户输入的路径
                        let path = std::path::Path::new(path);
                        // 获取用户输入的路径的绝对路径
                        let path = path.canonicalize()?;
                        // 获取用户输入的路径的绝对路径的字符串
                        let path = path.to_str().expect(NOT_GET_PATH);
                    },
                    _ => exec_cmd(Command::new(cmd).args(program).output(), cmd)?
                }
            }
        }
    }
}

// 执行命令
fn exec_cmd(result: Result<Output, io::Error>, cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
    match result {
        // 匹配Result()结果，成功则从stdout流打印输出结果，失败打印stderr流
        Ok(output) => { println!("{}", String::from_utf8(output.stdout)?) },
        Err(e) => { eprintln!("besh: {cmd}: {e}") }
    }
    Ok(())
}

// 参考
// source: https://github.com/xitu/gold-miner/blob/master/TODO1/tutorial-write-a-shell-in-c.md