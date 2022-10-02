use crate::plotmux::{color, Color, PlotableData};
use eframe::egui;
use eframe::egui::widgets::plot::PlotPoint;
use egui_extras::image::RetainedImage;

use image::buffer::ConvertBuffer;
use image::Rgba;
use image::RgbaImage;
use std::collections::HashMap;
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
    pub color: Color,
    pub text: Vec<String>,
    pub series_2d: HashMap<String, (Color, Vec<PlotPoint>)>,
    pub plot_image: RetainedImage,
}
impl PlotSource {
    pub fn make(name: String) -> Self {
        let default_image = &DEFAULT_IMAGE;
        Self {
            color: color(&name),
            name: name,
            text: vec![],
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
            PlotableData::String(s) => self.text.push(s.s),
            PlotableData::Series2d(series_2d) => match self.series_2d.get_mut(&series_2d.series) {
                Some(points) => {
                    points.1.push(PlotPoint::new(series_2d.x, series_2d.y));
                }
                None => {
                    let c = color(&series_2d.series);
                    self.series_2d.insert(
                        series_2d.series,
                        (c, vec![PlotPoint::new(series_2d.x, series_2d.y)]),
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
