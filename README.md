# BESH

 Bemly_ Shell is a simple shell, written in Rust.

 inspired by lsh: https://github.com/brenns10/lsh

## Usage

 besh -h | --help     帮助
 
      -v | --version  版本
      
      -u | --use-libc 使用unsafe的libc库进行更底层的shell
      

 besh <*.besh [args]>        执行besh语言脚本
 
 besh <command [args]>       直接执行传入命令
 
 besh                        默认进入besh环境

## Build

 git clone https://github.com/Bemly/Besh.git

 cargo build --release

 cd ./target/release

 ./besh

## Install

 v0.0.1 Nightly Tester None
