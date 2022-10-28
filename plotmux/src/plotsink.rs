use crate::plotmux::{color, Color, PlotReceiver, PlotSender, Plotable2d, PlotableData, PlotableInitImage, PlotableDeltaImage, PlotableString, InitSeries2d, RgbDeltaImage};

use std::collections::HashMap;

use image::RgbImage;

pub enum ImageCompression {
    Lossless = 0b1111_1111,
    Lvl1 = 0b1111_1110,
    Lvl2 = 0b1111_1100,
    Lvl3 = 0b1111_1000,
}


#[derive(Debug)]
pub struct PlotSink {
    name: (Color, String),
    pipe: (PlotSender, PlotReceiver),
    first_send: bool,
    full_warn: bool,
    series_plots_2d: HashMap<String, (usize, HashMap<String, usize>)>,
    image_plots: HashMap<String, (usize, Option<RgbImage>)>,
}
impl PlotSink {
    pub fn make(name: String, color: Color, pipe: (PlotSender, PlotReceiver)) -> Self {
        Self {
            name: (color, name),
            pipe: pipe,
            first_send: true,
            full_warn: false,
            series_plots_2d: HashMap::new(),
            image_plots: HashMap::new(),
        }
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
                    self.name.0.0, self.name.0.1, self.name.0.2, self.name.1, e
                );
                ret = false;
            }
        }
        ret
    }
    pub fn println(&mut self, s: &str) {
        self.println_c(None, s);
    }
    pub fn println2(&mut self, channel: &str, s: &str) {
        self.println_c(Some(channel), s);
    }
    fn println_c(&mut self, channel: Option<&str>, s: &str) {
        if let Some(channel) = channel {
            let c = color(channel);
            println!(
                "\x1b[38;2;{};{};{}m[{}]\x1b[0m\x1b[38;2;{};{};{}m[{}]\x1b[0m: {}",
                self.name.0.0, self.name.0.1, self.name.0.2, self.name.1,
                c.0, c.1, c.2, channel, s,
            );
        } else {
            println!(
                "\x1b[38;2;{};{};{}m[{}]\x1b[0m: {}",
                self.name.0.0, self.name.0.1, self.name.0.2, self.name.1, s,
            );
        }
        self.send(PlotableString::make(channel, s));
    }
    pub fn plot_series_2d(&mut self, plot_name: &str, series_name: &str, x: f64, y: f64) {
        if !self.series_plots_2d.contains_key(plot_name) {
            self.series_plots_2d.insert(plot_name.into(),
                (self.series_plots_2d.len(), HashMap::from([(series_name.into(), 0)]))
            );
            self.send(PlotableData::InitSeriesPlot2d(plot_name.to_string()));
            self.send(InitSeries2d::make(self.series_plots_2d[plot_name].0, series_name));
        }
        if !self.series_plots_2d[plot_name].1.contains_key(series_name) {
            let series_idx = self.series_plots_2d[plot_name].1.len();
            self.series_plots_2d.get_mut(plot_name).unwrap().1.insert(series_name.into(), series_idx);
            self.send(InitSeries2d::make(self.series_plots_2d[plot_name].0, series_name));
        }
        let plot_idx = self.series_plots_2d[plot_name].0;
        let series_idx = self.series_plots_2d[plot_name].1[series_name];
        self.send(Plotable2d::make(plot_idx, series_idx, x, y));
    }
    pub fn plot_image(&mut self, channel: &str, image: image::RgbImage, mask: ImageCompression) {
        if !self.image_plots.contains_key(channel) || self.image_plots[channel].1.is_none() || self.image_plots[channel].1.as_ref().unwrap().dimensions() != image.dimensions() {
            if !self.image_plots.contains_key(channel) {
                self.image_plots.insert(channel.into(), (self.image_plots.len(), Some(image.clone())));
            } else {
                self.image_plots.get_mut(channel).unwrap().1 = Some(image.clone());
            }
            self.send(PlotableInitImage::make(channel.to_string(), image));
        } else {
            let mask = mask as u8;
            let dimage = RgbDeltaImage::from_vec(image.width(), image.height(),
                std::iter::zip(self.image_plots.get_mut(channel).unwrap().1.as_mut().unwrap().pixels_mut(), image.pixels()).map(|(a, b)| {
                    let c = [
                        (b[0] & mask) as i16 - (a[0] & mask) as i16,
                        (b[1] & mask) as i16 - (a[1] & mask) as i16,
                        (b[2] & mask) as i16 - (a[2] & mask) as i16,
                    ];
                    *a = *b;
                    c
                }).flat_map(|a| a.into_iter()).collect::<Vec<_>>()
            ).unwrap();
            if !self.send(PlotableDeltaImage::make(self.image_plots[channel].0, dimage)) {
                self.image_plots.get_mut(channel).unwrap().1 = None;
            }
        }
    }
}
