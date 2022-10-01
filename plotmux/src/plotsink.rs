use crate::plotmux::{Color, PlotReceiver, PlotSender, Plotable2d, PlotableData, PlotableImage};

#[derive(Debug)]
pub struct PlotSink {
    name: String,
    color: Color,
    pipe: (PlotSender, PlotReceiver),
    full_warn: bool,
}
impl PlotSink {
    pub fn make(name: String, color: Color, pipe: (PlotSender, PlotReceiver)) -> Self {
        Self {
            name: name,
            color: color,
            pipe: pipe,
            full_warn: false
        }
    }
    fn send(&mut self, d: PlotableData) {
        if self.pipe.0.is_full() {
            if !self.full_warn {
                self.full_warn = true;
                println!("\x1b[38;2;{};{};{}m[{}]\x1b[0m: \x1b[38;5;11m[plotmux]: channel is full, dropping data\x1b[0m",
                    self.color.0, self.color.1, self.color.2, self.name
                );
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
            Err(e) => println!(
                "\x1b[38;2;{};{};{}m[{}]\x1b[0m: \x1b[1;31m[plotmux]: {}\x1b[0m",
                self.color.0, self.color.1, self.color.2, self.name, e
            ),
        }
    }
    pub fn println(&mut self, s: &str) {
        println!(
            "\x1b[38;2;{};{};{}m[{}]\x1b[0m: {}",
            self.color.0, self.color.1, self.color.2, self.name, s
        );
        self.send(s.into());
    }
    pub fn plot_series_2d(&mut self, series: String, x: f64, y: f64) {
        self.send(Plotable2d::make(series, x, y));
    }
    pub fn plot_image(&mut self, image: image::RgbaImage) {
        self.send(PlotableData::Image(PlotableImage::make(image)));
    }
}
