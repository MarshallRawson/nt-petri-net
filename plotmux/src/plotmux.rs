use sha1::{Sha1, Digest};
use crossbeam_channel::{Sender, Receiver, bounded, Select};
use serde::{Serialize, Deserialize};
use bincode;
use std::process::{Command, Child};
use std::env;
use std::path::Path;
use std::net::{TcpListener, TcpStream};//, IpAddr, Ipv4Addr, Shutdown};
use std::io::Write;

use crate::plotsink::{PlotSink};

pub type Color = (u8, u8, u8);
pub fn color(s: &str) -> Color {
    let normalize = |x: f32| (((x / 255.0) * 155.0) + 100.0) as u8;
    let mut hasher = Sha1::new();
    hasher.update(s);
    let digest = hasher.finalize();
    (
        normalize(digest[0].into()),
        normalize(digest[2].into()),
        normalize(digest[4].into()),
    )
}

pub type PlotReceiver = Receiver<PlotableData>;
pub type PlotSender = Sender<PlotableData>;

#[derive(Serialize, Deserialize, Clone)]
pub enum PlotableData {
    String(PlotableString),
    Series2d(Plotable2d),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlotableString { pub s: String }
impl From<&str> for PlotableData {
    fn from(item: &str) -> PlotableData {
        PlotableData::String(PlotableString{ s: item.into() })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Plotable2d { pub series: String, pub x: f64, pub y: f64 }
impl Plotable2d {
    pub fn make(series: String, x: f64, y: f64) -> PlotableData {
        PlotableData::Series2d(Plotable2d{series, x, y})
    }
}

struct PlotMuxServer {
    listener: TcpListener,
    port: u16,
    ui_subprocess: Child,
    client: TcpStream,
}
impl PlotMuxServer {
    fn make(receiver_names: &Vec<String>) -> Self {
        let listener = TcpListener::bind("localhost:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let ui_subprocess = Command::new(env::current_exe().expect("Getting current exe").as_path().parent().unwrap().join(Path::new("plotmuxui")))
                .arg(format!("{}", port))
                .args(receiver_names)
                .spawn()
                .expect("starting plotmuxui")
        ;
        let client = listener.accept().unwrap().0;
        Self {listener, port, ui_subprocess, client}
    }
}
pub struct PlotMux {
    receiver_names: Vec<String>,
    receivers: Vec<PlotReceiver>,
    server: Option<PlotMuxServer>,
}
impl PlotMux {
    pub fn make() -> Self {
        PlotMux { receivers: vec![], receiver_names: vec![], server: None }
    }
    pub fn add_plot_sink(&mut self, name: String) -> PlotSink {
        let (sender, receiver) = bounded(100);
        let c = color(&name);
        self.receiver_names.push(name.clone());
        self.receivers.push(receiver.clone());
        PlotSink::make(name, c, (sender, receiver))
    }
    pub fn make_ready(&mut self) {
        self.server = Some(PlotMuxServer::make(&self.receiver_names));
    }
    pub fn spin(mut self) {
        assert!(self.server.is_some());
        // spin up plotmuxui in subprocess with correct port and source names
        |rs: &[PlotReceiver]| -> () {
            let mut sel =  Select::new();
            for r in rs {
                sel.recv(&r);
            }
            loop {
                let oper = sel.select();
                let idx = oper.index();
                let data = oper.recv(&rs[idx]).unwrap();
                let buf = bincode::serialize(&(idx, data)).unwrap();
                self.server.as_mut().unwrap().client.write(&bincode::serialize(&buf.len()).unwrap()).unwrap();
                self.server.as_mut().unwrap().client.write(&buf).unwrap();
            }
        } (self.receivers.as_mut_slice());
    }
}
