use std::env;

mod common_shell;
mod better_truth_tty;

use common_shell::main as common_shell;
use better_truth_tty::main as unix_shell;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    
    // 没有参数的情况
    match args.len() {
        0 => common_shell(args),
        _ => match args.get(0).unwrap().to_lowercase().trim() {
            // 指定参数功能输出
            "-h" | "--help" => println!("toggle use libc shell: -u | --use-libc"),
            "-v" | "--version" => println!("BESH version 0.1 \n\t By Bemly_. 2024.08.08"),
            "-u" | "--use-libc" => unix_shell(args),
            _ => common_shell(args)
        },
    }

    
    
    

    // println!("args: {:?}", args.get(0));
    
}