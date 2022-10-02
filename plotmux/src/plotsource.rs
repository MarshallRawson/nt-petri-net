use crate::plotmux::{color, Color, PlotableData};
use eframe::egui;
use eframe::egui::widgets::plot::PlotPoint;
use egui_extras::image::RetainedImage;
use std::mem::size_of;

use image::buffer::ConvertBuffer;
use image::Rgba;
use image::RgbaImage;
use std::collections::{HashMap, VecDeque};
use lazy_static::lazy_static;

lazy_static! {
    static ref DEFAULT_IMAGE : RgbaImage = RgbaImage::from_fn(1920, 1080, |y, x| {
        let x = x as f64 * (6. / 1079.) - 3.;
        let y = y as i32 - (1920 - 1080) / 2;
        let y = y as f64 * (6. / 1079.) - 3.;
        let left = x.tan().cos().sin();
        let right = y.tan().cos().sin();
        if (left - right).abs() < 0.01 {
            Rgba::<u8>::from([0xE9_u8, 0x45, 0x60, 0xff])
        } else if left > right {
            Rgba::<u8>::from([0x53_u8, 0x34, 0x83, 0xff])
        } else {
            Rgba::<u8>::from([0x16_u8, 0x21, 0x3E, 0xff])
        }
    });
}

pub struct PlotSource {
    pub name: String,
    max_memory_footprint: usize,
    memory_footprint: usize,
    pub color: Color,
    pub text: VecDeque<String>,
    pub series_2d: HashMap<String, (Color, VecDeque<PlotPoint>)>,
    pub plot_image: RetainedImage,
}
impl PlotSource {
    pub fn make(name: String) -> Self {
        let default_image = &DEFAULT_IMAGE;
        Self {
            color: color(&name),
            name: name,
            max_memory_footprint: 1073741824, // 1 Gibibyte
            memory_footprint: 0,
            text: VecDeque::new(),
            series_2d: HashMap::new(),
            plot_image: RetainedImage::from_color_image("plotmux image",
                egui::ColorImage::from_rgba_unmultiplied(
                    [default_image.width() as _, default_image.height() as _],
                    default_image.as_raw(),
            )),
        }
    }
    pub fn new_data(&mut self, d: PlotableData) {
        match d {
            PlotableData::String(s) => {
                self.memory_footprint += size_of::<String>();
                self.memory_footprint += s.s.len();
                self.text.push_back(s.s);
                while self.memory_footprint > self.max_memory_footprint {
                    let s = self.text.pop_front().unwrap();
                    self.memory_footprint -= size_of::<String>();
                    self.memory_footprint -= s.len();
                }
            },
            PlotableData::Series2d(series_2d) => match self.series_2d.get_mut(&series_2d.series) {
                Some(points) => {
                    self.memory_footprint += size_of::<PlotPoint>();
                    points.1.push_back(PlotPoint::new(series_2d.x, series_2d.y));
                    if self.memory_footprint > self.max_memory_footprint {
                        points.1.pop_front();
                        self.memory_footprint -= size_of::<PlotPoint>();
                    }
                }
                None => {
                    let c = color(&series_2d.series);
                    self.series_2d.insert(
                        series_2d.series,
                        (c, VecDeque::from([PlotPoint::new(series_2d.x, series_2d.y)])),
                    );
                }
            },
            PlotableData::Image(pimage) => {
                let pimage : image::RgbaImage = image::RgbImage::from_raw(pimage.dim.0, pimage.dim.1, pimage.raw).unwrap().convert();
                self.plot_image = RetainedImage::from_color_image("plotmux image",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [pimage.width() as _, pimage.height() as _],
                        pimage.as_raw(),
                ));
            },
        }
    }
}
