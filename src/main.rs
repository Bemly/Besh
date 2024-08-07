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

use std::alloc::handle_alloc_error;
use std::io::{self, stdin, stdout, Write};
use std::process::{Command, exit};
use log::error;

// # [derive(Debug)]
// struct Command(i32, Vec<String>);
// impl Command {
//     fn new(status: i32, cmd: String) -> Command { Command(status, Vec::new()) }
//     fn get_status(&self) -> i32 { self.0 }
//     fn get_cmd(&self) -> &Vec<String> { &self.1 }
// }






// rust 有各种平台实现，可以跨平台编译使用
fn main() {
    init();
    loop {
        // 函数式写法不可取 :(
        draw_cursor();
        draw_result(exec_command(get_usr_input()));

        // 退出




        fn get_usr_input() -> String {
            // 读取用户输入
            let mut command_buffer = String::new();
            stdin().read_line(&mut command_buffer).expect("Failed to read line");

            match command_buffer.trim() {
                "exit" => exit(0),
                "cd" => todo!(),
                s => s.to_string()
            }
        }

        fn exec_command(S: String) -> String {
            let mut program = S.split_whitespace();
            String::from_utf8(
                Command::new(program.next().unwrap())
                    .args(program)
                    .output()
                    .expect("Failed to execute command")
                    .stdout
            ).expect("Failed to convert output to string")
        }

        fn draw_result(S: String) {
            println!("{S}")
        }
    }
}
fn init() {
    println!("Welcome to the Bemly shell!");
}
fn draw_cursor() -> io::Result<()> {
    print!("> ");
    Ok(stdout().flush()?)
}

// 参考
// source: https://github.com/xitu/gold-miner/blob/master/TODO1/tutorial-write-a-shell-in-c.md