use image::RgbImage;
use nokhwa::{Camera, CameraFormat, FrameFormat};
use ntpnet_lib::TransitionMaker;
use plotmux::plotsink::PlotSink;
use std::time::Instant;
#[derive(ntpnet_macro::TransitionInputTokens)]
struct E {
    _enable: (),
}
#[derive(ntpnet_macro::TransitionOutputTokens)]
struct Image {
    image: RgbImage,
}
#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(read: Input(E) -> Output(Image))]
pub struct CameraReader {
    camera: Camera,
    start_time: Instant,
    last_time: Option<Instant>,
    p: PlotSink,
}
impl CameraReader {
    pub fn maker(width: u32, height: u32, plotsink: PlotSink) -> TransitionMaker {
        Box::new(move || {
            let mut cam = Camera::new(
                0,
                Some(CameraFormat::new_from(
                    width,
                    height,
                    FrameFormat::MJPEG,
                    30,
                )),
            )
            .unwrap();
            cam.open_stream().unwrap();
            Box::new(Self {
                camera: cam,
                start_time: Instant::now(),
                last_time: None,
                p: plotsink,
            })
        })
    }
    fn read(&mut self, _: Input) -> Output {
        let resolution = self.camera.resolution();
        let frame = self.camera.frame().unwrap();
        let rgb_frame =
            RgbImage::from_raw(resolution.width(), resolution.height(), frame.to_vec()).unwrap();
        let now = Instant::now();
        if let Some(last_time) = self.last_time {
            self.p.plot_series_2d(
                "frame rate".into(),
                (now - self.start_time).as_secs_f64(),
                1. / (now - last_time).as_secs_f64(),
            );
        }
        self.last_time = Some(now);
        Output::Image(Image { image: rgb_frame })
    }
}
