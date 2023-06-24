use crate::plotmux::{color, Color, PlotableData, RgbDeltaImage};
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
    pub image_plots: Vec<(String, image::RgbaImage, RetainedImage)>,
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
            PlotableData::InitTcp(_) => unimplemented!(),
            PlotableData::InitSource(s) => {
                *self = Self::make(s);
            }
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
                self.series_plots_2d[new_series.channel]
                    .1
                    .push((new_series.series, (c, VecDeque::new())));
            }
            PlotableData::Series2d(series_2d) => {
                self.series_plots_2d[series_2d.channel].1[series_2d.series]
                    .1
                     .1
                    .push_back(PlotPoint::new(series_2d.x, series_2d.y));
            }
            PlotableData::Series2dVec(series_2d) => {
                for (x, y) in series_2d.data {
                    self.series_plots_2d[series_2d.channel].1[series_2d.series]
                        .1
                         .1
                        .push_back(PlotPoint::new(x, y));
                }
            }
            PlotableData::Line2d(series_2d) => {
                self.series_plots_2d[series_2d.channel].1[series_2d.series]
                    .1
                     .1 = series_2d
                    .data
                    .into_iter()
                    .map(|(x, y)| PlotPoint::new(x, y))
                    .collect();
            }
            PlotableData::InitImage(pimage) => {
                let image: image::RgbaImage =
                    image::RgbImage::from_raw(pimage.dim.0, pimage.dim.1, pimage.raw)
                        .unwrap()
                        .convert();
                let rimage = RetainedImage::from_color_image(
                    &pimage.channel,
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as _, image.height() as _],
                        image.as_raw(),
                    ),
                );
                if let Some(position) = self
                    .image_plots
                    .iter()
                    .position(|(c, _, _)| pimage.channel == *c)
                {
                    self.image_plots[position] = (pimage.channel, image, rimage);
                } else {
                    self.image_plots.push((pimage.channel, image, rimage));
                }
            }
            PlotableData::DeltaImage(dimage) => {
                let dims = self.image_plots[dimage.channel].1.dimensions();
                let image = RgbDeltaImage::from_vec(dims.0, dims.1, dimage.raw).unwrap();
                for (a, b) in std::iter::zip(
                    self.image_plots
                        .get_mut(dimage.channel)
                        .unwrap()
                        .1
                        .pixels_mut(),
                    image.pixels(),
                ) {
                    *a = image::Rgba::from([
                        (b[0] + a[0] as i16) as u8,
                        (b[1] + a[1] as i16) as u8,
                        (b[2] + a[2] as i16) as u8,
                        a[3],
                    ]);
                }
                let image = &self.image_plots[dimage.channel].1;
                self.image_plots[dimage.channel].2 = RetainedImage::from_color_image(
                    &self.image_plots[dimage.channel].0,
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as _, image.height() as _],
                        image.as_raw(),
                    ),
                );
            }
        }
    }
}
