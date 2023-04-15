use std::{thread, net::{SocketAddr, IpAddr, Ipv4Addr, TcpStream}, io::{Write, Read}, time::Duration, sync::{Arc, atomic::AtomicBool}};

use crate::{cpu::addressable::Addressable, BlockingQueue};


pub struct IO {
    console_queue: Arc<BlockingQueue<i32>>,
    telnet_input: Arc<BlockingQueue<i32>>,
    telnet_output: Arc<BlockingQueue<i32>>,
}

impl IO {

}

impl Addressable for IO {
    fn load_file(&mut self, file: &std::path::Path) -> i32 {
        0
    }
    fn write(&mut self, loc: i32, val: i32) {
        match loc & 255 {
            0x00 => self.console_queue.en_q(val),
            0xfe => { self.telnet_output.en_q(val&255); println!("[rinting"); },
            0xff => {
                println!("printingpacked {}", (val >> 8) as u8 as char);
                self.telnet_output.en_q(val>>8);
                if val & 255 != 0 {
                    self.telnet_output.en_q(val);
                }
            },

            _ => (),
        }
    }
    fn read(&self, loc: i32) -> i32 {
        match loc {
            0xfe => {  println!("readunpacked"); self.telnet_input.de_q() },
            0xff => {
                println!("readunpacked") ;
                (self.telnet_input.de_q() << 8) | self.telnet_input.de_q()
            }
            _ => 0,
        }
    }
}

impl IO {
    pub fn init() -> IO {
        let io = IO { 
            console_queue: Arc::new(BlockingQueue::new()), 
            telnet_input: Arc::new(BlockingQueue::new()), 
            telnet_output: Arc::new(BlockingQueue::new()), 
        };

        let console_io = io.console_queue.clone();
        thread::spawn(move || {
            loop {
                println!("{}", console_io.de_q());
            }
        });

        let telnet_inbound = io.telnet_input.clone();
        let telnet_outbound = io.telnet_output.clone();
        thread::spawn(move || {
            let mut serv = TelnetIO::new(telnet_inbound, telnet_outbound);
            serv.telnet_server_main();
        });

        io
    }
}


pub struct TelnetIO {
    inbound: Arc<BlockingQueue<i32>>, // user input from telnet -> cpu
    outbound: Arc<BlockingQueue<i32>>, // cpu -> telnet console
    client_sock: TcpStream,
}

impl TelnetIO {
    pub fn new(inbound: Arc<BlockingQueue<i32>>, outbound: Arc<BlockingQueue<i32>>) -> TelnetIO {
        let serversocket =  std::net::TcpListener::bind("127.0.0.1:23").unwrap();
        TelnetIO { 
            inbound, 
            outbound, 
            client_sock: serversocket.accept().unwrap().0, 
        }
    }

    pub fn telnet_server_main(&mut self) {
        // output_tcp_stream.write(new byte[] {-1, -3, 3, -1, -2, 1});
        self.client_sock.write(&[0-1, 0-3, 3, 0-1, 0-2, 1]).unwrap();
        self.client_sock.flush().unwrap();

        let mut data = Vec::new();
        self.client_sock.read_to_end(&mut data).unwrap();

        while data.len() == 0 { thread::sleep(Duration::from_millis(50)); self.client_sock.read_to_end(&mut data).unwrap(); }

        loop {
            if self.outbound.len() > 0 {
                let val = self.outbound.de_q();
                self.client_sock.write(&[(val & 255) as u8]).unwrap();
                self.client_sock.flush().unwrap();
            }
            let mut data = Vec::new();
            self.client_sock.read_to_end(&mut data).unwrap();
            for i in data {
                self.inbound.en_q(i as i32);
            }
        }
    }      
}