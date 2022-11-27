use clap::*;
use server::ThreadPool;
use std::io::{prelude::*, BufReader};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::{env, fs, thread, time::Duration};

#[macro_use]
extern crate log;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    ///address+socket, "--address 127.0.0.1:7878"
    #[arg(long)]
    address: String,

    ///count of threads, nonzero, 10 by default
    #[arg(short, long, default_value_t = 10)]
    threads_count: usize,

    ///count of threads, nonzero, auto by default
    #[arg(short, long, default_value_t = 0)]
    stack_size: usize,
}

fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    env::set_var("RUST_LOG", "trace");

    env_logger::init();

    let args = Args::parse();
    info!("args.address:{}", args.address);

    let address: SocketAddr = args
        .address
        .parse()
        .expect("Unable to parse socket address");

    let threads_count: usize = args.threads_count;

    let stack_size: usize = args.stack_size;

    info!(
        "Start server, address:{}, threads_count:{}, stack_size:{}",
        address, threads_count, stack_size
    );

    let listener = TcpListener::bind(address).expect("Unable to bind socket address");
    let pool = ThreadPool::new(threads_count, stack_size);

    loop {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => pool.execute(|| {
                    handle_connection(stream);
                }),
                Err(error) => {
                    error!("{:?}", error);
                    ()
                }
            };
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    /*let http_request: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();

        info!("Request: {:#?}", http_request);
    */
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = match fs::read_to_string(filename.clone()) {
        Ok(text) => text,
        Err(error) => {
            error!("{:?}, file:{}", error, filename);
            String::from("Unable to open file ") + &filename
        }
    };
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    if let Err(error) = stream.write_all(response.as_bytes()) {
        error!("{:?}", error);
    }
}
