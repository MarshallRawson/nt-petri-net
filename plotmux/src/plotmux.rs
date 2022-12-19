use bincode;
use crossbeam_channel::{bounded, Receiver, Select, Sender};
use defer::defer;
use image::{ImageBuffer, Rgb, RgbImage};
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
    InitSource(String),
    String(PlotableString),
    InitSeriesPlot2d(String),
    InitSeries2d(InitSeries2d),
    Series2d(Series2d),
    Series2dVec(Series2dVec),
    Line2d(Series2dVec),
    InitImage(PlotableInitImage),
    DeltaImage(PlotableDeltaImage),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlotableString {
    pub channel: Option<String>,
    pub s: String,
}
impl PlotableString {
    pub fn make(channel: Option<&str>, s: &str) -> PlotableData {
        let channel = match channel {
            Some(c) => Some(c.to_string()),
            None => None,
        };
        PlotableData::String(PlotableString {
            channel: channel,
            s: s.into(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InitSeries2d {
    pub channel: usize,
    pub series: String,
}
impl InitSeries2d {
    pub fn make(channel: usize, series: &str) -> PlotableData {
        PlotableData::InitSeries2d(Self {
            channel: channel,
            series: series.into(),
        })
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Series2d {
    pub channel: usize,
    pub series: usize,
    pub x: f64,
    pub y: f64,
}
impl Series2d {
    pub fn make(channel: usize, series: usize, x: f64, y: f64) -> PlotableData {
        PlotableData::Series2d(Series2d {
            channel: channel,
            series: series,
            x: x,
            y: y,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Series2dVec {
    pub channel: usize,
    pub series: usize,
    pub data: Vec<(f64, f64)>,
}
impl Series2dVec {
    fn make(channel: usize, series: usize, data: Vec<(f64, f64)>) -> Self {
        Series2dVec {
            channel: channel,
            series: series,
            data: data,
        }
    }
    pub fn make_series(channel: usize, series: usize, data: Vec<(f64, f64)>) -> PlotableData {
        PlotableData::Series2dVec(Self::make(channel, series, data))
    }
    pub fn make_line(channel: usize, series: usize, data: Vec<(f64, f64)>) -> PlotableData {
        PlotableData::Line2d(Self::make(channel, series, data))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlotableInitImage {
    pub channel: String,
    pub dim: (u32, u32),
    #[serde(with = "serde_bytes")]
    pub raw: Vec<u8>,
}
impl PlotableInitImage {
    pub fn make(channel: String, image: RgbImage) -> PlotableData {
        PlotableData::InitImage(Self {
            channel: channel,
            dim: image.dimensions(),
            raw: image.into_raw(),
        })
    }
}

pub type RgbDeltaImage = ImageBuffer<Rgb<i16>, Vec<i16>>;
#[derive(Serialize, Deserialize, Clone)]
pub struct PlotableDeltaImage {
    pub channel: usize,
    pub raw: Vec<i16>,
}
impl PlotableDeltaImage {
    pub fn make(channel: usize, image: RgbDeltaImage) -> PlotableData {
        PlotableData::DeltaImage(Self {
            channel: channel,
            raw: image.into_raw(),
        })
    }
}

pub enum ClientMode {
    Local(),
    Remote(String),
}

fn make_client(png_path: Option<&PathBuf>, mode: ClientMode) -> TcpStream {
    let listener = match &mode {
        ClientMode::Local() => TcpListener::bind("localhost:0").unwrap(),
        ClientMode::Remote(addr) => TcpListener::bind(addr).unwrap(),
    };
    match mode {
        ClientMode::Local() => {
            let port = listener.local_addr().unwrap().port();
            let mut cmd = Command::new(
                env::current_exe()
                    .expect("Getting current exe")
                    .as_path()
                    .parent()
                    .unwrap()
                    .join(Path::new("plotmuxui")),
            );
            if let Some(png_path) = png_path {
                let png_path = &png_path.as_os_str().to_str().unwrap().to_string();
                cmd.arg("--graph-png").arg(format!("{}", png_path));
            }
            cmd.arg("--addr")
                .arg(format!("localhost:{}", port))
                .spawn()
                .expect("starting plotmuxui");
        }
        ClientMode::Remote(addr) => {
            println!("cargo run --bin plotmuxui -- --addr {}", addr);
        }
    }
    let (client, _socket) = listener.accept().unwrap();
    client
}

pub struct PlotMux {
    receivers: Vec<PlotReceiver>,
    client: Option<TcpStream>,
}
impl PlotMux {
    pub fn make() -> Self {
        PlotMux {
            receivers: vec![],
            client: None,
        }
    }
    pub fn add_plot_sink(&mut self, name: &str) -> PlotSink {
        let (sender, receiver) = bounded(100);
        let c = color(&name);
        self.receivers.push(receiver.clone());
        PlotSink::make(name.into(), c, (sender, receiver))
    }
    pub fn make_ready(
        mut self,
        png_path: Option<&PathBuf>,
        client: ClientMode,
    ) -> impl Drop {
        self.client = Some(make_client(png_path, client));
        let join_handle = thread::Builder::new()
            .name("plotmux-server".into())
            .spawn(move || self.spin())
            .expect("unable to spawn plotmux-server thread");
        defer(|| join_handle.join().unwrap())
    }
    fn spin(mut self) {
        let mut plot_idx: Vec<_> = (0..self.receivers.len()).collect();
        while self.receivers.len() > 0 {
            let remove_idx = |rs: &[PlotReceiver]| -> usize {
                let mut sel = Select::new();
                for r in rs {
                    sel.recv(&r);
                }
                let mut encoder = snap::raw::Encoder::new();
                loop {
                    let oper = sel.select();
                    let idx = oper.index();
                    match oper.recv(&rs[idx]) {
                        Ok(data) => {
                            let buf = bincode::serialize(&(plot_idx[idx], data)).unwrap();
                            let buf = encoder.compress_vec(&buf).unwrap();
                            if let Err(_) = self.client
                                .as_mut()
                                .unwrap()
                                .write(&bincode::serialize(&buf.len()).unwrap()) {
                                    continue;
                            }
                            if let Err(_) = self.client.as_mut().unwrap().write(&buf) {
                                continue;
                            }
                        }, Err(_) => {
                            return idx;
                        },
                    }
                }
            }(self.receivers.as_mut_slice());
            self.receivers.remove(remove_idx);
            plot_idx.remove(remove_idx);
        }
    }
}
