use crate::plotmux::{color, Color, PlotableData};
use eframe::egui;
use eframe::egui::widgets::plot::PlotPoint;
use egui_extras::image::RetainedImage;

use image::buffer::ConvertBuffer;
use image::io::Reader as ImageReader;
use std::collections::HashMap;
use std::path::Path;

pub struct PlotSource {
    pub name: String,
    pub color: Color,
    pub text: Vec<String>,
    pub series_2d: HashMap<String, (Color, Vec<PlotPoint>)>,
    pub plot_image: RetainedImage,
}
impl PlotSource {
    pub fn make(name: String) -> Self {
        let default_image = ImageReader::open(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/", "battle mech playing acoustic guitar.jpg"))).unwrap();
        let default_image = default_image.decode().unwrap();
        let default_image = default_image.thumbnail(1280, 720).into_rgba8();
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
                use std::time::Instant;
                let pimage : image::RgbaImage = image::RgbaImage::from_raw(pimage.dim.0, pimage.dim.1, pimage.raw).unwrap();
                self.plot_image = RetainedImage::from_color_image("plotmux image",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [pimage.width() as _, pimage.height() as _],
                        pimage.as_raw(),
                ));
            },
        }
    }
}
