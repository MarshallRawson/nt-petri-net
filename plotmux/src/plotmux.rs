use bincode;
use crossbeam_channel::{bounded, Receiver, Select, Sender};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::env;
use std::io::Write;
use std::net::{TcpListener, TcpStream}; //, IpAddr, Ipv4Addr, Shutdown};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use crate::plotsink::PlotSink;

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
    Image(PlotableImage),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlotableString {
    pub s: String,
}
impl From<&str> for PlotableData {
    fn from(item: &str) -> PlotableData {
        PlotableData::String(PlotableString { s: item.into() })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Plotable2d {
    pub series: String,
    pub x: f64,
    pub y: f64,
}
impl Plotable2d {
    pub fn make(series: String, x: f64, y: f64) -> PlotableData {
        PlotableData::Series2d(Plotable2d { series, x, y })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlotableImage {
    pub dim: (u32, u32),
    #[serde(with = "serde_bytes")]
    pub raw: Vec<u8>,
}
impl PlotableImage {
    pub fn make(image: image::RgbImage) -> Self {
        Self {
            dim: image.dimensions(),
            raw: image.into_raw(),
        }
    }
}

fn make_client(png_path: &String, receiver_names: &Vec<String>) -> TcpStream {
    let listener = TcpListener::bind("localhost:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    Command::new(
        env::current_exe()
            .expect("Getting current exe")
            .as_path()
            .parent()
            .unwrap()
            .join(Path::new("plotmuxui")),
    )
    .arg(format!("{}", png_path))
    .arg(format!("{}", port))
    .args(receiver_names)
    .spawn()
    .expect("starting plotmuxui");
    let (client, socket) = listener.accept().unwrap();
    assert_eq!("127.0.0.1".parse(), Ok(socket.ip()));
    client
}

pub struct PlotMux {
    receiver_names: Vec<String>,
    receivers: Vec<PlotReceiver>,
    client: Option<TcpStream>,
}
impl PlotMux {
    pub fn make() -> Self {
        PlotMux {
            receivers: vec![],
            receiver_names: vec![],
            client: None,
        }
    }
    pub fn add_plot_sink(&mut self, name: &str) -> PlotSink {
        let (sender, receiver) = bounded(100);
        let c = color(&name);
        self.receiver_names.push(name.into());
        self.receivers.push(receiver.clone());
        PlotSink::make(name.into(), c, (sender, receiver))
    }
    pub fn make_ready(mut self, png_path: &PathBuf) -> std::thread::JoinHandle<()> {
        self.client = Some(make_client(
            &png_path.as_os_str().to_str().unwrap().into(),
            &self.receiver_names,
        ));
        thread::spawn(move || self.spin())
    }
    fn spin(mut self) {
        assert!(self.client.is_some());
        |rs: &[PlotReceiver]| -> () {
            let mut sel = Select::new();
            for r in rs {
                sel.recv(&r);
            }
            loop {
                let oper = sel.select();
                let idx = oper.index();
                let data = oper.recv(&rs[idx]).unwrap();
                let buf = bincode::serialize(&(idx, data)).unwrap();
                self.client
                    .as_mut()
                    .unwrap()
                    .write(&bincode::serialize(&buf.len()).unwrap())
                    .unwrap();
                self.client.as_mut().unwrap().write(&buf).unwrap();
            }
        }(self.receivers.as_mut_slice());
    }
}
