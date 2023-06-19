use image::{
    imageops::{resize, FilterType},
    GrayImage, RgbImage,
};
use std::collections::VecDeque;
use std::time::Instant;

use image::DynamicImage;
use ntpnet;
use plotmux::plotsink::{ImageCompression, PlotSink};

use crate::facial_detection::FaceBBox;

#[derive(ntpnet::TransitionInputTokensMacro)]
struct Faces {
    faces: (Instant, RgbImage, Vec<FaceBBox>),
    _enable: (),
}

#[derive(ntpnet::TransitionOutputTokensMacro)]
struct Moving {
    moving: (Instant, Vec<f64>),
    done: (),
}

#[derive(ntpnet::Transition)]
#[ntpnet_transition(f: Input(Faces) -> Output(Moving))]
pub struct MouthMoving {
    p: PlotSink,
    t0: Instant,
    hist: VecDeque<f64>,
}
impl MouthMoving {
    pub fn maker(plotsink: PlotSink) -> ntpnet::TransitionMaker {
        Box::new(|| {
            Box::new(Self {
                p: plotsink,
                t0: std::time::Instant::now(),
                hist: VecDeque::new(),
            })
        })
    }
    fn f(&mut self, i: Input) -> Output {
        let (t, faces, bboxes) = match i {
            Input::Faces(Faces { faces, .. }) => faces,
        };
        let mut moving = vec![];
        for bbox in &bboxes {
            let mouth_center = (
                (bbox.landmarks.right_mouth.0 + bbox.landmarks.left_mouth.0) / 2.,
                (bbox.landmarks.nose.1 + bbox.y2) / 2.,
            );
            let mut mouth = RgbImage::new(
                ((bbox.landmarks.right_mouth.0 - bbox.landmarks.left_mouth.0) / 2.) as u32,
                ((bbox.y2 - bbox.landmarks.nose.1) / 4.) as u32,
            );
            for i in 0..mouth.width() {
                for j in 0..mouth.height() {
                    let i2 =
                        (i + mouth_center.0 as u32 - mouth.width() / 2).clamp(0, faces.width() - 1);
                    let j2 = (j + mouth_center.1 as u32 - mouth.height() / 2)
                        .clamp(0, faces.height() - 1);
                    mouth.put_pixel(i, j, *faces.get_pixel(i2, j2));
                }
            }
            let mouth: GrayImage = DynamicImage::ImageRgb8(mouth).into_luma8();
            let mut bright = 0.;
            for p in mouth.pixels() {
                bright += p[0] as f64;
            }
            bright /= mouth.len() as f64;
            self.hist.push_back(bright);
            if self.hist.len() >= 30 {
                self.hist.pop_front();
                let avg: f64 = self.hist.iter().sum::<f64>() / self.hist.len() as f64;
                bright = f64::exp(f64::abs((bright / avg) - 1.)) - 1.;
                self.p
                    .plot_series_2d("", "brightness", (t - self.t0).as_secs_f64(), bright);
                let w = mouth.width();
                let h = mouth.height();
                self.p.plot_image(
                    "",
                    resize(
                        &DynamicImage::ImageLuma8(mouth).into_rgb8(),
                        w * 15,
                        h * 15,
                        FilterType::Triangle,
                    ),
                    ImageCompression::Lossless,
                );
                moving.push(bright);
            }
        }
        Output::Moving(Moving {
            moving: (t, moving),
            done: (),
        })
    }
}
