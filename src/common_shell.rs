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



use std::io::{stdin, stdout, Write};
use std::process::{Command, exit};


// rust 有各种平台实现，可以跨平台编译使用
pub fn main(cmd: Vec<String>) {

    println!("Welcome to the Bemly shell!\n");

    // 快速执行命令
    if cmd.len() != 0 {
        // println!("{}", cmd.get(0).unwrap());
        //
        // match cmd.get(0).unwrap().to_lowercase().trim() {
        //     // 指定参数功能输出
        //     "-h" | "--help" => println!("toggle use libc shell: -u | --use-libc"),
        //     _ => todo!()
        // }

        let s = String::from_utf8(
            Command::new(cmd.get(0).unwrap())
                .args(cmd.iter().skip(1))
                .output()
                .expect("[TODO!] Failed to execute command. 命令执行失败")
                .stdout
        ).unwrap();
        println!("{s}");
    }

    // 进入循环执行模式
    loop {

        print!("> ");
        stdout().flush().expect("[TODO!] Failed to flush stdout. 输出流刷新失败");


        // 读取用户输入
        let mut command_buffer = String::new();
        stdin().read_line(&mut command_buffer).expect("[TODO!] Failed to read line. 读取输入失败");

        match command_buffer.trim() {
            "exit" => exit(0),
            "cd" => todo!(),
            s => {
                let mut program = s.split_whitespace();
                let s = String::from_utf8(
                    Command::new(program.next().unwrap())
                        .args(program)
                        .output()
                        .expect("[TODO!] Failed to execute command. 命令执行失败")
                        .stdout
                ).expect("[TODO!] Failed to convert output to string. 输出转换为字符串失败");
                println!("{s}");
            }
        };



    }
}

// 参考
// source: https://github.com/xitu/gold-miner/blob/master/TODO1/tutorial-write-a-shell-in-c.md