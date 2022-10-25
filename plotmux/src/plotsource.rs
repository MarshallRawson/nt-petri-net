use crate::plotmux::{color, Color, PlotableData};
use eframe::egui;
use eframe::egui::widgets::plot::PlotPoint;
use egui_extras::image::RetainedImage;

use image::buffer::ConvertBuffer;
use image::Rgba;
use image::RgbaImage;
use lazy_static::lazy_static;
use std::collections::VecDeque;

lazy_static! {
    static ref DEFAULT_IMAGE: RgbaImage = RgbaImage::from_fn(1920, 1080, |y, x| {
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
    pub texts: VecDeque<(Option<(Color, String)>, String)>,
    pub series_plots_2d: Vec<(String, Vec<(String, (Color, VecDeque<PlotPoint>))>)>,
    pub image_plots: Vec<(String, RetainedImage)>,
}
impl PlotSource {
    pub fn make(name: String) -> Self {
        Self {
            name: name,
            texts: VecDeque::new(),
            series_plots_2d: Vec::new(),
            image_plots: Vec::new(),
        }
    }
    pub fn new_data(&mut self, d: PlotableData) {
        match d {
            PlotableData::InitSource(s) => {
                *self = Self::make(s);
            },
            PlotableData::String(s) => {
                let color_channel = match s.channel {
                    None => None,
                    Some(channel) => Some((color(&channel), channel)),
                };
                self.texts.push_back((color_channel, s.s));
            }
            PlotableData::InitSeriesPlot2d(new_plot) => {
                self.series_plots_2d.push((new_plot, vec![]));
            }
            PlotableData::InitSeries2d(new_series) => {
                let c = color(&new_series.series);
                self.series_plots_2d[new_series.channel].1.push((new_series.series, (c, VecDeque::new())));
            }
            PlotableData::Series2d(series_2d) => {
                self.series_plots_2d[series_2d.channel].1[series_2d.series].1.1
                    .push_back(PlotPoint::new(series_2d.x, series_2d.y));
            },
            PlotableData::InitImagePlot(channel) => {
                let rimage = RetainedImage::from_color_image(
                    &channel,
                    egui::ColorImage::from_rgba_unmultiplied(
                        [DEFAULT_IMAGE.width() as _, DEFAULT_IMAGE.height() as _],
                        DEFAULT_IMAGE.as_raw(),
                    ),
                );
                self.image_plots.push((channel, rimage));
            }
            PlotableData::Image(pimage) => {
                let image: image::RgbaImage =
                    image::RgbImage::from_raw(pimage.dim.0, pimage.dim.1, pimage.raw)
                        .unwrap()
                        .convert();
                let rimage = RetainedImage::from_color_image(
                    &self.image_plots[pimage.channel].0,
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as _, image.height() as _],
                        image.as_raw(),
                    ),
                );
                self.image_plots[pimage.channel].1 = rimage;
            }
        }
    }
}
