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
use std::process::{Command, exit, Output};
use std::{env, io, fs};
use std::path::PathBuf;

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

    // 获取用户名和主机名 第一层环境变量 第二层使用软件 大气层恐慌
    let username = env::var("USER")
        .unwrap_or(String::from_utf8(Command::new("/usr/bin/whoami").output()?.stdout)?);
    let hostname = fs::read_to_string("/etc/hostname")
        .unwrap_or(String::from("unknown")).replace("\n", "");
    
    // 获取当前工作目录 第一层环境变量 第二层使用软件 大气层恐慌
    let mut pwd = env::current_dir()
        .unwrap_or(
            fs::canonicalize(
                String::from_utf8(Command::new("/usr/bin/pwd").output()?.stdout)?.replace("\n", "")
            )?
        );

    let homedir = Homedir::init(env::var("HOME").unwrap_or(String::new()));

    // 进入循环执行模式
    loop {
        // 显示：获取用户目录
        print!("{username}@{hostname} {}> ", homedir.to_relative_home(&pwd));
        stdout().flush()?;
        // 读取用户输入
        let mut command_buffer = String::new();
        stdin().read_line(&mut command_buffer)?;

        // 捕获指定命令
        // TODO: 屎山代码 分割空格为 实现了迭代器方法的spw对象
        // cmd从program弹出，剩下的是参数
        let mut program = command_buffer.trim().split_whitespace();
        let cmd = program.next().unwrap_or("");
        match cmd {
            // 退出程序 最穷举的一集
            "exit" | "exit()" | "quit" | "qui" | "qu" | "q" | ":q" => exit(0),
            // 就是空行 上方None替换
            "" => println!(),
            "cd" => {
                // 没有参数
                let args = program.next().unwrap_or("");
                // 多个参数
                if args.eq("") { println!() }
                else if program.count().eq(&0usize) {
                    homedir.to_absoulte_home(args, &mut pwd);
                    println!("{pwd:?}");
                    // unimplemented!  替换~的Bug 暂时不想修了 做libc调用的时候再考虑
                    pwd = match fs::canonicalize(&pwd) {
                        Ok(pwd) => {
                            println!("{cmd}: success change dir to : {args} 成功切换目录到: {args}");
                            pwd
                        },
                        Err(e) => {
                            println!("besh: {cmd}: {e}");
                            PathBuf::from(homedir.path.clone())
                        }
                    }
                    
                } else {
                    println!("besh: {cmd}: too many arguments. 只允许一个参数")
                }
            },
            // 执行路径下软件或者脚本
            _ => exec_cmd(Command::new(cmd).args(program).current_dir(&pwd).output(), cmd)?
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

struct Homedir<'a> {
    path: String,
    name: &'a str
}
impl<'a> Homedir<'a> {
    fn init(path: String) -> Self {
        Homedir { path, name: "~" }
    }
    fn to_absoulte_home(&self, args: &str, pwd: &mut PathBuf) {
        // 替换掉~通配符之后再push拼接
        if args.find(self.name).unwrap_or(0).eq(&0) {
            pwd.push(args.replacen(self.name, &self.path, 1))
        }
    }

    fn to_relative_home(&self, pwd: &PathBuf) -> String {
        pwd.to_str().unwrap_or("").replacen(&self.path, self.name, 1)
    }
}


// 参考
// source: https://github.com/xitu/gold-miner/blob/master/TODO1/tutorial-write-a-shell-in-c.md