use std::io::Write;
use std::thread;
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::net::TcpListener;

use derivative::Derivative;
use defer::defer;
use crate::plotmux::{
    color, Color, InitSeries2d, PlotReceiver, PlotSender, PlotableData, PlotableDeltaImage,
    PlotableInitImage, PlotableString, RgbDeltaImage, Series2d, Series2dVec,
};

use std::collections::HashMap;

use image::RgbImage;

pub enum ImageCompression {
    Lossless = 0b1111_1111,
    Lvl1 = 0b1111_1110,
    Lvl2 = 0b1111_1100,
    Lvl3 = 0b1111_1000,
}


#[derive(Derivative)]
#[derivative(Debug)]
struct ClientSender {
    name: (Color, String),
    pipe: (PlotSender, PlotReceiver),
    #[derivative(Debug="ignore")]
    _tcp_thread: Box<dyn Send>,
    first_send: bool,
    full_warn: bool,
}
impl ClientSender {
    pub fn make(idx: usize, name: String, c: Color, addr: String) -> (Self, u16) {
        let pipe = bounded(100);
        let (port, tcp_thread) = make_tcp_sender(idx, name.clone(), addr, pipe.1.clone());
        (
            Self {
                name: (c, name),
                pipe: pipe,
                _tcp_thread: Box::new(defer(|| tcp_thread.join().unwrap())),
                first_send: true,
                full_warn: false,
            },
            port
        )
    }
    fn send(&mut self, d: PlotableData) -> bool {
        if self.first_send {
            self.first_send = false;
            self.send(PlotableData::InitSource(self.name.1.clone()));
        }
        let mut ret = true;
        if self.pipe.0.is_full() {
            if !self.full_warn {
                self.full_warn = true;
                println!("\x1b[38;2;{};{};{}m[{}]\x1b[0m: \x1b[38;5;11m[plotmux]: channel is full, dropping data\x1b[0m",
                    self.name.0.0, self.name.0.1, self.name.0.2, self.name.1
                );
                ret = false;
            }
            match self.pipe.1.try_recv() {
                Ok(_) => (),
                Err(_) => (),
            }
        } else {
            self.full_warn = false;
        }
        match self.pipe.0.try_send(d) {
            Ok(_) => (),
            Err(e) => {
                println!(
                    "\x1b[38;2;{};{};{}m[{}]\x1b[0m: \x1b[1;31m[plotmux]: {}\x1b[0m",
                    self.name.0 .0, self.name.0 .1, self.name.0 .2, self.name.1, e
                );
                ret = false;
            }
        }
        ret
    }
}

#[derive(Debug)]
pub struct PlotSink {
    name: (Color, String),
    addr: String,
    series_plots_2d: HashMap<String, (usize, HashMap<String, usize>)>,
    image_plots: HashMap<String, (usize, Option<RgbImage>)>,
    idx: usize,
    senders: Vec<ClientSender>,
    new_sender: (Receiver<()>, Sender<u16>),
}
impl PlotSink {
    pub fn make(idx: usize, name: String, addr: String, color: Color, new_sender: (Receiver<()>, Sender<u16>)) -> Self {
        Self {
            name: (color, name),
            addr: addr,
            series_plots_2d: HashMap::new(),
            image_plots: HashMap::new(),
            idx: idx,
            senders: vec![],
            new_sender: new_sender,
        }
    }
    fn add_sender(&mut self) -> u16 {
        let (mut s, p) = ClientSender::make(self.idx, self.name.1.clone(), self.name.0, self.addr.clone());
        for (plot, (plot_idx, line)) in &self.series_plots_2d {
            s.send(PlotableData::InitSeriesPlot2d(plot.clone()));
            for (line_name, _line_idx) in line {
                s.send(InitSeries2d::make(*plot_idx, &line_name));
            }
        }
        for (plot, (_plot_idx, img)) in &self.image_plots {
            if let Some(image) = img {
                s.send(PlotableInitImage::make(plot.clone(), image.clone()));
            }
        }
        self.senders.push(s);
        p
    }
    fn check_new_sender(&mut self) {
        if let Ok(()) = self.new_sender.0.try_recv() {
            let sender = self.add_sender();
            let _ = self.new_sender.1.send(sender);
        }
    }
    fn send(&mut self, d: PlotableData) -> bool {
        let mut ret = true;
        for i in 0..self.senders.len() {
            ret &= self.senders[i].send(d.clone());
        }
        ret
    }
    pub fn println(&mut self, s: &str) {
        self.check_new_sender();
        self.println_c(None, s);
    }
    pub fn println2(&mut self, channel: &str, s: &str) {
        self.check_new_sender();
        self.println_c(Some(channel), s);
    }
    fn println_c(&mut self, channel: Option<&str>, s: &str) {
        if let Some(channel) = channel {
            let c = color(channel);
            println!(
                "\x1b[38;2;{};{};{}m[{}]\x1b[0m\x1b[38;2;{};{};{}m[{}]\x1b[0m: {}",
                self.name.0 .0,
                self.name.0 .1,
                self.name.0 .2,
                self.name.1,
                c.0,
                c.1,
                c.2,
                channel,
                s,
            );
        } else {
            println!(
                "\x1b[38;2;{};{};{}m[{}]\x1b[0m: {}",
                self.name.0 .0, self.name.0 .1, self.name.0 .2, self.name.1, s,
            );
        }
        self.send(PlotableString::make(channel, s));
    }
    fn init_series_2d(&mut self, plot_name: &str, series_name: &str) {
        if !self.series_plots_2d.contains_key(plot_name) {
            self.series_plots_2d.insert(
                plot_name.into(),
                (
                    self.series_plots_2d.len(),
                    HashMap::from([(series_name.into(), 0)]),
                ),
            );
            self.send(PlotableData::InitSeriesPlot2d(plot_name.to_string()));
            self.send(InitSeries2d::make(
                self.series_plots_2d[plot_name].0,
                series_name,
            ));
        }
        if !self.series_plots_2d[plot_name].1.contains_key(series_name) {
            let series_idx = self.series_plots_2d[plot_name].1.len();
            self.series_plots_2d
                .get_mut(plot_name)
                .unwrap()
                .1
                .insert(series_name.into(), series_idx);
            self.send(InitSeries2d::make(
                self.series_plots_2d[plot_name].0,
                series_name,
            ));
        }
    }
    pub fn plot_series_2d(&mut self, plot_name: &str, series_name: &str, x: f64, y: f64) {
        self.check_new_sender();
        self.init_series_2d(&plot_name, &series_name);
        let plot_idx = self.series_plots_2d[plot_name].0;
        let series_idx = self.series_plots_2d[plot_name].1[series_name];
        self.send(Series2d::make(plot_idx, series_idx, x, y));
    }
    pub fn plot_series_2d_vec(
        &mut self,
        plot_name: &str,
        series_name: &str,
        data: Vec<(f64, f64)>,
    ) {
        self.check_new_sender();
        self.init_series_2d(&plot_name, &series_name);
        let plot_idx = self.series_plots_2d[plot_name].0;
        let series_idx = self.series_plots_2d[plot_name].1[series_name];
        self.send(Series2dVec::make_series(plot_idx, series_idx, data));
    }
    pub fn plot_line_2d(&mut self, plot_name: &str, series_name: &str, data: Vec<(f64, f64)>) {
        self.check_new_sender();
        self.init_series_2d(&plot_name, &series_name);
        let plot_idx = self.series_plots_2d[plot_name].0;
        let series_idx = self.series_plots_2d[plot_name].1[series_name];
        self.send(Series2dVec::make_line(plot_idx, series_idx, data));
    }
    pub fn plot_image(&mut self, channel: &str, image: image::RgbImage, mask: ImageCompression) {
        self.check_new_sender();
        if !self.image_plots.contains_key(channel)
            || self.image_plots[channel].1.is_none()
            || self.image_plots[channel].1.as_ref().unwrap().dimensions() != image.dimensions()
        {
            if !self.image_plots.contains_key(channel) {
                self.image_plots.insert(
                    channel.into(),
                    (self.image_plots.len(), Some(image.clone())),
                );
            } else {
                self.image_plots.get_mut(channel).unwrap().1 = Some(image.clone());
            }
            self.send(PlotableInitImage::make(channel.to_string(), image));
        } else {
            let mask = mask as u8;
            let dimage = RgbDeltaImage::from_vec(
                image.width(),
                image.height(),
                std::iter::zip(
                    self.image_plots
                        .get_mut(channel)
                        .unwrap()
                        .1
                        .as_mut()
                        .unwrap()
                        .pixels_mut(),
                    image.pixels(),
                )
                .map(|(a, b)| {
                    let c = [
                        (b[0] & mask) as i16 - (a[0] & mask) as i16,
                        (b[1] & mask) as i16 - (a[1] & mask) as i16,
                        (b[2] & mask) as i16 - (a[2] & mask) as i16,
                    ];
                    *a = *b;
                    c
                })
                .flat_map(|a| a.into_iter())
                .collect::<Vec<_>>(),
            )
            .unwrap();
            if !self.send(PlotableDeltaImage::make(
                self.image_plots[channel].0,
                dimage,
            )) {
                self.image_plots.get_mut(channel).unwrap().1 = None;
            }
        }
    }
}

fn make_tcp_sender(idx: usize, name: String, addr: String, recv: PlotReceiver) -> (u16, JoinHandle<()>) {
    let thread_name = name+"-tcp-sender-thread";
    let exp = format!("failed to spawn thread: {}", thread_name);
    let mut encoder = snap::raw::Encoder::new();
    let listener = TcpListener::bind(addr+":0").unwrap();
    let port = listener.local_addr().unwrap().port();
    (
        port,
        thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if let Ok((mut stream, _)) = listener.accept() {
                    while let Ok(data) = recv.recv() {
                        let buf = bincode::serialize(&(idx, data)).unwrap();
                        let buf = encoder.compress_vec(&buf).unwrap();
                        let len = bincode::serialize(&buf.len()).unwrap();
                        if let Err(_) = stream.write(&len) {
                            continue;
                        }
                        if let Err(_) = stream.write(&buf) {
                            continue;
                        }
                    }
                }
            })
            .expect(&exp)
    )
}
