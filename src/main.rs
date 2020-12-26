#[macro_use(defer)]
extern crate scopeguard;
use libc::syscall;
use std::fs::File;
use std::io::BufReader;
use std::process::Command;
use steg::decoder::Decoder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input_path = args.get(1).expect("Expected argument 1 to be input path");
    let input_file = File::open(input_path).expect("file not found/missing");

    let mut reader = BufReader::new(input_file);
    let mut decoded_data: Vec<u8> = Vec::new();

    let decoder = Decoder::new();
    if let Err(err) = decoder.decode(&mut reader, &mut decoded_data) {
        eprintln!("Failed to decode: {}", err);
        std::process::exit(1);
    }

    match execute_binary(&decoded_data, &args[2..]) {
        Ok(exit) => {
            std::process::exit(exit.code().unwrap_or(0));
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}

fn execute_binary(data: &[u8], args: &[String]) -> Result<std::process::ExitStatus, String> {
    unsafe {
        let fd = memfd_create_exec_empty();
        if fd == -1 {
            return Err(String::from("memfd_create failed"));
        }

        let write_mode = 119; // w
        let file = libc::fdopen(fd, &write_mode); // TODO this can return NULL...

        defer! {
            if libc::fclose(file) == -1 {
                eprintln!("failed to fclose(file): -1");
            }
        }

        libc::fwrite(
            data.as_ptr() as *mut libc::c_void,
            8 as usize,
            data.len() as usize,
            file,
        );

        let output = Command::new(format!("/proc/self/fd/{}", fd))
            .args(args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn();

        if let Err(err) = output {
            return Err(err.to_string());
        }

        match output.unwrap().wait() {
            Ok(exit) => Ok(exit),
            Err(err) => Err(err.to_string()),
        }
    }
}

unsafe fn memfd_create_exec_empty() -> libc::c_int {
    let w = 119; //libc::c_char::from(119);
    let fd = syscall(libc::SYS_memfd_create, &w, 1);
    fd as libc::c_int
}
